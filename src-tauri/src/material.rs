use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Material {
    Frosted,
    Liquid,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MaterialRequest {
    material: Material,
    theme: Theme,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    radius: f64,
    scene: bool,
}

impl MaterialRequest {
    fn validate(&self) -> Result<(), String> {
        let values = [self.x, self.y, self.width, self.height, self.radius];
        if values.iter().any(|value| !value.is_finite()) {
            return Err("material geometry must be finite".into());
        }
        if !(1.0..=16_384.0).contains(&self.width)
            || !(1.0..=16_384.0).contains(&self.height)
            || !(0.0..=128.0).contains(&self.radius)
            || self.x.abs() > 16_384.0
            || self.y.abs() > 16_384.0
        {
            return Err("material geometry is out of bounds".into());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialSupport {
    native: bool,
    liquid: bool,
    reduce_transparency: bool,
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use std::ffi::c_void;
    use tokio::sync::oneshot;

    unsafe extern "C" {
        fn fleqi_material_support() -> i32;
        fn fleqi_apply_material(
            window: *mut c_void,
            material: i32,
            dark: i32,
            x: f64,
            y: f64,
            width: f64,
            height: f64,
            radius: f64,
            scene: i32,
        ) -> i32;
    }

    pub async fn support(window: tauri::WebviewWindow) -> Result<MaterialSupport, String> {
        let (send, receive) = oneshot::channel();
        window
            .run_on_main_thread(move || {
                // SAFETY: Swift reads AppKit state synchronously on its required main thread.
                let flags = unsafe { fleqi_material_support() };
                let _ = send.send(MaterialSupport {
                    native: true,
                    liquid: flags & 1 != 0,
                    reduce_transparency: flags & 2 != 0,
                });
            })
            .map_err(|error| error.to_string())?;
        receive.await.map_err(|error| error.to_string())
    }

    pub async fn apply(
        window: tauri::WebviewWindow,
        request: MaterialRequest,
    ) -> Result<Material, String> {
        let (send, receive) = oneshot::channel();
        let native_window = window.clone();
        window
            .run_on_main_thread(move || {
                let result = native_window
                    .ns_window()
                    .map_err(|error| error.to_string())
                    .and_then(|pointer| {
                        // SAFETY: Tauri owns this live NSWindow. The retained window handle stays in
                        // this closure, Swift uses the pointer only during this synchronous call,
                        // geometry is validated, and AppKit is always called on the main thread.
                        let applied = unsafe {
                            fleqi_apply_material(
                                pointer,
                                i32::from(matches!(request.material, Material::Liquid)),
                                i32::from(matches!(request.theme, Theme::Dark)),
                                request.x,
                                request.y,
                                request.width,
                                request.height,
                                request.radius,
                                i32::from(request.scene),
                            )
                        };
                        match applied {
                            0 => Ok(Material::Frosted),
                            1 => Ok(Material::Liquid),
                            _ => Err("native material view is unavailable".into()),
                        }
                    });
                let _ = send.send(result);
            })
            .map_err(|error| error.to_string())?;
        receive.await.map_err(|error| error.to_string())?
    }
}

#[tauri::command]
pub async fn material_support(window: tauri::WebviewWindow) -> Result<MaterialSupport, String> {
    #[cfg(target_os = "macos")]
    {
        macos::support(window).await
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
        Ok(MaterialSupport {
            native: false,
            liquid: false,
            reduce_transparency: false,
        })
    }
}

#[tauri::command]
pub async fn apply_material(
    window: tauri::WebviewWindow,
    request: MaterialRequest,
) -> Result<Material, String> {
    request.validate()?;
    #[cfg(target_os = "macos")]
    {
        macos::apply(window, request).await
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window, request);
        Err("native materials require macOS".into())
    }
}
