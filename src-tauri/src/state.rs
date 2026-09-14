use fleqi_core::{db::Database, files::EnginePaths, model::*};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, atomic::AtomicBool},
};
use tokio::sync::{Mutex as AsyncMutex, oneshot};

#[derive(Clone)]
pub struct AppState(pub Arc<CoreState>);
pub struct CoreState {
    pub db: Arc<Database>,
    pub data_dir: PathBuf,
    pub resources: PathBuf,
    pub pi_dir: PathBuf,
    pub engines: EnginePaths,
    pub context: Mutex<Option<ContextSnapshot>>,
    /// 最近一次“确实带着选中文件”的上下文。Finder 在焦点切换瞬间可能向
    /// AppleScript 报告空选区，提交时用它兜底，避免“明明选了文件却说没选”。
    pub last_selection: Mutex<Option<ContextSnapshot>>,
    /// 用户主动清除过的选区签名；同一选区不再自动回填，直到 Finder 里的选择变化。
    pub dismissed: Mutex<Option<String>>,
    pub active: Mutex<Option<ActiveTask>>,
    pub runtime_gate: Arc<AsyncMutex<()>>,
    pub runtime_cancel: Mutex<Option<Arc<AtomicBool>>>,
    /// 最近一次账号登录下发的授权信息（授权链接或设备码），供界面显示
    /// “重新打开授权页面”，不包含任何凭据内容。
    pub auth: Mutex<Option<Value>>,
    pub configuration_gate: AsyncMutex<()>,
    pub shortcut_recording: AtomicBool,
    pub pending: Mutex<HashMap<String, PendingInteraction>>,
}
pub struct ActiveTask {
    pub id: String,
    pub profile_id: String,
    pub cancelled: Arc<AtomicBool>,
}
pub struct PendingInteraction {
    pub info: Interaction,
    pub response: oneshot::Sender<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Interaction {
    pub id: String,
    pub kind: String,
    pub message: String,
    pub task_id: Option<String>,
    pub options: Value,
}
impl AppState {
    pub fn interaction_list(&self) -> Vec<Interaction> {
        self.0
            .pending
            .lock()
            .map(|v| v.values().map(|p| p.info.clone()).collect())
            .unwrap_or_default()
    }
    pub fn active_id(&self) -> Option<String> {
        self.0
            .active
            .lock()
            .ok()
            .and_then(|a| a.as_ref().map(|a| a.id.clone()))
    }
}
