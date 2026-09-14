use crate::{
    platform::{self, NativeHost},
    runtime,
    state::{ActiveTask, AppState},
};
use fleqi_core::{model::*, now_ms};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

type Reply<T> = Result<T, String>;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineInfo {
    pub id: String,
    pub name: String,
    pub ready: bool,
    pub version: String,
    pub formats: String,
}

fn version(path: &std::path::Path, arg: &str) -> String {
    std::process::Command::new(path)
        .arg(arg)
        .output()
        .ok()
        .filter(|r| r.status.success())
        .map(|r| {
            String::from_utf8_lossy(&r.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(100)
                .collect()
        })
        .unwrap_or_else(|| "不可用".into())
}
#[tauri::command]
pub async fn app_snapshot(state: State<'_, AppState>) -> Reply<Value> {
    let state = state.inner().clone();
    tokio::task::spawn_blocking(move||{
        let mut profiles=state.0.db.profiles().map_err(|e|e.to_string())?;
        for profile in &mut profiles{profile.has_credential=platform::credential("has",&profile.id,None).is_ok_and(|v|v==true);}
        let mut engines=vec![EngineInfo{id:"files".into(),name:"基础文件 / ZIP / 图片 / 文档".into(),ready:true,version:format!("Fleqi Core {}",env!("CARGO_PKG_VERSION")),formats:"文件操作、ZIP、PNG / JPG / WebP、TXT / Markdown / 简单 DOCX".into()}];
        for (id,name,path,formats) in [("ffmpeg","FFmpeg",&state.0.engines.ffmpeg,"MP3 / M4A / WAV / MP4"),("ffprobe","ffprobe",&state.0.engines.ffprobe,"音视频格式与时长核验"),("qpdf","qpdf",&state.0.engines.qpdf,"普通未加密 PDF")]{engines.push(EngineInfo{id:id.into(),name:name.into(),ready:path.as_ref().is_some_and(|p|p.is_file()),version:if path.is_some(){"随应用提供".into()}else{"尚未打包".into()},formats:formats.into()});}
        let node=state.0.resources.join("bin").join(if cfg!(windows){"node.exe"}else{"node"});
        platform::auto_probe_permissions();
        let context=state.0.context.lock().map_err(|_|"上下文不可用")?.clone();
        Ok(json!({"settings":state.0.db.settings().map_err(|e|e.to_string())?,"profiles":profiles,"permissions":NativeHost.permissions().map_err(|e|e.to_string())?,"platform":NativeHost.capabilities(),"runtime":{"ready":node.is_file()&&state.0.pi_dir.join("entry.mjs").is_file(),"node":"24.21.0","pi":"0.85.0","app":env!("CARGO_PKG_VERSION")},"engines":engines,"tasks":state.0.db.tasks(100).map_err(|e|e.to_string())?,"context":context,"interactions":state.interaction_list(),"auth":state.0.auth.lock().ok().and_then(|v|v.clone()),"github":crate::github::status(),"update":{"configured":crate::update::configured(),"repository":null},"activeTaskId":state.active_id(),"native":true}))
    }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub async fn import_appearance(
    app: AppHandle,
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    appearance: Appearance,
) -> Reply<()> {
    if window.label() != "main" {
        return Ok(());
    }
    let _guard = state.0.configuration_gate.lock().await;
    state
        .0
        .db
        .import_appearance(appearance)
        .map_err(|e| e.to_string())?;
    runtime::changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn settings_save(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: SettingsSnapshot,
) -> Reply<SettingsSnapshot> {
    let _configuration = state.0.configuration_gate.lock().await;
    settings.validate().map_err(|e| e.to_string())?;
    let previous = state.0.db.settings().map_err(|e| e.to_string())?;
    if previous.revision != settings.revision {
        return Err("设置已更新，请刷新后重试".into());
    }
    if previous.shortcut != settings.shortcut {
        crate::shell::replace_shortcut(&app, &previous.shortcut, &settings.shortcut)?;
    }
    if previous.launch_at_login != settings.launch_at_login {
        let result = if settings.launch_at_login {
            app.autolaunch().enable()
        } else {
            app.autolaunch().disable()
        };
        if let Err(error) = result {
            if previous.shortcut != settings.shortcut {
                let _ =
                    crate::shell::replace_shortcut(&app, &settings.shortcut, &previous.shortcut);
            }
            return Err(format!("登录启动设置失败：{error}"));
        }
    }
    let saved = match state.0.db.save_settings(settings.clone()) {
        Ok(saved) => saved,
        Err(error) => {
            if previous.shortcut != settings.shortcut {
                let _ =
                    crate::shell::replace_shortcut(&app, &settings.shortcut, &previous.shortcut);
            }
            if previous.launch_at_login != settings.launch_at_login {
                let _ = if previous.launch_at_login {
                    app.autolaunch().enable()
                } else {
                    app.autolaunch().disable()
                };
            }
            return Err(format!("设置保存失败：{error}"));
        }
    };
    if saved.activation != previous.activation || saved.bar_enabled != previous.bar_enabled {
        let _ = platform::host_call(
            json!({"operation":"automatic","enabled":saved.bar_enabled&&saved.activation=="automatic"}),
        );
    }
    if !saved.bar_enabled {
        let _ = platform::host_call(json!({"operation":"hide"}));
    }
    runtime::changed(&app);
    crate::shell::refresh_menu(&app);
    Ok(saved)
}
#[tauri::command]
pub async fn profile_save(
    app: AppHandle,
    state: State<'_, AppState>,
    mut profile: ModelProfile,
    api_key: Option<String>,
) -> Reply<()> {
    let _configuration = state.0.configuration_gate.lock().await;
    let _runtime_guard = if api_key.as_ref().is_some_and(|k| !k.trim().is_empty()) {
        Some(
            state
                .0
                .runtime_gate
                .clone()
                .try_lock_owned()
                .map_err(|_| "请先结束正在使用模型的操作，再更新凭据")?,
        )
    } else {
        None
    };
    profile.validate().map_err(|e| e.to_string())?;
    if let Ok(previous) = state.0.db.profile(&profile.id) {
        if previous.protocol == profile.protocol
            && previous.base_url == profile.base_url
            && previous.auth_type == profile.auth_type
        {
            profile.models = previous.models;
            profile.models_fetched_at = previous.models_fetched_at;
        } else {
            let _idle = if _runtime_guard.is_none() {
                Some(
                    state
                        .0
                        .runtime_gate
                        .clone()
                        .try_lock_owned()
                        .map_err(|_| "请先结束模型操作，再更换连接地址或认证方式")?,
                )
            } else {
                None
            };
            platform::credential("delete", &profile.id, None).map_err(|e| e.to_string())?;
            profile.models.clear();
            profile.models_fetched_at = None;
            profile.model_id.clear();
            profile.service_tier = None;
        }
    } else {
        profile.models.clear();
        profile.models_fetched_at = None;
    }
    if !profile.model_id.is_empty() {
        profile.validate_selection().map_err(|e| e.to_string())?;
    }
    if let Some(key) = api_key.filter(|v| !v.trim().is_empty()) {
        if key.len() > 20_000 || profile.auth_type != AuthType::ApiKey {
            return Err("API Key 格式或登录方式无效".into());
        }
        let id = profile.id.clone();
        tokio::task::spawn_blocking(move || {
            platform::credential(
                "write",
                &id,
                Some(json!({"type":"api_key","key":key.trim()})),
            )
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    }
    profile.has_credential = false;
    state
        .0
        .db
        .save_profile(&profile)
        .map_err(|e| e.to_string())?;
    let mut settings = state.0.db.settings().map_err(|e| e.to_string())?;
    if settings.default_profile_id.is_none() {
        settings.default_profile_id = Some(profile.id);
        state
            .0
            .db
            .save_settings(settings)
            .map_err(|e| e.to_string())?;
    }
    runtime::changed(&app);
    Ok(())
}
fn profile_idle(state: &AppState, id: &str) -> Reply<()> {
    if state
        .0
        .active
        .lock()
        .map_err(|_| "任务状态不可用")?
        .as_ref()
        .is_some_and(|a| a.profile_id == id)
    {
        return Err("此账号正在被任务使用，请先完成或取消任务".into());
    }
    if state.0.runtime_gate.try_lock().is_err() {
        return Err("模型运行时正在使用，请稍后重试".into());
    }
    Ok(())
}
#[tauri::command]
pub async fn profile_delete(app: AppHandle, state: State<'_, AppState>, id: String) -> Reply<()> {
    profile_idle(&state, &id)?;
    state.0.db.profile(&id).map_err(|e| e.to_string())?;
    platform::credential("delete", &id, None).map_err(|e| e.to_string())?;
    state.0.db.delete_profile(&id).map_err(|e| e.to_string())?;
    let mut settings = state.0.db.settings().map_err(|e| e.to_string())?;
    if settings.default_profile_id.as_deref() == Some(&id) {
        settings.default_profile_id = None;
        state
            .0
            .db
            .save_settings(settings)
            .map_err(|e| e.to_string())?;
    }
    runtime::changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn profile_logout(app: AppHandle, state: State<'_, AppState>, id: String) -> Reply<()> {
    profile_idle(&state, &id)?;
    platform::credential("delete", &id, None).map_err(|e| e.to_string())?;
    runtime::changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn profile_run(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    mode: String,
) -> Reply<Value> {
    if !["login", "test", "models"].contains(&mode.as_str()) {
        return Err("连接操作无效".into());
    }
    let state = state.inner().clone();
    // 每次连接操作都从干净的授权状态开始，避免显示上一次的授权链接。
    if let Ok(mut auth) = state.0.auth.lock() {
        *auth = None;
    }
    let _guard = state
        .0
        .runtime_gate
        .clone()
        .try_lock_owned()
        .map_err(|_| "已有任务或连接检查正在运行")?;
    let profile = state.0.db.profile(&id).map_err(|e| e.to_string())?;
    if mode == "test" {
        profile.validate_selection().map_err(|e| e.to_string())?;
    }
    let cancelled = Arc::new(AtomicBool::new(false));
    *state
        .0
        .runtime_cancel
        .lock()
        .map_err(|_| "运行时状态不可用")? = Some(cancelled.clone());
    let mut result = runtime::run(
        app.clone(),
        state.clone(),
        &mode,
        profile.clone(),
        None,
        cancelled.clone(),
    )
    .await;
    let persist = (|| -> Reply<()> {
        if let Ok(value) = result.as_mut() {
            if let Some(catalog) = value["result"].get("catalog") {
                let models: Vec<ModelInfo> =
                    serde_json::from_value(catalog["models"].clone()).map_err(|e| e.to_string())?;
                let mut current = state.0.db.profile(&id).map_err(|e| e.to_string())?;
                if current.base_url == profile.base_url
                    && current.protocol == profile.protocol
                    && current.auth_type == profile.auth_type
                {
                    current.models = models;
                    current.models_fetched_at = catalog["fetchedAt"].as_u64();
                    if let Some(url) = catalog["baseUrl"].as_str() {
                        current.base_url = url.into();
                    }
                    if !current.models.iter().any(|m| m.id == current.model_id) {
                        current.model_id = current
                            .models
                            .first()
                            .map(|m| m.id.clone())
                            .unwrap_or_default();
                    }
                    if let Some(model) = current.models.iter().find(|m| m.id == current.model_id) {
                        if !model.thinking_levels.contains(&current.thinking) {
                            current.thinking = model.default_thinking.clone();
                        }
                        if current
                            .service_tier
                            .as_ref()
                            .is_some_and(|tier| !model.service_tiers.iter().any(|t| &t.id == tier))
                        {
                            current.service_tier = None;
                        }
                    }
                    state
                        .0
                        .db
                        .save_profile(&current)
                        .map_err(|e| e.to_string())?;
                    value["profile"] = json!(current);
                }
            }
        }
        Ok(())
    })();
    if let Err(error) = persist {
        result = Err(error);
    }
    cancelled.store(true, Ordering::Release);
    if let Ok(mut cancel) = state.0.runtime_cancel.lock() {
        *cancel = None;
    }
    if let Ok(mut pending) = state.0.pending.lock() {
        pending.retain(|_, p| p.info.task_id.is_some());
    }
    runtime::changed(&app);
    result
}
#[tauri::command]
pub async fn auth_open(app: AppHandle, state: State<'_, AppState>, url: Option<String>) -> Reply<()> {
    let stored = state
        .0
        .auth
        .lock()
        .map_err(|_| "授权状态不可用")?
        .as_ref()
        .and_then(|value| value["url"].as_str().map(str::to_owned));
    let target = url.or(stored).ok_or("当前没有待完成的授权链接")?;
    if !crate::auth::open_authorized(&app, &target) {
        return Err("授权链接无效，请在浏览器中手动打开 Codex 登录页面。".into());
    }
    Ok(())
}
#[tauri::command]
pub async fn github_status() -> Reply<Value> {
    Ok(crate::github::status())
}
#[tauri::command]
pub async fn github_login_start(app: AppHandle) -> Reply<Value> {
    crate::github::login_start(&app).await
}
#[tauri::command]
pub async fn github_login_poll(app: AppHandle) -> Reply<Value> {
    let result = crate::github::login_poll().await?;
    runtime::changed(&app);
    Ok(result)
}
#[tauri::command]
pub async fn github_logout(app: AppHandle) -> Reply<()> {
    crate::github::logout()?;
    runtime::changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn update_check(app: AppHandle) -> Reply<Value> {
    crate::update::check(&app).await
}
#[tauri::command]
pub async fn update_install(app: AppHandle) -> Reply<Value> {
    crate::update::install(&app).await
}
#[tauri::command]
pub async fn context_clear(app: AppHandle, state: State<'_, AppState>) -> Reply<()> {
    let cleared = state
        .0
        .context
        .lock()
        .map_err(|_| "上下文不可用")?
        .take();
    if let Some(context) = cleared {
        if let Ok(mut dismissed) = state.0.dismissed.lock() {
            *dismissed = Some(selection_signature(&context));
        }
    }
    runtime::changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn runtime_cancel(state: State<'_, AppState>) -> Reply<()> {
    if state.active_id().is_some() {
        return Err("文件任务请使用任务取消入口".into());
    }
    if let Some(cancel) = state
        .0
        .runtime_cancel
        .lock()
        .map_err(|_| "运行时状态不可用")?
        .as_ref()
    {
        cancel.store(true, Ordering::Release);
    }
    Ok(())
}
#[tauri::command]
pub async fn permissions_request(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Reply<Value> {
    if ["full_disk", "files"].contains(&id.as_str()) {
        // 先探测：首次访问受保护文件夹会触发系统授权弹窗，允许后系统列表
        // 才会出现条目；仍未授权时再打开系统设置对应面板。
        let probe_id = id.clone();
        let granted = tokio::task::spawn_blocking(move || {
            platform::run_probe(&probe_id).is_some_and(|item| item.status == "granted")
        })
        .await
        .map_err(|e| e.to_string())?;
        if !granted {
            let pane = if id == "full_disk" {
                "Privacy_AllFiles"
            } else {
                "Privacy_FilesAndFolders"
            };
            app.opener()
                .open_url(
                    format!("x-apple.systempreferences:com.apple.preference.security?{pane}"),
                    None::<&str>,
                )
                .map_err(|e| e.to_string())?;
        }
    } else if ["accessibility", "automation"].contains(&id.as_str()) {
        let id = id.clone();
        tokio::task::spawn_blocking(move || {
            platform::host_call(json!({"operation":"request_permission","id":id}))
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    } else {
        return Err("未知权限项".into());
    }
    runtime::changed(&app);
    app_snapshot(state).await
}
#[tauri::command]
pub async fn permissions_check(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Reply<Value> {
    tokio::task::spawn_blocking(|| {
        platform::run_probe("files");
        platform::run_probe("full_disk");
    })
    .await
    .map_err(|e| e.to_string())?;
    runtime::changed(&app);
    app_snapshot(state).await
}
/// 选区签名：目录 + 文件名集合。用户清除过同一选区后不再自动回填；
/// 一旦 Finder 里的选择发生变化，标记立刻失效，新的选区照常跟随。
fn selection_signature(context: &ContextSnapshot) -> String {
    format!("{}|{}", context.directory, context.files.join("\u{1}"))
}
fn dismissed(state: &AppState, context: &ContextSnapshot) -> bool {
    let signature = selection_signature(context);
    match state.0.dismissed.lock() {
        Ok(mut value) => {
            if value.as_deref() == Some(signature.as_str()) {
                return true;
            }
            *value = None;
            false
        }
        Err(_) => false,
    }
}
#[tauri::command]
pub async fn context_capture(app: AppHandle, state: State<'_, AppState>) -> Reply<ContextSnapshot> {
    let state = state.inner().clone();
    let context = tokio::task::spawn_blocking(|| NativeHost.context())
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    if dismissed(&state, &context) {
        return Ok(context);
    }
    if !context.files.is_empty() {
        if let Ok(mut last) = state.0.last_selection.lock() {
            *last = Some(context.clone());
        }
    }
    *state.0.context.lock().map_err(|_| "上下文不可用")? = Some(context.clone());
    runtime::changed(&app);
    Ok(context)
}
/// 底栏轮询用的轻量探测：只读取 Finder 当前上下文，不落库、不广播，
/// 前端据此判断选区是否变化后再决定是否真正捕获。
#[tauri::command]
pub async fn context_peek(state: State<'_, AppState>) -> Reply<ContextSnapshot> {
    let state = state.inner().clone();
    let context = tokio::task::spawn_blocking(|| NativeHost.context())
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    if !context.files.is_empty() {
        if let Ok(mut last) = state.0.last_selection.lock() {
            *last = Some(context.clone());
        }
    }
    Ok(context)
}
#[tauri::command]
pub async fn choose_files(
    app: AppHandle,
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
) -> Reply<Option<ContextSnapshot>> {
    let (send, receive) = tokio::sync::oneshot::channel();
    // 在唤起底栏的那个窗口上以表单方式弹出选择器：不激活主窗口，也不跳转页面。
    let mut dialog = app.dialog().file().set_title("选择本次任务的文件");
    if window.label() == "agent-bar" {
        dialog = dialog.set_parent(&window);
    }
    dialog.pick_files(move |files| {
        let _ = send.send(files);
    });
    let Some(files) = receive.await.map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    let paths = files
        .into_iter()
        .map(|p| p.into_path().map_err(|e| e.to_string()))
        .collect::<Reply<Vec<_>>>()?;
    let Some(first) = paths.first() else {
        return Ok(None);
    };
    let context = ContextSnapshot {
        id: uuid::Uuid::new_v4().to_string(),
        host_window: "explicit-selection".into(),
        directory: first
            .parent()
            .ok_or("文件位置无效")?
            .to_string_lossy()
            .into(),
        files: paths.iter().map(|p| p.to_string_lossy().into()).collect(),
        captured_at: now_ms(),
    };
    *state.0.context.lock().map_err(|_| "上下文不可用")? = Some(context.clone());
    runtime::changed(&app);
    Ok(Some(context))
}
#[tauri::command]
pub async fn choose_directory(app: AppHandle) -> Reply<Option<String>> {
    let (send, receive) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("选择文件夹")
        .pick_folder(move |path| {
            let _ = send.send(path);
        });
    receive
        .await
        .map_err(|e| e.to_string())?
        .map(|p| {
            p.into_path()
                .map(|p| p.to_string_lossy().into())
                .map_err(|e| e.to_string())
        })
        .transpose()
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubmitInput {
    request_id: String,
    prompt: String,
    context_id: Option<String>,
}
#[tauri::command]
pub async fn task_submit(
    app: AppHandle,
    state: State<'_, AppState>,
    input: SubmitInput,
) -> Reply<TaskRecord> {
    let state = state.inner().clone();
    if let Some(existing) = state
        .0
        .db
        .task_by_request(&input.request_id)
        .map_err(|e| e.to_string())?
    {
        return Ok(existing);
    }
    let guard = state
        .0
        .runtime_gate
        .clone()
        .try_lock_owned()
        .map_err(|_| "已有任务或连接检查正在运行")?;
    let settings = state.0.db.settings().map_err(|e| e.to_string())?;
    if !settings.bar_enabled {
        return Err("底部功能已关闭，请先在设置中启用".into());
    }
    let profile = state
        .0
        .db
        .profile(
            settings
                .default_profile_id
                .as_deref()
                .ok_or("请先在设置中配置模型连接")?,
        )
        .map_err(|e| e.to_string())?;
    profile.validate_selection().map_err(|e| e.to_string())?;
    let cached = state.0.context.lock().map_err(|_| "上下文不可用")?.clone();
    let context = if let Some(id) = input.context_id {
        cached
            .filter(|c| c.id == id)
            .ok_or("文件选区已更新，请重新检查后提交")?
    } else {
        let fresh = tokio::task::spawn_blocking(|| NativeHost.context())
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        // Finder 在焦点切换的瞬间可能向 AppleScript 报告空选区。若提交时
        // 捕获为空、而最近一次探测确实带文件且目录一致，则采用那次记录。
        if fresh.files.is_empty() {
            let last = state.0.last_selection.lock().map_err(|_| "上下文不可用")?.clone();
            if let Some(last) = last {
                let recent = now_ms().saturating_sub(last.captured_at) <= 10 * 60 * 1000;
                if recent && last.directory == fresh.directory {
                    *state.0.context.lock().map_err(|_| "上下文不可用")? = Some(last.clone());
                    last
                } else {
                    fresh
                }
            } else {
                fresh
            }
        } else {
            if let Ok(mut last) = state.0.last_selection.lock() {
                *last = Some(fresh.clone());
            }
            fresh
        }
    };
    let task = state
        .0
        .db
        .create_task(&TaskRequest {
            request_id: input.request_id,
            prompt: input.prompt,
            context,
            profile: profile.clone(),
        })
        .map_err(|e| e.to_string())?;
    launch_task(app, state, task, guard, "task", None)
}
fn launch_task(
    app: AppHandle,
    state: AppState,
    task: TaskRecord,
    guard: tokio::sync::OwnedMutexGuard<()>,
    mode: &'static str,
    temporary: Option<tempfile::TempDir>,
) -> Reply<TaskRecord> {
    let profile = task.profile.clone();
    let cancelled = Arc::new(AtomicBool::new(false));
    *state.0.active.lock().map_err(|_| "任务状态不可用")? = Some(ActiveTask {
        id: task.id.clone(),
        profile_id: profile.id.clone(),
        cancelled: cancelled.clone(),
    });
    *state
        .0
        .runtime_cancel
        .lock()
        .map_err(|_| "运行时状态不可用")? = Some(cancelled.clone());
    let running = task.clone();
    let app_copy = app.clone();
    tauri::async_runtime::spawn(async move {
        let _guard = guard;
        let _temporary = temporary;
        let result = runtime::run(
            app_copy.clone(),
            state.clone(),
            mode,
            profile,
            Some(running.clone()),
            cancelled.clone(),
        )
        .await;
        if let Err(error) = result {
            let interrupted = state
                .0
                .db
                .task(&running.id)
                .ok()
                .is_some_and(|t| t.actions.iter().any(|a| a.status == "running"));
            let _ = state.0.db.update_task(
                &running.id,
                if interrupted {
                    TaskStatus::NeedsReview
                } else if cancelled.load(Ordering::Acquire) {
                    TaskStatus::Cancelled
                } else {
                    TaskStatus::Failed
                },
                &error,
                0,
                None,
            );
        }
        cancelled.store(true, Ordering::Release);
        if let Ok(mut active) = state.0.active.lock() {
            *active = None;
        }
        if let Ok(mut cancel) = state.0.runtime_cancel.lock() {
            *cancel = None;
        }
        if let Ok(mut pending) = state.0.pending.lock() {
            pending.retain(|_, p| p.info.task_id.as_deref() != Some(&running.id));
        }
        runtime::changed(&app_copy);
        crate::shell::refresh_menu(&app_copy);
    });
    runtime::changed(&app);
    crate::shell::refresh_menu(&app);
    Ok(task)
}
#[tauri::command]
pub async fn selftest_start(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Reply<Option<TaskRecord>> {
    let context = tokio::task::spawn_blocking(|| NativeHost.context())
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let Some(directory) = choose_directory(app.clone()).await? else {
        return Ok(None);
    };
    let state = state.inner().clone();
    let guard = state
        .0
        .runtime_gate
        .clone()
        .try_lock_owned()
        .map_err(|_| "已有任务或连接检查正在运行")?;
    let settings = state.0.db.settings().map_err(|e| e.to_string())?;
    let profile = state
        .0
        .db
        .profile(
            settings
                .default_profile_id
                .as_deref()
                .ok_or("请先配置模型连接")?,
        )
        .map_err(|e| e.to_string())?;
    profile.validate_selection().map_err(|e| e.to_string())?;
    let temporary = tempfile::Builder::new()
        .prefix(".fleqi-self-check-")
        .tempdir_in(directory)
        .map_err(|e| e.to_string())?;
    let directory = temporary.path().to_string_lossy().into_owned();
    let context = ContextSnapshot {
        id: uuid::Uuid::new_v4().to_string(),
        host_window: context.host_window,
        directory: directory.clone(),
        files: vec![directory],
        captured_at: now_ms(),
    };
    let prompt="执行 Fleqi 文件自检：在提供的测试目录内新建 Fleqi-check.txt，写入“Fleqi 自检成功”；读取正文；重命名为 Fleqi-verified.txt；最后调用 fleqi_selftest_cleanup 清理测试内容。四步都必须真实调用工具完成，不能只描述操作。只处理此测试目录。".to_string();
    let task = state
        .0
        .db
        .create_task(&TaskRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            prompt,
            context,
            profile,
        })
        .map_err(|e| e.to_string())?;
    Ok(Some(launch_task(
        app,
        state,
        task,
        guard,
        "selftest",
        Some(temporary),
    )?))
}
#[tauri::command]
pub async fn task_cancel(app: AppHandle, state: State<'_, AppState>) -> Reply<()> {
    if let Some(active) = state
        .0
        .active
        .lock()
        .map_err(|_| "任务状态不可用")?
        .as_ref()
    {
        active.cancelled.store(true, Ordering::Release);
    }
    runtime::changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn interaction_answer(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    answer: String,
) -> Reply<()> {
    if answer.len() > 16_000 {
        return Err("回答过长".into());
    }
    let pending = state
        .0
        .pending
        .lock()
        .map_err(|_| "交互状态不可用")?
        .remove(&id)
        .ok_or("此问题已处理或取消")?;
    pending.response.send(answer).map_err(|_| "此问题已失效")?;
    runtime::changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn history_clear(app: AppHandle, state: State<'_, AppState>) -> Reply<()> {
    state.0.db.clear_history().map_err(|e| e.to_string())?;
    runtime::changed(&app);
    crate::shell::refresh_menu(&app);
    Ok(())
}
#[tauri::command]
pub async fn result_reveal(
    app: AppHandle,
    state: State<'_, AppState>,
    task_id: String,
    path: String,
) -> Reply<()> {
    let task = state.0.db.task(&task_id).map_err(|e| e.to_string())?;
    if !task.actions.iter().any(|a| {
        a.result
            .as_ref()
            .and_then(|v| v["outputs"].as_array())
            .is_some_and(|paths| paths.iter().any(|v| v.as_str() == Some(&path)))
    }) {
        return Err("该路径不是此任务的输出".into());
    }
    app.opener()
        .reveal_item_in_dir(PathBuf::from(path))
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn diagnostics_export(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Reply<Option<String>> {
    let (send, receive) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_file_name("Fleqi-diagnostics.json")
        .save_file(move |path| {
            let _ = send.send(path);
        });
    let Some(path) = receive.await.map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let tasks = state.0.db.tasks(100).map_err(|e| e.to_string())?;
    let report = json!({"version":env!("CARGO_PKG_VERSION"),"platform":NativeHost.capabilities(),"runtime":{"node":"24.21.0","pi":"0.85.0"},"tasks":tasks.iter().map(|t|json!({"id":t.id,"status":t.status,"createdAt":t.created_at,"actions":t.actions.iter().map(|a|json!({"operation":a.operation,"status":a.status})).collect::<Vec<_>>()})).collect::<Vec<_>>()});
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(Some(path.to_string_lossy().into()))
}
#[tauri::command]
pub async fn engines_check(state: State<'_, AppState>) -> Reply<Value> {
    let state = state.inner().clone();
    tokio::task::spawn_blocking(move || {
        let mut result = serde_json::Map::new();
        for (id, path, arg) in [
            ("ffmpeg", &state.0.engines.ffmpeg, "-version"),
            ("ffprobe", &state.0.engines.ffprobe, "-version"),
            ("qpdf", &state.0.engines.qpdf, "--version"),
        ] {
            result.insert(
                id.into(),
                json!(
                    path.as_ref()
                        .map(|p| version(p, arg))
                        .unwrap_or_else(|| "尚未打包".into())
                ),
            );
        }
        result.insert(
            "node".into(),
            json!(version(
                &state.0.resources.join("bin").join(if cfg!(windows) {
                    "node.exe"
                } else {
                    "node"
                }),
                "--version"
            )),
        );
        Ok(Value::Object(result))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn open_settings(
    app: AppHandle,
    window: tauri::WebviewWindow,
    page: Option<String>,
    origin: Option<crate::window_motion::Origin>,
) -> Reply<()> {
    use tauri::Manager;
    if origin.is_some_and(|o| !o.valid()) {
        return Err("动画起点无效".into());
    }
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    if let Some(settings) = app.get_webview_window("settings") {
        crate::window_motion::visibility(settings, Some(window), origin, true)?;
        app.emit_to(
            "settings",
            "fleqi:navigate",
            page.as_deref().unwrap_or("general"),
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn bar_toggle(app: AppHandle, state: State<'_, AppState>, show: bool) -> Reply<()> {
    if show
        && !state
            .0
            .db
            .settings()
            .map_err(|e| e.to_string())?
            .bar_enabled
    {
        return Err("请先启用底部功能".into());
    }
    // 每次唤起都是全新上下文：旧选区立即作废，焦点事件触发重新捕获。
    if let Ok(mut context) = state.0.context.lock() {
        *context = None;
    }
    if let Ok(mut dismissed) = state.0.dismissed.lock() {
        *dismissed = None;
    }
    tokio::task::spawn_blocking(move || {
        platform::host_call(json!({"operation":if show{"show"}else{"hide"}}))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    if show {
        let _ = app.emit_to("agent-bar", "fleqi:focus", ());
    }
    runtime::changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn bar_resize(height: f64, bar_height: Option<f64>, bar_offset: Option<f64>) -> Reply<Value> {
    platform::host_call(json!({
        "operation": "height",
        "height": height,
        "barHeight": bar_height.unwrap_or(56.),
        "barOffset": bar_offset.unwrap_or(0.)
    }))
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn shortcut_record(
    app: AppHandle,
    window: tauri::WebviewWindow,
    recording: bool,
) -> Reply<()> {
    if window.label() != "settings" {
        return Err("请在设置窗口录入快捷键".into());
    }
    crate::shell::record_shortcut(&app, recording)
}
#[tauri::command]
pub async fn close_settings(app: AppHandle) -> Reply<()> {
    crate::shell::record_shortcut(&app, false)?;
    crate::shell::hide_window(&app, "settings");
    Ok(())
}
