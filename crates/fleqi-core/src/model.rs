use crate::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Appearance {
    pub theme: String,
    pub material: String,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: "light".into(),
            material: "frosted".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct SettingsSnapshot {
    pub revision: u64,
    pub appearance: Appearance,
    pub launch_at_login: bool,
    pub bar_enabled: bool,
    pub activation: String,
    pub shortcut: String,
    pub default_profile_id: Option<String>,
    pub output_directory: Option<String>,
    pub conflict_policy: String,
    /// 任务回复气泡的自动关闭秒数；None 表示常驻显示（带关闭按钮）。
    pub bubble_seconds: Option<f64>,
}
impl Default for SettingsSnapshot {
    fn default() -> Self {
        Self {
            revision: 0,
            appearance: Appearance::default(),
            launch_at_login: false,
            bar_enabled: true,
            activation: "manual".into(),
            shortcut: "Control+Alt+Space".into(),
            default_profile_id: None,
            output_directory: None,
            conflict_policy: "rename".into(),
            bubble_seconds: Some(6.0),
        }
    }
}
impl SettingsSnapshot {
    pub fn validate(&self) -> Result<()> {
        if !["light", "dark"].contains(&self.appearance.theme.as_str())
            || !["frosted", "liquid"].contains(&self.appearance.material.as_str())
            || !["manual", "automatic"].contains(&self.activation.as_str())
            || !["rename", "ask"].contains(&self.conflict_policy.as_str())
            || self.shortcut.len() > 100
            || self.shortcut.is_empty()
        {
            return Err(Error::Invalid("设置值无效".into()));
        }
        if let Some(path) = &self.output_directory {
            if !std::path::Path::new(path).is_dir() {
                return Err(Error::Invalid("输出目录不存在".into()));
            }
        }
        if let Some(seconds) = self.bubble_seconds {
            if !seconds.is_finite() || !(1.0..=30.0).contains(&seconds) {
                return Err(Error::Invalid("气泡显示时长无效".into()));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Protocol {
    OpenaiCompletions,
    OpenaiResponses,
    AnthropicMessages,
    GoogleGenerativeAi,
    OpenaiCodexResponses,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    ApiKey,
    Oauth,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServiceTier {
    pub id: String,
    pub name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub thinking_levels: Vec<String>,
    pub default_thinking: String,
    pub service_tiers: Vec<ServiceTier>,
    pub context_window: u32,
    pub max_tokens: u32,
    pub capability_source: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelProfile {
    pub id: String,
    pub name: String,
    pub protocol: Protocol,
    pub base_url: String,
    pub model_id: String,
    pub thinking: String,
    pub auth_type: AuthType,
    #[serde(default)]
    pub has_credential: bool,
    #[serde(default)]
    pub models: Vec<ModelInfo>,
    #[serde(default)]
    pub models_fetched_at: Option<u64>,
    #[serde(default)]
    pub service_tier: Option<String>,
}
impl ModelProfile {
    pub fn validate(&self) -> Result<()> {
        if self.id.is_empty()
            || self.id.len() > 100
            || !self
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Err(Error::Invalid("连接编号无效".into()));
        }
        if self.name.trim().is_empty() || self.name.len() > 200 || self.model_id.len() > 500 {
            return Err(Error::Invalid("请填写连接名称".into()));
        }
        let url =
            url::Url::parse(&self.base_url).map_err(|_| Error::Invalid("服务地址无效".into()))?;
        if !["http", "https"].contains(&url.scheme())
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(Error::Invalid(
                "服务地址需为 HTTP(S) 地址，凭据请单独填写".into(),
            ));
        }
        if ![
            "default", "off", "minimal", "low", "medium", "high", "xhigh", "max", "ultra",
        ]
        .contains(&self.thinking.as_str())
        {
            return Err(Error::Invalid("推理强度无效".into()));
        }
        if (self.auth_type == AuthType::Oauth) != (self.protocol == Protocol::OpenaiCodexResponses)
        {
            return Err(Error::Invalid(
                "Codex 连接使用账号登录，其他连接使用 API Key 或本地无密钥模式".into(),
            ));
        }
        if self.auth_type == AuthType::Oauth && self.base_url != "https://chatgpt.com/backend-api" {
            return Err(Error::Invalid("Codex 账号只用于对应的官方服务".into()));
        }
        Ok(())
    }
    pub fn validate_selection(&self) -> Result<()> {
        if self.model_id.is_empty() {
            return Err(Error::Invalid("请先获取并选择模型".into()));
        }
        if let Some(model) = self.models.iter().find(|m| m.id == self.model_id) {
            if !model.thinking_levels.is_empty() && !model.thinking_levels.contains(&self.thinking)
            {
                return Err(Error::Invalid(
                    "当前模型不支持此思考档位，请重新选择".into(),
                ));
            }
            if self
                .service_tier
                .as_ref()
                .is_some_and(|tier| !model.service_tiers.iter().any(|t| &t.id == tier))
            {
                return Err(Error::Invalid("当前模型不支持此速度档位".into()));
            }
        } else if !self.models.is_empty() {
            return Err(Error::Invalid(
                "所选模型不在该连接的可用列表中，请刷新模型".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSnapshot {
    pub id: String,
    pub host_window: String,
    pub directory: String,
    pub files: Vec<String>,
    pub captured_at: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRequest {
    pub request_id: String,
    pub prompt: String,
    pub context: ContextSnapshot,
    pub profile: ModelProfile,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Checking,
    WaitingModel,
    AwaitingInput,
    AwaitingConfirmation,
    Running,
    Verifying,
    Completed,
    Partial,
    Failed,
    Cancelled,
    NeedsReview,
}
impl TaskStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Partial | Self::Failed | Self::Cancelled | Self::NeedsReview
        )
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Checking => "checking",
            Self::WaitingModel => "waiting_model",
            Self::AwaitingInput => "awaiting_input",
            Self::AwaitingConfirmation => "awaiting_confirmation",
            Self::Running => "running",
            Self::Verifying => "verifying",
            Self::Completed => "completed",
            Self::Partial => "partial",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::NeedsReview => "needs_review",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionRecord {
    pub id: String,
    pub operation: String,
    pub status: String,
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRecord {
    pub id: String,
    pub request_id: String,
    pub prompt: String,
    pub context: ContextSnapshot,
    pub profile: ModelProfile,
    pub status: TaskStatus,
    pub message: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub sequence: u64,
    pub completed: usize,
    pub total: Option<usize>,
    pub actions: Vec<ActionRecord>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionItem {
    pub id: String,
    pub name: String,
    pub purpose: String,
    pub status: String,
    pub detail: String,
    pub required: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCapabilities {
    pub platform: String,
    pub host_name: String,
    pub host_attachment: bool,
    pub tray: bool,
    pub credential_store: bool,
    pub detail: String,
}
pub trait HostAdapter: Send + Sync {
    fn capabilities(&self) -> PlatformCapabilities;
    fn context(&self) -> Result<ContextSnapshot>;
    fn permissions(&self) -> Result<Vec<PermissionItem>>;
}
