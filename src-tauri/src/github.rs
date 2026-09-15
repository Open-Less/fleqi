//! GitHub App 浏览器授权。设备码仅由宿主持有，网络轮询不依赖设置窗口的生命周期。
//! 用户在系统授权弹窗登录 GitHub；个人访问令牌不再作为登录入口。
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

const CLIENT_ID: &str = match option_env!("FLEQI_GITHUB_CLIENT_ID") {
    Some(value) => value,
    None => "Iv23liy1Kop6o7aIC6Ns",
};
const ACCOUNT: &str = "github";
const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const USER_URL: &str = "https://api.github.com/user";
const VERIFY_URL: &str = "https://github.com/login/device";
static FLOW: Mutex<Option<Flow>> = Mutex::new(None);

#[derive(Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Starting,
    Waiting,
    Authorized,
    Cancelled,
    Expired,
    Failed,
}

struct Flow {
    id: String,
    phase: Phase,
    device_code: String,
    user_code: String,
    interval: u64,
    expires_at: Instant,
    expires_at_ms: u64,
    message: Option<String>,
}
impl Flow {
    fn new(id: String) -> Self {
        Self {
            id,
            phase: Phase::Starting,
            device_code: String::new(),
            user_code: String::new(),
            interval: 5,
            expires_at: Instant::now() + Duration::from_secs(900),
            expires_at_ms: fleqi_core::now_ms() + 900_000,
            message: None,
        }
    }
    fn active(&self, id: &str) -> bool {
        self.id == id
            && matches!(self.phase, Phase::Starting | Phase::Waiting)
            && Instant::now() < self.expires_at
    }
    fn public(&self) -> Value {
        json!({"id":self.id,"status":self.phase,"userCode":self.user_code,"expiresAt":self.expires_at_ms,"message":self.message})
    }
    fn finish(&mut self, phase: Phase, message: Option<String>) {
        self.phase = phase;
        self.message = message;
        self.device_code.clear();
        self.user_code.clear();
    }
}

pub fn configured() -> bool {
    !CLIENT_ID.trim().is_empty()
}

fn allowed(endpoint: &str, hosts: &[&str]) -> Result<reqwest::Url, String> {
    let url = reqwest::Url::parse(endpoint).map_err(|_| "GitHub 地址无效")?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.port_or_known_default() != Some(443)
        || !url.host_str().is_some_and(|host| hosts.contains(&host))
    {
        return Err("GitHub 地址不在允许的范围内".into());
    }
    Ok(url)
}
fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("Fleqi")
        .build()
        .map_err(|_| "网络客户端不可用".into())
}
fn valid_secret(value: &str) -> bool {
    (20..=512).contains(&value.len()) && value.bytes().all(|c| c.is_ascii_graphic())
}
fn read_account() -> Option<Value> {
    crate::platform::credential("read", ACCOUNT, None).ok()
        .filter(|v| v["token"].as_str().is_some_and(valid_secret))
        .filter(|v| v["expiresAt"].as_u64().is_none_or(|time| time > fleqi_core::now_ms()))
        .map(|v| json!({"loggedIn":true,"login":v["login"],"name":v["name"],"avatarUrl":v["avatarUrl"],"htmlUrl":v["htmlUrl"]}))
}
pub fn status() -> Value {
    json!({"configured":configured(),"mode":"browser","account":read_account(),
        "flow":FLOW.lock().ok().and_then(|v| v.as_ref().map(Flow::public))})
}
pub fn login_poll(id: &str) -> Result<Value, String> {
    FLOW.lock()
        .map_err(|_| "登录状态不可用")?
        .as_ref()
        .filter(|flow| flow.id == id)
        .map(Flow::public)
        .ok_or_else(|| "登录请求已失效，请重试".into())
}
fn close_browser(id: &str) {
    #[cfg(target_os = "macos")]
    let _ = crate::platform::host_call(json!({"operation":"github_auth_close","flowId":id}));
    #[cfg(not(target_os = "macos"))]
    let _ = id;
}
fn open_browser(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let _ = app;
        crate::platform::host_call(json!({"operation":"github_auth_open","flowId":id}))
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        use tauri_plugin_opener::OpenerExt;
        let _ = id;
        app.opener()
            .open_url(VERIFY_URL, None::<&str>)
            .map_err(|_| "无法打开 GitHub 登录页面".into())
    }
}
fn browser_closed(id: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        crate::platform::host_call(json!({"operation":"github_auth_status","flowId":id}))
            .ok()
            .is_some_and(|v| v["open"] == false)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = id;
        false
    }
}
fn finish(app: &tauri::AppHandle, id: &str, phase: Phase, message: Option<String>) {
    let changed = if let Ok(mut slot) = FLOW.lock() {
        if let Some(flow) = slot
            .as_mut()
            .filter(|f| f.id == id && matches!(f.phase, Phase::Starting | Phase::Waiting))
        {
            flow.finish(phase, message);
            true
        } else {
            false
        }
    } else {
        false
    };
    if changed {
        close_browser(id);
        crate::runtime::changed(app);
    }
}
pub fn login_cancel(app: &tauri::AppHandle, id: &str) {
    finish(app, id, Phase::Cancelled, None);
}
pub fn logout(app: &tauri::AppHandle) -> Result<(), String> {
    let id = {
        let mut slot = FLOW.lock().map_err(|_| "登录状态不可用")?;
        // 与授权落盘使用同一把锁，退出后旧请求不能重新写入凭据。
        crate::platform::credential("delete", ACCOUNT, None).map_err(|e| e.to_string())?;
        slot.take().map(|f| f.id)
    };
    if let Some(id) = id {
        close_browser(&id);
    }
    crate::runtime::changed(app);
    Ok(())
}

pub fn login_start(app: &tauri::AppHandle, id: String) -> Result<Value, String> {
    if !configured() {
        return Err("此版本尚未启用 GitHub 登录，请稍后重试".into());
    }
    uuid::Uuid::parse_str(&id).map_err(|_| "登录请求无效")?;
    let public = {
        let mut slot = FLOW.lock().map_err(|_| "登录状态不可用")?;
        if slot.as_ref().is_some_and(|f| f.active(&f.id)) {
            return Err("已有 GitHub 登录正在进行，请先完成或取消".into());
        }
        let flow = Flow::new(id.clone());
        let public = flow.public();
        *slot = Some(flow);
        public
    };
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = authorize(&app, &id).await {
            finish(&app, &id, Phase::Failed, Some(error));
        }
    });
    Ok(public)
}

fn device_response(body: &Value) -> Result<(String, String, u64, u64), String> {
    if body["error"] == "device_flow_disabled" {
        return Err("GitHub 登录暂不可用，请联系应用维护者".into());
    }
    let device = body["device_code"]
        .as_str()
        .filter(|v| valid_secret(v))
        .ok_or("GitHub 未返回有效授权信息")?;
    let user = body["user_code"]
        .as_str()
        .filter(|v| {
            v.len() == 9
                && v.as_bytes()[4] == b'-'
                && v.bytes()
                    .enumerate()
                    .all(|(i, c)| i == 4 || c.is_ascii_uppercase() || c.is_ascii_digit())
        })
        .ok_or("GitHub 未返回有效验证码")?;
    if body["verification_uri"].as_str() != Some(VERIFY_URL) {
        return Err("GitHub 授权地址无效".into());
    }
    let interval = body["interval"].as_u64().unwrap_or(5);
    let expires = body["expires_in"]
        .as_u64()
        .ok_or("GitHub 未返回授权有效期")?;
    if !(1..=3600).contains(&interval) || !(1..=3600).contains(&expires) {
        return Err("GitHub 授权有效期无效".into());
    }
    Ok((device.into(), user.into(), interval, expires))
}
async fn body(response: reqwest::Response) -> Result<Value, String> {
    if !response.status().is_success() {
        return Err(format!(
            "GitHub 请求失败（HTTP {}），请稍后重试",
            response.status()
        ));
    }
    response
        .json()
        .await
        .map_err(|_| "GitHub 响应无法读取".into())
}
async fn authorize(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    let client = client()?;
    let requested_at = Instant::now();
    let response = client
        .post(allowed(DEVICE_CODE_URL, &["github.com"])?)
        .header(reqwest::header::ACCEPT, "application/json")
        .form(&[("client_id", CLIENT_ID)])
        .send()
        .await
        .map_err(|_| "无法连接 GitHub，请检查网络后重试")?;
    let (device_code, user_code, interval, expires) = device_response(&body(response).await?)?;
    {
        let mut slot = FLOW.lock().map_err(|_| "登录状态不可用")?;
        let Some(flow) = slot.as_mut().filter(|f| f.active(id)) else {
            return Ok(());
        };
        flow.device_code = device_code;
        flow.user_code = user_code;
        flow.interval = interval;
        flow.expires_at = requested_at + Duration::from_secs(expires);
        flow.expires_at_ms = fleqi_core::now_ms()
            + flow
                .expires_at
                .saturating_duration_since(Instant::now())
                .as_millis() as u64;
        if Instant::now() >= flow.expires_at {
            return Err("GitHub 登录已过期，请重试".into());
        }
        open_browser(app, id)?;
        flow.phase = Phase::Waiting;
    }
    crate::runtime::changed(app);
    loop {
        let Some((interval, expires_at)) = FLOW
            .lock()
            .map_err(|_| "登录状态不可用")?
            .as_ref()
            .filter(|f| f.id == id && f.phase == Phase::Waiting)
            .map(|f| (f.interval, f.expires_at))
        else {
            return Ok(());
        };
        tokio::time::sleep(
            Duration::from_secs(interval).min(expires_at.saturating_duration_since(Instant::now())),
        )
        .await;
        if Instant::now() >= expires_at {
            finish(app, id, Phase::Expired, Some("登录已过期，请重试".into()));
            return Ok(());
        }
        let Some(code) = FLOW
            .lock()
            .map_err(|_| "登录状态不可用")?
            .as_ref()
            .filter(|f| f.active(id))
            .map(|f| f.device_code.clone())
        else {
            return Ok(());
        };
        if browser_closed(id) {
            finish(app, id, Phase::Cancelled, None);
            return Ok(());
        }
        let response = client
            .post(allowed(TOKEN_URL, &["github.com"])?)
            .header(reqwest::header::ACCEPT, "application/json")
            .form(&[
                ("client_id", CLIENT_ID),
                ("device_code", code.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await
            .map_err(|_| "无法连接 GitHub，请检查网络后重试")?;
        let value = body(response).await?;
        if let Some(token) = value["access_token"].as_str() {
            if !valid_secret(token) || value["token_type"].as_str() != Some("bearer") {
                return Err("GitHub 返回的登录信息无效".into());
            }
            let token_expiry = value["expires_in"]
                .as_u64()
                .filter(|seconds| *seconds > 0 && *seconds <= 31_536_000)
                .map(|seconds| fleqi_core::now_ms() + seconds * 1000);
            if value.get("expires_in").is_some() && token_expiry.is_none() {
                return Err("GitHub 返回的登录有效期无效".into());
            }
            let response = client
                .get(allowed(USER_URL, &["api.github.com"])?)
                .header(reqwest::header::ACCEPT, "application/vnd.github+json")
                .bearer_auth(token)
                .send()
                .await
                .map_err(|_| "无法读取 GitHub 账号，请重试")?;
            let account = body(response).await?;
            let login = account["login"]
                .as_str()
                .filter(|v| {
                    !v.is_empty()
                        && v.len() <= 39
                        && v.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
                })
                .ok_or("GitHub 账号信息无效")?;
            {
                let mut slot = FLOW.lock().map_err(|_| "登录状态不可用")?;
                let Some(flow) = slot.as_mut().filter(|f| f.active(id)) else {
                    drop(slot);
                    finish(app, id, Phase::Expired, Some("登录已过期，请重试".into()));
                    return Ok(());
                };
                crate::platform::credential("write",ACCOUNT,Some(json!({"token":token,"login":login,"name":account["name"],
                    "avatarUrl":account["avatar_url"],"htmlUrl":account["html_url"],"expiresAt":token_expiry,"source":"github-app"})))
                    .map_err(|e| e.to_string())?;
                flow.finish(Phase::Authorized, None);
            }
            close_browser(id);
            crate::runtime::changed(app);
            return Ok(());
        }
        match value["error"].as_str().unwrap_or("") {
            "authorization_pending" => {}
            "slow_down" => {
                let mut slot = FLOW.lock().map_err(|_| "登录状态不可用")?;
                if let Some(flow) = slot.as_mut().filter(|f| f.active(id)) {
                    flow.interval = value["interval"]
                        .as_u64()
                        .unwrap_or(0)
                        .max(flow.interval.saturating_add(5));
                }
            }
            "access_denied" => {
                finish(app, id, Phase::Cancelled, None);
                return Ok(());
            }
            "expired_token" => {
                finish(app, id, Phase::Expired, Some("登录已过期，请重试".into()));
                return Ok(());
            }
            _ => return Err("GitHub 未能完成授权，请重新登录".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_foreign_hosts_credentials_ports_and_fragments() {
        for url in [
            "http://github.com/login/device",
            "https://github.com.evil.test/login/device",
            "https://github.com@evil.test/",
            "https://user:secret@github.com/",
            "https://github.com:444/",
            "https://github.com/#token",
        ] {
            assert!(allowed(url, &["github.com"]).is_err());
        }
        assert!(allowed(VERIFY_URL, &["github.com"]).is_ok());
    }
    #[test]
    fn validates_device_response_without_extending_expiry() {
        let mut response = json!({"device_code":"fixture-device-code-01234567890123456789","user_code":"ABCD-1234","verification_uri":VERIFY_URL,"interval":5,"expires_in":1});
        assert_eq!(device_response(&response).unwrap().3, 1);
        response["verification_uri"] = json!("https://evil.test/");
        assert!(device_response(&response).is_err());
        response["verification_uri"] = json!(VERIFY_URL);
        response["expires_in"] = json!(0);
        assert!(device_response(&response).is_err());
    }
    #[test]
    fn cancelled_superseded_and_expired_sessions_cannot_accept_credentials() {
        let mut flow = Flow::new("session-a".into());
        assert!(flow.active("session-a"));
        assert!(!flow.active("session-b"));
        flow.device_code = "private-device-code".into();
        assert!(!flow.public().to_string().contains("private-device-code"));
        flow.finish(Phase::Cancelled, None);
        assert!(!flow.active("session-a"));
        assert!(flow.device_code.is_empty());
        let mut flow = Flow::new("session-a".into());
        flow.expires_at = Instant::now();
        assert!(!flow.active("session-a"));
    }
}
