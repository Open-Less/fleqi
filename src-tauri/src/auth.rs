//! 账号登录（Codex OAuth）的授权链接与提示文案处理。
//!
//! 链接来自内置运行时进程，属于不可信输入：只有主机完全等于官方 Codex
//! 域名、且为 https 的地址才会交给系统浏览器打开，其它地址一律忽略。
use serde_json::{Value, json};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

const ALLOWED_HOSTS: [&str; 2] = ["auth.openai.com", "chatgpt.com"];

fn host_allowed(value: &str) -> bool {
    if value.chars().any(char::is_control) || value.len() > 4096 {
        return false;
    }
    let Some(rest) = value.strip_prefix("https://") else {
        return false;
    };
    let Some((host, _)) = rest.split_once('/') else {
        return false;
    };
    ALLOWED_HOSTS.contains(&host)
}

/// 打开授权页面。返回是否真的请求了系统浏览器。
pub fn open_authorized(app: &AppHandle, value: &str) -> bool {
    host_allowed(value) && app.opener().open_url(value, None::<&str>).is_ok()
}

/// 运行时下发的进度同样是英文；只翻译已知几条，其余原样保留。
pub fn progress_copy(message: &str) -> String {
    match message {
        "Exchanging authorization code for tokens..." => "正在用授权码换取登录令牌…".into(),
        "Enabling models..." => "正在启用账号模型…".into(),
        other => other.to_string(),
    }
}

/// 运行时下发的是英文提示；这里换成中文，选项 id 保持原样供运行时解析。
pub fn prompt_copy(prompt: &Value) -> (String, Value) {
    match prompt["type"].as_str().unwrap_or("") {
        "select" => {
            let options = prompt["options"]
                .as_array()
                .map(|items| {
                    Value::Array(
                        items
                            .iter()
                            .map(|item| {
                                let id = item["id"].as_str().unwrap_or("").to_string();
                                let label = match id.as_str() {
                                    "browser" => "浏览器登录（推荐）".to_string(),
                                    "device_code" => "设备码登录（无浏览器）".to_string(),
                                    _ => item["label"]
                                        .as_str()
                                        .unwrap_or(id.as_str())
                                        .to_string(),
                                };
                                json!({"id":id,"label":label})
                            })
                            .collect(),
                    )
                })
                .unwrap_or(Value::Null);
            ("选择 Codex 账号登录方式".into(), options)
        }
        "manual_code" => (
            "浏览器里完成登录后，把地址栏中的完整回调链接（或授权码）粘贴到这里。".into(),
            Value::Null,
        ),
        _ => (
            prompt["message"]
                .as_str()
                .unwrap_or("完成浏览器授权，或粘贴授权返回码")
                .chars()
                .take(500)
                .collect(),
            prompt["options"].clone(),
        ),
    }
}
