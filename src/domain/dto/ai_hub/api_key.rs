use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct CreateApiKeyDTO {
    pub name: String,
    pub description: Option<String>,
    pub rate_limit_enabled: bool,
    pub max_requests_per_minute: Option<i64>,
    pub max_tokens_per_minute: Option<i64>,
    pub model_rate_limits: Option<Vec<ModelRateLimit>>,
    pub status: Option<String>,
    pub quota_limit: Option<String>,
    pub quota_period: Option<String>,
    pub permissions: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ModelRateLimit {
    pub model: String,
    pub max_requests_per_minute: i64,
    pub max_tokens_per_minute: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UpdateApiKeyDTO {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub quota_limit: Option<String>,
    pub quota_period: Option<String>,
    pub enabled: Option<bool>,
    pub permissions: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ApiKeyQueryDTO {
    pub user_id: Option<String>,
    pub status: Option<String>,
    pub enabled: Option<bool>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ApiKeyResponse {
    pub id: String,
    pub key: String,
    pub prefix: String,
    pub name: Option<String>,
    pub quota_limit: Option<String>,
    pub quota_period: Option<String>,
    pub quota_used: Option<String>,
    pub quota_reset_time: Option<String>,
    pub last_used_time: Option<String>,
    pub enabled: Option<bool>,
    pub status: Option<String>,
    pub created_at: String,
    pub user_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ApiKeyValidationResult {
    pub valid: bool,
    pub api_key_id: Option<String>,
    pub user_id: Option<String>,
    pub user_level: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ListApiKeysResponse {
    pub items: Vec<ApiKeyResponse>,
    pub total: i64,
}
