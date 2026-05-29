use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::RwLock;

pub const CODEX_OAUTH_AUTH_URL: &str = "https://auth.openai.com/oauth/authorize";
pub const CODEX_OAUTH_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
pub const CODEX_OAUTH_SCOPE: &str = "openid email profile offline_access";
pub const CODEX_OAUTH_SESSION_EXPIRE_MINUTES: i64 = 10;

#[derive(Debug, Clone)]
pub struct CodexOAuthSession {
    pub provider_id: String,
    pub code_verifier: String,
    pub redirect_uri: String,
    pub frontend_redirect_uri: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CodexOAuthStartRequest {
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CodexOAuthStartResponse {
    pub authorize_url: String,
    pub state: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CodexOAuthCallbackRequest {
    pub code: String,
    pub state: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CodexOAuthStatusResponse {
    pub connected: bool,
    pub needs_reauth: bool,
    pub account_count: usize,
    pub active_account_count: usize,
    #[serde(default)]
    pub accounts: Vec<CodexOAuthAccountSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token_expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CodexOAuthAccountSummary {
    pub account_key: String,
    pub enabled: bool,
    pub weight: u32,
    pub needs_reauth: bool,
    pub total_requests: u64,
    pub success_requests: u64,
    pub failed_requests: u64,
    pub qps: f64,
    pub error_rate: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token_expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CodexOAuthAccountSetting {
    pub account_key: String,
    pub enabled: bool,
    pub weight: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CodexOAuthAccountsUpdateRequest {
    pub accounts: Vec<CodexOAuthAccountSetting>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexOAuthTokenData {
    #[serde(default)]
    pub api_key: String,
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub id_token: String,
    #[serde(default)]
    pub token_type: String,
    #[serde(default)]
    pub expires_in: i64,
}

static CODEX_OAUTH_SESSIONS: OnceLock<RwLock<HashMap<String, CodexOAuthSession>>> = OnceLock::new();

pub fn codex_oauth_sessions() -> &'static RwLock<HashMap<String, CodexOAuthSession>> {
    CODEX_OAUTH_SESSIONS.get_or_init(|| RwLock::new(HashMap::new()))
}
