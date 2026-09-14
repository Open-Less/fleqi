use serde::Deserialize;
use tauri::WebviewWindow;

#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Origin {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Origin {
    pub fn valid(self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 16_384.)
            && self.width > 0.
            && self.height > 0.
    }
}
pub fn visibility(
    window: WebviewWindow,
    source: Option<WebviewWindow>,
    origin: Option<Origin>,
    visible: bool,
) -> Result<(), String> {
    if origin.is_some_and(|o| !o.valid()) {
        return Err("动画起点无效".into());
    }
    #[cfg(target_os = "macos")]
    {
        use std::ffi::c_void;
        unsafe extern "C" {
            fn fleqi_window_visibility(
                window: *mut c_void,
                source: *mut c_void,
                x: f64,
                y: f64,
                width: f64,
                height: f64,
                visible: i32,
            );
        }
        let handle = window.clone();
        window
            .run_on_main_thread(move || {
                if let Ok(pointer) = handle.ns_window() {
                    let source = source
                        .as_ref()
                        .and_then(|w| w.ns_window().ok())
                        .unwrap_or(std::ptr::null_mut());
                    let o = origin.unwrap_or(Origin {
                        x: 0.,
                        y: 0.,
                        width: 0.,
                        height: 0.,
                    });
                    // SAFETY: both handles remain owned by Tauri and all AppKit work
                    // runs on the main thread; Swift retains only weak window references.
                    unsafe {
                        fleqi_window_visibility(
                            pointer,
                            source,
                            o.x,
                            o.y,
                            o.width,
                            o.height,
                            i32::from(visible),
                        );
                    }
                }
            })
            .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (source, origin);
        if visible {
            window.show().and_then(|_| window.set_focus())
        } else {
            window.hide()
        }
        .map_err(|e| e.to_string())
    }
}
