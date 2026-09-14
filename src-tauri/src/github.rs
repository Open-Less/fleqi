//! GitHub 账号登录（OAuth 设备码流程）。
//!
//! 所有请求只发往固定的 GitHub 官方主机；每次请求前校验协议与主机名，拒绝
//! 环回、私有和保留地址。访问令牌只保存在系统凭据存储中，不进入日志或诊断。
use serde_json::{Value, json};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

/// 由 GitHub OAuth 应用提供；留空时界面会提示先按文档创建应用。
const CLIENT_ID: &str = match option_env!("FLEQI_GITHUB_CLIENT_ID") {
    Some(value) => value,
    None => "",
};
const SCOPE: &str = "read:user";
const ACCOUNT: &str = "github";
const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const USER_URL: &str = "https://api.github.com/user";
const VERIFY_URL: &str = "https://github.com/login/device";
static FLOW: Mutex<Option<Flow>> = Mutex::new(None);

struct Flow {
    device_code: String,
    interval: u64,
    expires_at: Instant,
}

pub fn configured() -> bool {
    !CLIENT_ID.is_empty()
}

/// 只允许 https 且主机名完全等于白名单条目的地址，其他一律拒绝。
fn allowed(endpoint: &str, hosts: &[&str]) -> Result<reqwest::Url, String> {
    let url = reqwest::Url::parse(endpoint).map_err(|_| "GitHub 地址无效".to_string())?;
    if url.scheme() != "https" {
        return Err("只允许 https 地址".into());
    }
    match url.host_str() {
        Some(host) if hosts.contains(&host) => Ok(url),
        _ => Err("GitHub 地址不在允许的主机列表内".into()),
    }
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("Fleqi")
        .build()
        .map_err(|error| format!("网络客户端不可用：{error}"))
}

fn stored() -> Option<Value> {
    crate::platform::credential("read", ACCOUNT, None).ok().filter(|v| !v.is_null())
}

pub fn status() -> Value {
    let account = stored()
        .filter(|value| value.get("token").and_then(Value::as_str).is_some())
        .map(|value| {
            json!({
                "loggedIn": true,
                "login": value.get("login").cloned().unwrap_or(Value::Null),
                "name": value.get("name").cloned().unwrap_or(Value::Null),
                "avatarUrl": value.get("avatarUrl").cloned().unwrap_or(Value::Null),
                "htmlUrl": value.get("htmlUrl").cloned().unwrap_or(Value::Null),
            })
        });
    json!({
        "configured": configured(),
        "account": account.unwrap_or(Value::Null),
    })
}

pub fn logout() -> Result<(), String> {
    if let Ok(mut flow) = FLOW.lock() {
        *flow = None;
    }
    crate::platform::credential("delete", ACCOUNT, None).map_err(|e| e.to_string())?;
    Ok(())
}

/// 第一步：向 GitHub 申请设备码，并把用户送去输入设备码的页面。
pub async fn login_start(app: &tauri::AppHandle) -> Result<Value, String> {
    use tauri_plugin_opener::OpenerExt;
    if !configured() {
        return Err("尚未配置 GitHub OAuth 应用，请先按文档创建并填写客户端 ID。".into());
    }
    let url = allowed(DEVICE_CODE_URL, &["github.com"])?;
    let response = client()?
        .post(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .form(&[("client_id", CLIENT_ID), ("scope", SCOPE)])
        .send()
        .await
        .map_err(|error| format!("无法连接 GitHub：{error}"))?;
    if !response.status().is_success() {
        return Err(format!("GitHub 拒绝了设备码请求（HTTP {}）", response.status()));
    }
    let body: Value = response.json().await.map_err(|e| e.to_string())?;
    let device_code = body["device_code"].as_str().ok_or("GitHub 未返回设备码")?.to_string();
    let user_code = body["user_code"].as_str().unwrap_or("").to_string();
    let verification = body["verification_uri"].as_str().unwrap_or(VERIFY_URL).to_string();
    let interval = body["interval"].as_u64().unwrap_or(5).clamp(1, 60);
    let expires = body["expires_in"].as_u64().unwrap_or(900).clamp(60, 3600);
    if let Ok(mut flow) = FLOW.lock() {
        *flow = Some(Flow {
            device_code,
            interval,
            expires_at: Instant::now() + Duration::from_secs(expires),
        });
    }
    if let Ok(url) = allowed(&verification, &["github.com"]) {
        let _ = app.opener().open_url(url.to_string(), None::<&str>);
    }
    Ok(json!({
        "userCode": user_code,
        "verificationUri": verification,
        "interval": interval,
        "expiresIn": expires,
    }))
}

/// 第二步：轮询授权结果。待授权时返回 pending，授权成功后写入系统凭据存储。
pub async fn login_poll() -> Result<Value, String> {
    let flow = FLOW
        .lock()
        .map_err(|_| "登录状态不可用")?
        .as_ref()
        .map(|flow| (flow.device_code.clone(), flow.interval, flow.expires_at));
    let Some((device_code, interval, expires_at)) = flow else {
        return Err("请先开始 GitHub 登录".into());
    };
    if Instant::now() >= expires_at {
        if let Ok(mut flow) = FLOW.lock() {
            *flow = None;
        }
        return Err("设备码已过期，请重新发起登录".into());
    }
    let url = allowed(TOKEN_URL, &["github.com"])?;
    let response = client()?
        .post(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .form(&[
            ("client_id", CLIENT_ID),
            ("device_code", device_code.as_str()),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .send()
        .await
        .map_err(|error| format!("无法连接 GitHub：{error}"))?;
    let body: Value = response.json().await.map_err(|e| e.to_string())?;
    if let Some(token) = body["access_token"].as_str() {
        let account = fetch_account(token).await?;
        crate::platform::credential(
            "write",
            ACCOUNT,
            Some(json!({
                "token": token,
                "login": account["login"],
                "name": account["name"],
                "avatarUrl": account["avatarUrl"],
                "htmlUrl": account["htmlUrl"],
            })),
        )
        .map_err(|e| e.to_string())?;
        if let Ok(mut flow) = FLOW.lock() {
            *flow = None;
        }
        return Ok(json!({"status": "authorized", "login": account["login"]}));
    }
    match body["error"].as_str().unwrap_or("") {
        "authorization_pending" => Ok(json!({"status": "pending", "interval": interval})),
        "slow_down" => Ok(json!({"status": "pending", "interval": (interval + 5).min(60)})),
        "access_denied" => {
            if let Ok(mut flow) = FLOW.lock() {
                *flow = None;
            }
            Err("已在 GitHub 取消授权".into())
        }
        "expired_token" => {
            if let Ok(mut flow) = FLOW.lock() {
                *flow = None;
            }
            Err("设备码已过期，请重新发起登录".into())
        }
        _ => Err(format!(
            "GitHub 登录失败：{}",
            body["error_description"].as_str().unwrap_or("未知错误")
        )),
    }
}

async fn fetch_account(token: &str) -> Result<Value, String> {
    let url = allowed(USER_URL, &["api.github.com"])?;
    let response = client()?
        .get(url)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .bearer_auth(token)
        .send()
        .await
        .map_err(|error| format!("无法读取 GitHub 账号：{error}"))?;
    if !response.status().is_success() {
        return Err(format!("GitHub 账号读取失败（HTTP {}）", response.status()));
    }
    let body: Value = response.json().await.map_err(|e| e.to_string())?;
    Ok(json!({
        "login": body["login"].as_str().unwrap_or(""),
        "name": body["name"].as_str().unwrap_or(""),
        "avatarUrl": body["avatar_url"].as_str().unwrap_or(""),
        "htmlUrl": body["html_url"].as_str().unwrap_or(""),
    }))
}
