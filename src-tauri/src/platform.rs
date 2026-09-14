use fleqi_core::{Error, Result, model::*};
use serde_json::{Value, json};

pub struct NativeHost;
impl HostAdapter for NativeHost {
    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities {
            platform: std::env::consts::OS.into(),
            host_name: if cfg!(target_os = "macos") {
                "Finder"
            } else if cfg!(target_os = "windows") {
                "文件资源管理器"
            } else {
                "Linux 文件管理器"
            }
            .into(),
            host_attachment: cfg!(target_os = "macos"),
            tray: true,
            credential_store: cfg!(target_os = "macos"),
            detail: if cfg!(target_os = "macos") {
                "Finder 集成需辅助功能和自动化授权"
            } else {
                "公共核心已准备；本平台的窗口挂靠与凭据存储尚待实现和验收"
            }
            .into(),
        }
    }
    fn context(&self) -> Result<ContextSnapshot> {
        Ok(serde_json::from_value(host_call(
            json!({"operation":"context"}),
        )?)?)
    }
    fn permissions(&self) -> Result<Vec<PermissionItem>> {
        #[cfg(target_os = "macos")]
        {
            let mut items: Vec<PermissionItem> =
                serde_json::from_value(host_call(json!({"operation":"permissions"}))?)?;
            let stored = probes::stored();
            for item in &mut items {
                match item.id.as_str() {
                    "files" => {
                        if let Some(probed) = &stored.files {
                            *item = probed.clone();
                        }
                    }
                    "full_disk" => {
                        if let Some(probed) = &stored.full_disk {
                            *item = probed.clone();
                        }
                    }
                    _ => {}
                }
            }
            Ok(items)
        }
        #[cfg(not(target_os = "macos"))]
        {
            Ok(vec![PermissionItem {
                id: "host".into(),
                name: "文件管理器集成".into(),
                purpose: "窗口与选区".into(),
                status: "unsupported".into(),
                detail: "本平台尚未完成宿主适配，不能用 macOS 权限结果代替".into(),
                required: true,
            }])
        }
    }
}

/// macOS 不提供 TCC 授权状态的查询接口，界面上的“待验证”只能通过真实
/// 访问探针消除。files 探测桌面/文稿/下载（首次访问会触发系统授权弹窗，
/// 允许后条目才会出现在系统“文件与文件夹”列表）；full_disk 以读取受完全
/// 磁盘访问保护的 TCC 数据库为判据，探测本身不会触发弹窗。结果仅存在
/// 内存中，重启后需重新检查。
#[cfg(target_os = "macos")]
pub mod probes {
    use fleqi_core::model::PermissionItem;
    use serde_json::json;
    use std::io::Read;
    use std::path::PathBuf;
    use std::sync::Mutex;

    #[derive(Default)]
    pub struct Stored {
        pub files: Option<PermissionItem>,
        pub full_disk: Option<PermissionItem>,
    }
    static STORED: Mutex<Stored> = Mutex::new(Stored {
        files: None,
        full_disk: None,
    });
    static STORE_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

    /// 启动时载入上次会话的探测结果，让界面立即显示已授权状态。
    pub fn init_store(path: PathBuf) {
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Ok(mut stored) = STORED.lock() {
                    stored.files = value
                        .get("files")
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                    stored.full_disk = value
                        .get("fullDisk")
                        .and_then(|v| serde_json::from_value(v.clone()).ok());
                }
            }
        }
        if let Ok(mut slot) = STORE_PATH.lock() {
            *slot = Some(path);
        }
    }
    fn persist(stored: &Stored) {
        let Ok(path) = STORE_PATH.lock() else { return };
        let Some(path) = path.as_ref() else { return };
        let value = json!({
            "files": stored.files,
            "fullDisk": stored.full_disk,
        });
        let _ = std::fs::write(path, serde_json::to_string(&value).unwrap_or_default());
    }

    pub fn stored() -> Stored {
        STORED
            .lock()
            .map(|s| Stored {
                files: s.files.clone(),
                full_disk: s.full_disk.clone(),
            })
            .unwrap_or_default()
    }

    fn files_item(status: &str, detail: &str) -> PermissionItem {
        PermissionItem {
            id: "files".into(),
            name: "文件与文件夹".into(),
            purpose: "读取与处理选中的文件".into(),
            status: status.into(),
            detail: detail.into(),
            required: true,
        }
    }
    fn full_disk_item(status: &str, detail: &str) -> PermissionItem {
        PermissionItem {
            id: "full_disk".into(),
            name: "完全磁盘访问".into(),
            purpose: "处理受保护位置中的文件".into(),
            status: status.into(),
            detail: detail.into(),
            required: false,
        }
    }

    /// 列目录内容：未授权时 TCC 返回 EPERM；列目录本身即会触发首次授权弹窗。
    fn folder_readable(path: PathBuf) -> bool {
        std::fs::read_dir(path).is_ok()
    }
    /// 读取受完全磁盘访问保护的系统 TCC 数据库；无授权时为 EPERM，且不会弹窗。
    fn full_disk_granted() -> bool {
        std::fs::File::open("/Library/Application Support/com.apple.TCC/TCC.db")
            .and_then(|mut file| {
                let mut byte = [0u8; 1];
                file.read_exact(&mut byte)
            })
            .is_ok()
    }

    pub fn run(id: &str) -> Option<PermissionItem> {
        let home = PathBuf::from(std::env::var("HOME").ok()?);
        let item = match id {
            "files" => {
                let checked = ["Desktop", "Documents", "Downloads"]
                    .map(|folder| folder_readable(home.join(folder)));
                let granted = checked.iter().filter(|ok| **ok).count();
                match granted {
                    3 => Some(files_item(
                        "granted",
                        "桌面、文稿与下载均已授权，可直接读取这些位置的文件",
                    )),
                    0 => Some(files_item(
                        "not_granted",
                        "尚未授权受保护文件夹；检查时系统会逐项弹出授权请求，允许后条目会出现在系统“文件与文件夹”列表",
                    )),
                    _ => Some(files_item(
                        "partial",
                        "部分受保护文件夹已授权；其余位置可重新检查并在弹窗中允许，或到系统设置手动打开",
                    )),
                }
            }
            "full_disk" => {
                if full_disk_granted() {
                    Some(full_disk_item(
                        "granted",
                        "完全磁盘访问已生效，可处理受保护位置的文件",
                    ))
                } else {
                    Some(full_disk_item(
                        "not_granted",
                        "完全磁盘访问未生效；请在系统设置中允许后重新检查。系统开关不会自动反馈到应用",
                    ))
                }
            }
            _ => None,
        }?;
        if let Ok(mut stored) = STORED.lock() {
            match item.id.as_str() {
                "files" => stored.files = Some(item.clone()),
                "full_disk" => stored.full_disk = Some(item.clone()),
                _ => {}
            }
            persist(&stored);
        }
        Some(item)
    }

    /// 快照时的静默自动检查：完全磁盘访问探测无副作用，始终执行；文件与
    /// 文件夹只在上一结果为已授权时复核（撤销后系统静默拒绝，不会弹窗），
    /// 避免每次启动都对未授权目录弹出系统请求。
    pub fn auto() {
        run("full_disk");
        if stored().files.is_some_and(|item| item.status == "granted") {
            run("files");
        }
    }
}
#[cfg(target_os = "macos")]
pub fn run_probe(id: &str) -> Option<PermissionItem> {
    probes::run(id)
}
#[cfg(target_os = "macos")]
pub fn init_probe_store(path: std::path::PathBuf) {
    probes::init_store(path)
}
#[cfg(target_os = "macos")]
pub fn auto_probe_permissions() {
    probes::auto()
}
#[cfg(not(target_os = "macos"))]
pub fn run_probe(_id: &str) -> Option<PermissionItem> {
    None
}
#[cfg(not(target_os = "macos"))]
pub fn init_probe_store(_path: std::path::PathBuf) {}
#[cfg(not(target_os = "macos"))]
pub fn auto_probe_permissions() {}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use std::ffi::{CStr, CString, c_char, c_void};
    unsafe extern "C" {
        fn fleqi_host_command(request: *const c_char) -> *mut c_char;
        fn fleqi_credential(request: *const c_char) -> *mut c_char;
        fn fleqi_string_free(pointer: *mut c_char);
        pub fn fleqi_attach_panel(pointer: *mut c_void);
        pub fn fleqi_prepare_panel(pointer: *mut c_void);
        pub fn fleqi_finish_panel(pointer: *mut c_void);
    }
    pub fn call(value: Value, credential: bool) -> Result<Value> {
        let request = CString::new(serde_json::to_string(&value)?)
            .map_err(|_| Error::Invalid("原生请求无效".into()))?;
        // SAFETY: Swift reads a valid C string synchronously and returns an owned
        // strdup buffer. The matching Swift free function releases it exactly once.
        // Host commands marshal AppKit access to the main thread internally.
        let pointer = unsafe {
            if credential {
                fleqi_credential(request.as_ptr())
            } else {
                fleqi_host_command(request.as_ptr())
            }
        };
        if pointer.is_null() {
            return Err(Error::Invalid("原生模块没有返回结果".into()));
        }
        // SAFETY: the non-null buffer is NUL-terminated and alive until freed below.
        let bytes = unsafe { CStr::from_ptr(pointer) }.to_bytes();
        let parsed = serde_json::from_slice::<Value>(bytes);
        // SAFETY: this is the owned response from the corresponding Swift allocator.
        unsafe { fleqi_string_free(pointer) };
        let result = parsed?;
        if let Some(error) = result.get("error").and_then(Value::as_str) {
            return Err(Error::Invalid(error.into()));
        }
        Ok(result)
    }
}
#[cfg(target_os = "macos")]
pub fn conversion_content(
    window: &tauri::WebviewWindow,
    detach: bool,
) -> std::result::Result<(), String> {
    let pointer = window.ns_window().map_err(|e| e.to_string())?;
    // SAFETY: called only by create_bar during Tauri's main-thread setup. The
    // window is retained by the caller; Swift retains detached content until
    // the matching restore, so WebKit never observes a class-swizzled window.
    unsafe {
        if detach {
            macos::fleqi_prepare_panel(pointer)
        } else {
            macos::fleqi_finish_panel(pointer)
        }
    };
    Ok(())
}
pub fn host_call(value: Value) -> Result<Value> {
    #[cfg(target_os = "macos")]
    {
        macos::call(value, false)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = value;
        Err(Error::Invalid("本平台的文件管理器挂靠尚未实现".into()))
    }
}
pub fn credential(operation: &str, profile: &str, value: Option<Value>) -> Result<Value> {
    if profile.is_empty()
        || profile.len() > 100
        || !profile
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(Error::Invalid("连接编号无效".into()));
    }
    #[cfg(target_os = "macos")]
    {
        let response = macos::call(
            json!({"operation":operation,"profile":profile,"credential":value}),
            true,
        )?;
        Ok(response.get("value").cloned().unwrap_or(Value::Null))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (operation, value);
        Err(Error::Invalid("本平台系统凭据存储尚未接入".into()))
    }
}
#[cfg(target_os = "macos")]
pub fn attach_panel(window: &tauri::WebviewWindow) -> std::result::Result<(), String> {
    let handle = window.clone();
    window
        .run_on_main_thread(move || {
            if let Ok(pointer) = handle.ns_window() {
                // SAFETY: Tauri owns the live NSWindow, retained in this closure.
                // Swift stores only a weak reference, and this runs on the main thread.
                unsafe { macos::fleqi_attach_panel(pointer) };
            }
        })
        .map_err(|e| e.to_string())
}
