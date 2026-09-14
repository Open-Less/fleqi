mod auth;
mod commands;
mod github;
mod material;
mod platform;
mod runtime;
mod runtime_package;
mod shell;
mod state;
mod update;
mod window_motion;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tauri::Manager;

pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            shell::show_main(app, "overview")
        }))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .arg("--background")
                .build(),
        )
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build());
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());
    let result = builder
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            platform::init_probe_store(data_dir.join("permission-probes.json"));
            let db = Arc::new(fleqi_core::db::Database::open(
                &data_dir.join("fleqi.sqlite"),
            )?);
            db.recover_interrupted()?;
            let bundled = app.path().resource_dir()?.join("runtime");
            let resources = if cfg!(debug_assertions) && !bundled.join("pi.tar.gz").exists() {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("runtime-resources")
            } else {
                bundled
            };
            let pi_dir = runtime_package::prepare(&resources, &data_dir)?;
            let engine = |name: &str| {
                let p = resources.join("bin").join(if cfg!(windows) {
                    format!("{name}.exe")
                } else {
                    name.into()
                });
                p.is_file().then_some(p)
            };
            let engines = fleqi_core::files::EnginePaths {
                ffmpeg: engine("ffmpeg"),
                ffprobe: engine("ffprobe"),
                qpdf: engine("qpdf"),
            };
            app.manage(state::AppState(Arc::new(state::CoreState {
                db,
                data_dir,
                resources,
                pi_dir,
                engines,
                context: Mutex::new(None),
                last_selection: Mutex::new(None),
                dismissed: Mutex::new(None),
                auth: Mutex::new(None),
                active: Mutex::new(None),
                runtime_gate: Arc::new(tokio::sync::Mutex::new(())),
                runtime_cancel: Mutex::new(None),
                configuration_gate: tokio::sync::Mutex::new(()),
                shortcut_recording: std::sync::atomic::AtomicBool::new(false),
                pending: Mutex::new(HashMap::new()),
            })));
            let motion: serde_json::Value =
                serde_json::from_str(include_str!("../../src/motion/tokens.json"))?;
            let _ = platform::host_call(
                serde_json::json!({"operation":"motion_config","config":motion}),
            );
            shell::create_bar(app.handle())?;
            shell::install(app.handle())?;
            let settings = app.state::<state::AppState>().0.db.settings()?;
            if settings.bar_enabled && settings.activation == "automatic" {
                let _ = platform::host_call(
                    serde_json::json!({"operation":"automatic","enabled":true}),
                );
            }
            if std::env::args().any(|a| a == "--background") {
                if let Some(window) = app.get_webview_window("main") {
                    window.hide()?;
                }
                #[cfg(target_os = "macos")]
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            } else {
                shell::show_main(app.handle(), "overview");
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Focused(false)) && window.label() == "settings" {
                let _ = shell::record_shortcut(window.app_handle(), false);
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if window.label() == "main" || window.label() == "settings" {
                    shell::hide_window(window.app_handle(), window.label());
                } else if window.label() == "agent-bar" {
                    let _ = platform::host_call(serde_json::json!({"operation":"hide"}));
                } else {
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            material::material_support,
            material::apply_material,
            commands::app_snapshot,
            commands::import_appearance,
            commands::settings_save,
            commands::profile_save,
            commands::profile_delete,
            commands::profile_logout,
            commands::profile_run,
            commands::auth_open,
            commands::github_status,
            commands::github_login_start,
            commands::github_login_poll,
            commands::github_login_token,
            commands::github_open_tokens,
            commands::github_logout,
            commands::update_check,
            commands::update_install,
            commands::context_clear,
            commands::runtime_cancel,
            commands::permissions_request,
            commands::permissions_check,
            commands::context_capture,
            commands::context_peek,
            commands::choose_files,
            commands::choose_directory,
            commands::task_submit,
            commands::task_cancel,
            commands::selftest_start,
            commands::interaction_answer,
            commands::history_clear,
            commands::result_reveal,
            commands::diagnostics_export,
            commands::engines_check,
            commands::open_settings,
            commands::close_settings,
            commands::shortcut_record,
            commands::bar_toggle,
            commands::bar_resize,
        ])
        .build(tauri::generate_context!());

    match result {
        Ok(app) => app.run(|app, event| match event {
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => shell::show_main(app, "overview"),
            tauri::RunEvent::ExitRequested {
                code: None, api, ..
            } => {
                api.prevent_exit();
                shell::request_quit(app.clone());
            }
            _ => {}
        }),
        Err(error) => {
            eprintln!("failed to start Fleqi: {error}");
            std::process::exit(1);
        }
    }
}
