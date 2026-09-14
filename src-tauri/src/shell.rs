use crate::{commands, platform, runtime, state::AppState};
use serde_json::json;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tauri::{
    AppHandle, Emitter, Manager,
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

pub fn show_main(app: &AppHandle, page: &str) {
    let page = if [
        "overview",
        "general",
        "appearance",
        "models",
        "permissions",
        "files",
        "tasks",
        "about",
    ]
    .contains(&page)
    {
        page
    } else {
        "overview"
    };
    #[cfg(target_os = "macos")]
    {
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = crate::window_motion::visibility(window.clone(), None, None, true);
        let _ = app.emit_to("main", "fleqi:navigate", page);
    }
}
pub fn show_settings(app: &AppHandle, page: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    }
    if let Some(window) = app.get_webview_window("settings") {
        let _ = crate::window_motion::visibility(window.clone(), None, None, true);
        let _ = app.emit_to("settings", "fleqi:navigate", page);
    }
}
pub fn refresh_menu(app: &AppHandle) {
    if let Ok(menu) = make_menu(app) {
        if let Some(tray) = app.tray_by_id("fleqi") {
            let _ = tray.set_menu(Some(menu));
        }
    }
}
fn make_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let state = app.state::<AppState>();
    let settings = state.0.db.settings().unwrap_or_default();
    let tasks = state.0.db.tasks(5).unwrap_or_default();
    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(
        app,
        "open",
        "打开 Fleqi",
        true,
        None::<&str>,
    )?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        "bar-show",
        "显示底部栏",
        settings.bar_enabled,
        Some(settings.shortcut.as_str()),
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        "bar-hide",
        "收起底部栏",
        true,
        None::<&str>,
    )?)?;
    menu.append(&CheckMenuItem::with_id(
        app,
        "bar-enabled",
        "启用底部功能",
        true,
        settings.bar_enabled,
        None::<&str>,
    )?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        "tasks",
        if state.active_id().is_some() {
            "当前任务：处理中"
        } else {
            "当前没有运行任务"
        },
        true,
        None::<&str>,
    )?)?;
    if state.active_id().is_some() {
        menu.append(&MenuItem::with_id(
            app,
            "cancel",
            "取消当前任务",
            true,
            None::<&str>,
        )?)?;
    }
    for task in tasks {
        let label = format!(
            "{} · {}",
            match task.status {
                fleqi_core::model::TaskStatus::Completed => "完成",
                fleqi_core::model::TaskStatus::Partial => "部分完成",
                fleqi_core::model::TaskStatus::Cancelled => "已取消",
                fleqi_core::model::TaskStatus::NeedsReview => "待核对",
                fleqi_core::model::TaskStatus::Failed => "失败",
                _ => "进行中",
            },
            task.prompt.chars().take(22).collect::<String>()
        );
        menu.append(&MenuItem::with_id(
            app,
            format!("recent-{}", task.id),
            label,
            true,
            None::<&str>,
        )?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    for (id, label) in [
        ("models", "模型与账号…"),
        ("permissions", "权限检查…"),
        ("settings", "设置…"),
    ] {
        menu.append(&MenuItem::with_id(app, id, label, true, None::<&str>)?)?;
    }
    menu.append(&MenuItem::with_id(
        app,
        "p1",
        "侧边终端 · P1 后续提供",
        false,
        None::<&str>,
    )?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        "quit",
        "退出 Fleqi",
        true,
        Some("CmdOrCtrl+Q"),
    )?)?;
    Ok(menu)
}
pub fn install(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-template.png"))?;
    TrayIconBuilder::with_id("fleqi")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("Fleqi")
        .show_menu_on_left_click(true)
        .menu(&make_menu(app)?)
        .on_menu_event(|app, event| handle(app, event.id.as_ref()))
        .build(app)?;
    let shortcut = app.state::<AppState>().0.db.settings()?.shortcut;
    if let Err(error) = register_shortcut(app, &shortcut) {
        let _ = app.emit(
            "fleqi:notice",
            format!("快捷键注册失败，可从菜单栏打开：{error}"),
        );
    }
    Ok(())
}
fn handle(app: &AppHandle, id: &str) {
    match id {
        "open" => show_main(app, "overview"),
        "settings" => show_settings(app, "general"),
        "models" => show_settings(app, "models"),
        "permissions" => show_settings(app, "permissions"),
        "tasks" => show_main(app, "tasks"),
        id if id.starts_with("recent-") => show_main(app, "tasks"),
        "quit" => request_quit(app.clone()),
        "bar-show" | "bar-hide" | "bar-enabled" | "cancel" => {
            let app = app.clone();
            let id = id.to_string();
            tauri::async_runtime::spawn(async move {
                let state = app.state::<AppState>();
                let result = match id.as_str() {
                    "cancel" => commands::task_cancel(app.clone(), state).await,
                    "bar-enabled" => match state.0.db.settings() {
                        Ok(mut settings) => {
                            settings.bar_enabled = !settings.bar_enabled;
                            commands::settings_save(app.clone(), state, settings)
                                .await
                                .map(|_| ())
                        }
                        Err(error) => Err(error.to_string()),
                    },
                    _ => commands::bar_toggle(app.clone(), state, id == "bar-show").await,
                };
                if let Err(error) = result {
                    show_main(&app, "overview");
                    let _ = app.emit("fleqi:notice", error);
                }
            });
        }
        _ => {}
    }
}
fn register_shortcut(app: &AppHandle, shortcut: &str) -> Result<(), String> {
    app.global_shortcut()
        .on_shortcut(shortcut, |app, _, event| {
            if event.state == ShortcutState::Pressed {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let requested = platform::host_call(json!({"operation":"status"}))
                        .ok()
                        .and_then(|v| v["requested"].as_bool())
                        .unwrap_or(false);
                    if let Err(error) =
                        commands::bar_toggle(app.clone(), app.state::<AppState>(), !requested).await
                    {
                        show_main(&app, "overview");
                        let _ = app.emit("fleqi:notice", error);
                    }
                });
            }
        })
        .map_err(|e| e.to_string())
}
pub fn record_shortcut(app: &AppHandle, recording: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    if state.0.shortcut_recording.load(Ordering::Acquire) == recording {
        return Ok(());
    }
    let shortcut = state.0.db.settings().map_err(|e| e.to_string())?.shortcut;
    if recording {
        app.global_shortcut()
            .unregister(shortcut.as_str())
            .map_err(|e| e.to_string())?;
    } else {
        register_shortcut(app, &shortcut)?;
    }
    state
        .0
        .shortcut_recording
        .store(recording, Ordering::Release);
    Ok(())
}
pub fn replace_shortcut(app: &AppHandle, old: &str, new: &str) -> Result<(), String> {
    if old == new {
        return Ok(());
    }
    if app
        .state::<AppState>()
        .0
        .shortcut_recording
        .load(Ordering::Acquire)
    {
        register_shortcut(app, new)?;
        return app
            .global_shortcut()
            .unregister(new)
            .map_err(|e| e.to_string());
    }
    register_shortcut(app, new)?;
    if let Err(error) = app.global_shortcut().unregister(old) {
        let _ = app.global_shortcut().unregister(new);
        return Err(error.to_string());
    }
    Ok(())
}
pub fn request_quit(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>().inner().clone();
        if state.active_id().is_some() {
            show_main(&app, "tasks");
            let answer=runtime::ask(app.clone(),state.clone(),"quit".into(),"有文件任务正在运行。取消任务后退出，还是继续在后台运行？".into(),None,json!([{"id":"background","label":"继续后台运行"},{"id":"quit","label":"取消任务并退出"}]),Arc::new(AtomicBool::new(false))).await;
            if answer.as_deref() != Ok("quit") {
                return;
            }
        }
        if let Ok(cancel) = state.0.runtime_cancel.lock() {
            if let Some(cancel) = cancel.as_ref() {
                cancel.store(true, Ordering::Release);
            }
        }
        while state.0.runtime_gate.try_lock().is_err() {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        let _ = platform::host_call(json!({"operation":"hide"}));
        hide_window(&app, "settings");
        hide_window(&app, "main");
        let motion: serde_json::Value =
            serde_json::from_str(include_str!("../../src/motion/tokens.json")).unwrap_or_default();
        tokio::time::sleep(std::time::Duration::from_secs_f64(
            motion["duration"]["window"].as_f64().unwrap_or(0.32) + 0.05,
        ))
        .await;
        app.exit(0);
    });
}

#[cfg(target_os = "macos")]
pub fn create_bar(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    use tauri_nspanel::{PanelLevel, StyleMask, WebviewWindowExt, tauri_panel};
    tauri_panel! {panel!(FleqiAgentPanel{config:{can_become_key_window:true,can_become_main_window:false,is_floating_panel:true}})}
    let window = tauri::WebviewWindowBuilder::new(
        app,
        "agent-bar",
        tauri::WebviewUrl::App("index.html?surface=bar".into()),
    )
    .title("Fleqi")
    .inner_size(900., 68.)
    .decorations(false)
    .transparent(true)
    .visible(false)
    .build()?;
    platform::conversion_content(&window, true).map_err(std::io::Error::other)?;
    let converted = window.to_panel::<FleqiAgentPanel>();
    match converted {
        Ok(panel) => {
            panel.set_level(PanelLevel::Floating.value());
            panel.set_style_mask(
                StyleMask::empty()
                    .borderless()
                    .nonactivating_panel()
                    .value(),
            );
            panel.set_has_shadow(true);
            panel.set_hides_on_deactivate(false);
            platform::conversion_content(&window, false).map_err(std::io::Error::other)?;
            panel.hide();
            platform::attach_panel(&window).map_err(std::io::Error::other)?;
        }
        Err(error) => {
            let _ = platform::conversion_content(&window, false);
            return Err(Box::new(error));
        }
    }
    Ok(())
}
#[cfg(not(target_os = "macos"))]
pub fn create_bar(_app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

pub fn hide_window(app: &AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = crate::window_motion::visibility(window, None, None, false);
    }
    let app = app.clone();
    let label = label.to_string();
    tauri::async_runtime::spawn(async move {
        let motion: serde_json::Value =
            serde_json::from_str(include_str!("../../src/motion/tokens.json")).unwrap_or_default();
        let duration = motion["duration"]["window"].as_f64().unwrap_or(0.32);
        tokio::time::sleep(std::time::Duration::from_secs_f64(duration + 0.05)).await;
        if label == "settings"
            && app
                .get_webview_window("settings")
                .is_some_and(|w| !w.is_visible().unwrap_or(false))
        {
            let _ = crate::shell::record_shortcut(&app, false);
        }
        let any_visible = ["main", "settings"].iter().any(|label| {
            app.get_webview_window(label)
                .is_some_and(|w| w.is_visible().unwrap_or(false))
        });
        #[cfg(target_os = "macos")]
        if !any_visible {
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let _ = platform::host_call(serde_json::json!({"operation":"refocus"}));
        }
        #[cfg(not(target_os = "macos"))]
        let _ = any_visible;
    });
}
