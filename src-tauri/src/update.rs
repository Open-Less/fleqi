//! 自动更新：从 GitHub Releases 读取更新并在应用内完成下载与重启。
//!
//! 仓库与签名公钥在打包前填入；未配置时界面显示“更新通道尚未配置”，
//! 不会发起任何网络请求。下载进度通过 `fleqi:update` 事件推送给界面。
//!
//! 通道由当前版本号决定，与 release.yml 的产物一一对应：
//!   - 正式版（0.0.1）从 releases/latest/download/latest.json 取更新；
//!   - 测试版（0.0.1-beta.1）从常驻的 beta 通道 Release 取 latest-beta.json，
//!     因为 GitHub 的 releases/latest 只指向非 prerelease。
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

/// 形如 "owner/repo"；留空表示尚未配置更新仓库。
const REPOSITORY: &str = "Open-Less/fleqi";
/// 由 `tauri signer generate` 生成的公钥；留空表示尚未配置签名校验。
/// 私钥只存在于本机与仓库 Secret（TAURI_SIGNING_PRIVATE_KEY），绝不可入库。
const PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDY5QjA4MDU5Njc4QzQyQTAKUldTZ1FveG5XWUN3YVZQU0RrVWlKMTdTa1MzYm9kbDFJNFpzWjkzazlpMDRNNFc1UnZMUWxkeWMK";

pub fn configured() -> bool {
    !REPOSITORY.is_empty() && !PUBKEY.is_empty()
}

pub fn repository() -> &'static str {
    REPOSITORY
}

/// 当前运行的版本，由 Cargo 在编译期写入，与 tauri.conf.json 的版本一致。
pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn channel() -> &'static str {
    if current_version().contains("-beta") {
        "beta"
    } else {
        "stable"
    }
}

fn endpoint() -> String {
    match channel() {
        "beta" => {
            format!("https://github.com/{REPOSITORY}/releases/download/beta/latest-beta.json")
        }
        _ => format!("https://github.com/{REPOSITORY}/releases/latest/download/latest.json"),
    }
}

fn updater(app: &AppHandle) -> Result<tauri_plugin_updater::Updater, String> {
    if !configured() {
        return Err("更新通道尚未配置，请先在应用设置中填入仓库与签名公钥。".into());
    }
    let url = reqwest::Url::parse(&endpoint()).map_err(|_| "更新地址无效".to_string())?;
    if url.scheme() != "https" || url.host_str() != Some("github.com") {
        return Err("更新地址不在允许的主机列表内".into());
    }
    app.updater_builder()
        .endpoints(vec![url])
        .map_err(|e| e.to_string())?
        .pubkey(PUBKEY)
        .build()
        .map_err(|e| e.to_string())
}

pub fn info() -> Value {
    let repository = repository();
    json!({
        "configured": configured(),
        "repository": if repository.is_empty() { Value::Null } else { json!(repository) },
        "channel": channel(),
        "currentVersion": current_version(),
    })
}

pub async fn check(app: &AppHandle) -> Result<Value, String> {
    let update = updater(app)?
        .check()
        .await
        .map_err(|error| format!("检查更新失败：{error}"))?;
    Ok(match update {
        Some(update) => json!({
            "available": true,
            "version": update.version,
            "currentVersion": update.current_version,
            "channel": channel(),
            "notes": update.body,
            "date": update.date.map(|date| date.to_string()),
        }),
        None => json!({
            "available": false,
            "channel": channel(),
            "currentVersion": app.package_info().version.to_string(),
        }),
    })
}

pub async fn install(app: &AppHandle) -> Result<Value, String> {
    let update = updater(app)?
        .check()
        .await
        .map_err(|error| format!("检查更新失败：{error}"))?
        .ok_or("当前已是最新版本")?;
    let handle = app.clone();
    let mut downloaded: u64 = 0;
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let _ = handle.emit(
                    "fleqi:update",
                    json!({"phase":"download","downloaded":downloaded,"total":total}),
                );
            },
            move || {
                let _ = app.emit("fleqi:update", json!({"phase":"installing"}));
            },
        )
        .await
        .map_err(|error| format!("更新安装失败：{error}"))?;
    let _ = app.emit("fleqi:update", json!({"phase":"restarting"}));
    // 安装完成后重启，直接运行新版本。原有凭据与设置都保存在系统存储与
    // 应用数据目录中，重启不会丢失。
    app.restart();
}
