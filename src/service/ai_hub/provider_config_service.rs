//! 供应商配置服务模块
//! 负责管理AI服务供应商的配置和生命周期

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use rbatis::rbdc::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};

use crate::context::ServiceContext;
use crate::domain::dto::provider::{ProviderConfig as ProviderConfigDTO, ProviderType};
use crate::domain::table::ai_hub::provider_config::ProviderConfig;
use crate::error::{Error, Result};
use crate::pool;
use crate::service::ai_hub::encryption_service::EncryptionService;
use crate::service::ai_hub::provider::oauth::{
    PROVIDER_OAUTH_SESSION_EXPIRE_MINUTES, ProviderOAuthSession, ProviderOAuthTokenData,
    get_provider_account_metrics, provider_account_needs_reauth,
    provider_accounts_from_auth_config, provider_oauth_accounts_key, provider_oauth_key,
    provider_oauth_sessions,
};
pub use crate::service::ai_hub::provider::oauth::{
    ProviderOAuthAccount, ProviderOAuthAccountImportItem, ProviderOAuthAccountSetting,
    ProviderOAuthAccountSummary, ProviderOAuthAccountsImportRequest,
    ProviderOAuthAccountsUpdateRequest, ProviderOAuthCallbackRequest, ProviderOAuthDeviceCodeData,
    ProviderOAuthDevicePollRequest, ProviderOAuthDevicePollResponse,
    ProviderOAuthDisconnectRequest, ProviderOAuthStartRequest, ProviderOAuthStartResponse,
    ProviderOAuthStatusResponse,
};
use rand::RngCore;
use std::collections::HashMap;
use std::sync::Arc;
use url::Url;

/// 供应商配置服务 Trait
#[async_trait]
pub trait ProviderConfigService: Send + Sync {
    /// 创建供应商配置
    async fn create_provider(&self, req: CreateProviderRequest) -> Result<ProviderConfig>;

    /// 获取供应商配置
    async fn get_provider(&self, id: &str) -> Result<ProviderConfig>;

    /// 通过provider_code获取供应商配置
    async fn get_provider_by_code(&self, provider_code: &str) -> Result<ProviderConfig>;

    /// 通过名称获取供应商配置
    async fn get_provider_by_name(&self, name: &str) -> Result<ProviderConfig>;

    /// 获取供应商列表（支持分页和过滤）
    async fn list_providers(&self, req: ListProvidersRequest) -> Result<ListProvidersResponse>;

    /// 更新供应商配置
    async fn update_provider(&self, id: &str, req: UpdateProviderRequest)
    -> Result<ProviderConfig>;

    /// 删除供应商配置
    async fn delete_provider(&self, id: &str) -> Result<()>;

    /// 获取解密的API密钥（带缓存）
    fn get_decrypted_api_key(&self, provider_id: &str, encrypted_key: &str) -> Result<String>;

    /// 初始化所有供应商的缓存
    async fn initialize_provider_cache(&self) -> Result<()>;

    async fn start_provider_oauth(
        &self,
        provider_id: &str,
        req: ProviderOAuthStartRequest,
    ) -> Result<ProviderOAuthStartResponse>;

    async fn complete_provider_oauth(
        &self,
        provider_id: &str,
        req: ProviderOAuthCallbackRequest,
    ) -> Result<ProviderOAuthStatusResponse>;

    async fn poll_provider_oauth_device(
        &self,
        provider_id: &str,
        req: ProviderOAuthDevicePollRequest,
    ) -> Result<ProviderOAuthDevicePollResponse>;

    async fn complete_provider_oauth_public_callback(
        &self,
        state: &str,
        code: Option<&str>,
        error: Option<&str>,
        expected_provider_type: Option<&str>,
    ) -> Result<String>;

    async fn get_provider_oauth_status(
        &self,
        provider_id: &str,
    ) -> Result<ProviderOAuthStatusResponse>;

    async fn disconnect_provider_oauth(
        &self,
        provider_id: &str,
        req: ProviderOAuthDisconnectRequest,
    ) -> Result<ProviderOAuthStatusResponse>;

    async fn update_provider_oauth_accounts(
        &self,
        provider_id: &str,
        req: ProviderOAuthAccountsUpdateRequest,
    ) -> Result<ProviderOAuthStatusResponse>;

    async fn import_provider_oauth_accounts(
        &self,
        provider_id: &str,
        req: ProviderOAuthAccountsImportRequest,
    ) -> Result<ProviderOAuthStatusResponse>;
}

#[derive(Debug, Clone)]
struct ProviderOAuthConfig {
    authorize_url: String,
    token_url: String,
    client_id: String,
    client_secret: Option<String>,
    scope: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ProviderOAuthErrorPayload {
    #[serde(default)]
    error: String,
    #[serde(default)]
    error_description: String,
}

#[derive(Debug)]
enum ProviderOAuthDevicePollOutcome {
    Pending {
        message: Option<String>,
        retry_after_seconds: i64,
    },
    Success(ProviderOAuthTokenData),
    Failed(String),
}

/// 创建供应商请求
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateProviderRequest {
    pub provider_code: String,
    pub name: String,
    pub provider_type: String,
    #[serde(default)]
    pub api_base: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_config: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circuit_breaker_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_threshold: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
}

/// 列表供应商请求
#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct ListProvidersRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i64>,
    #[serde(alias = "page_size")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyword: Option<String>,
}

/// 列表供应商响应
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProviderListItem {
    #[serde(flatten)]
    pub provider: ProviderConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_status: Option<ProviderOAuthStatusResponse>,
}

/// 列表供应商响应
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ListProvidersResponse {
    pub items: Vec<ProviderListItem>,
    pub total: i64,
    pub page: i64,
    pub size: i64,
}

/// 更新供应商请求
#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct UpdateProviderRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_base: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_config: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circuit_breaker_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_threshold: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
}

/// 供应商配置服务实现
pub struct ProviderConfigServiceImpl {
    encryption_service: EncryptionService,
    ctx: Arc<ServiceContext>,
}

impl ProviderConfigServiceImpl {
    pub fn new(encryption_service: EncryptionService, ctx: Arc<ServiceContext>) -> Self {
        Self {
            encryption_service,
            ctx,
        }
    }

    fn validate_required_text(field: &str, value: &str) -> Result<()> {
        if value.trim().is_empty() {
            return Err(Error::ValidationError(format!(
                "Missing required field: {}",
                field
            )));
        }

        Ok(())
    }

    fn default_api_base_for_provider_type(provider_type: &str) -> Option<&'static str> {
        match provider_type {
            "openai" | "codex" => Some("https://api.openai.com/v1"),
            "claude" => Some("https://api.anthropic.com/v1"),
            "gemini" | "gemini-cli" | "aistudio" => {
                Some("https://generativelanguage.googleapis.com/v1beta")
            }
            "vertex" => Some("https://us-central1-aiplatform.googleapis.com/v1"),
            "qwen" => Some("https://dashscope.aliyuncs.com/compatible-mode/v1"),
            "kimi" => Some("https://api.kimi.com/coding/v1"),
            "iflow" => Some("https://api.iflow.ai/v1"),
            "antigravity" => Some("https://api.antigravity.ai/v1"),
            _ => None,
        }
    }

    fn provider_type_requires_api_base(provider_type: &str) -> bool {
        matches!(provider_type, "openai-compatible" | "custom")
    }

    fn validate_and_normalize_provider_type(provider_type: &str) -> Result<String> {
        ProviderType::parse(provider_type)
            .map(|t| t.to_string())
            .ok_or_else(|| {
                Error::ValidationError(format!("Invalid provider type: {}", provider_type.trim()))
            })
    }

    #[cfg(test)]
    fn validate_and_normalize_optional_provider_type(
        provider_type: Option<&str>,
    ) -> Result<Option<String>> {
        provider_type
            .map(Self::validate_and_normalize_provider_type)
            .transpose()
    }

    fn normalize_optional_provider_type_for_query(
        provider_type: Option<&str>,
    ) -> Result<Option<String>> {
        provider_type
            .map(|provider_type| {
                let trimmed = provider_type.trim();
                if trimmed.is_empty() {
                    return Err(Error::ValidationError(format!(
                        "Invalid provider type: {}",
                        trimmed
                    )));
                }

                Ok(Self::normalize_provider_type_for_query_compatible(
                    provider_type,
                ))
            })
            .transpose()
    }

    fn normalize_provider_type_for_query_compatible(provider_type: &str) -> String {
        ProviderType::parse(provider_type)
            .map(|t| t.to_string())
            .unwrap_or_else(|| provider_type.trim().to_lowercase())
    }

    fn normalize_provider_type_for_comparison(provider_type: &str) -> Option<String> {
        ProviderType::parse(provider_type).map(|t| t.to_string())
    }

    fn has_valid_provider_type(provider: &ProviderConfig) -> bool {
        Self::normalize_provider_type_for_comparison(&provider.provider_type).is_some()
    }

    fn listable_provider_types() -> &'static [&'static str] {
        &[
            "aistudio",
            "antigravity",
            "claude",
            "codex",
            "custom",
            "gemini",
            "gemini-cli",
            "iflow",
            "kimi",
            "openai",
            "openai-compatible",
            "qwen",
            "vertex",
        ]
    }

    fn provider_supports_oauth_summary(provider_type: &str) -> bool {
        matches!(
            provider_type,
            "aistudio"
                | "antigravity"
                | "claude"
                | "codex"
                | "custom"
                | "gemini"
                | "gemini-cli"
                | "iflow"
                | "kimi"
                | "openai"
                | "openai-compatible"
                | "qwen"
                | "vertex"
        )
    }

    fn parse_query_count(value: &rbs::Value) -> i64 {
        value
            .as_array()
            .and_then(|rows| rows.first())
            .and_then(|row| serde_json::to_value(row).ok())
            .and_then(|row| {
                row.get("count")
                    .or_else(|| row.get("COUNT(*)"))
                    .or_else(|| row.get("COUNT"))
                    .cloned()
            })
            .and_then(|count| {
                count
                    .as_i64()
                    .or_else(|| count.as_str().and_then(|raw| raw.parse::<i64>().ok()))
            })
            .unwrap_or(0)
    }

    fn parse_query_providers(value: &rbs::Value) -> Vec<ProviderConfig> {
        value
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|item| {
                        rbs::from_value::<ProviderConfig>(item.clone())
                            .unwrap_or_else(|_| ProviderConfig::default())
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn now_ts() -> i64 {
        Utc::now().timestamp()
    }

    fn now_rfc3339() -> String {
        Utc::now().to_rfc3339()
    }

    fn ts_to_rfc3339(ts: i64) -> String {
        chrono::DateTime::from_timestamp(ts, 0)
            .map(|dt| dt.with_timezone(&Utc).to_rfc3339())
            .unwrap_or_else(Self::now_rfc3339)
    }

    fn generate_url_safe_random(bytes_len: usize) -> String {
        let mut bytes = vec![0_u8; bytes_len];
        rand::rng().fill_bytes(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    }

    fn generate_pkce_pair() -> (String, String) {
        let code_verifier = Self::generate_url_safe_random(96);
        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let code_challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
        (code_verifier, code_challenge)
    }

    fn get_provider_oauth_callback_url(&self, provider_type: &str) -> Result<String> {
        let server_url = self.ctx.config.server_url.trim();
        if server_url.is_empty() {
            return Err(Error::ValidationError(
                "server_url cannot be empty".to_string(),
            ));
        }

        let mut url = Url::parse(server_url)
            .map_err(|e| Error::ValidationError(format!("invalid server_url: {}", e)))?;

        if url.host_str() == Some("0.0.0.0") {
            url.set_host(Some("127.0.0.1"))
                .map_err(|_| Error::ValidationError("invalid server_url host".to_string()))?;
        }

        let provider_type = provider_type.trim().to_lowercase();
        if provider_type.is_empty() {
            return Err(Error::ValidationError(
                "provider_type cannot be empty".to_string(),
            ));
        }

        url.set_path(&format!(
            "/public/providers/{}/oauth/callback",
            provider_type
        ));
        url.set_query(None);
        url.set_fragment(None);

        Ok(url.to_string())
    }

    async fn update_provider_auth_config(
        &self,
        provider_id: &str,
        auth_config: JsonValue,
    ) -> Result<ProviderConfig> {
        self.update_provider(
            provider_id,
            UpdateProviderRequest {
                auth_config: Some(auth_config),
                ..UpdateProviderRequest::default()
            },
        )
        .await
    }

    fn validate_provider_oauth_target(provider: &ProviderConfig) -> Result<String> {
        let provider_type = Self::validate_and_normalize_provider_type(&provider.provider_type)?;
        match provider_type.as_str() {
            "openai" | "openai-compatible" | "custom" | "claude" | "gemini" | "gemini-cli"
            | "vertex" | "aistudio" | "qwen" | "kimi" | "iflow" | "antigravity" => {
                Ok(provider_type)
            }
            "codex" => Ok(provider_type),
            _ => Err(Error::ValidationError(format!(
                "provider_type {} does not support OAuth in provider scope",
                provider_type
            ))),
        }
    }

    fn provider_oauth_key(provider_type: &str) -> String {
        provider_oauth_key(provider_type)
    }

    fn default_provider_oauth_config(provider_type: &str) -> Option<ProviderOAuthConfig> {
        match provider_type {
            "openai" | "openai-compatible" | "custom" | "codex" => Some(ProviderOAuthConfig {
                authorize_url: "https://auth.openai.com/oauth/authorize".to_string(),
                token_url: "https://auth.openai.com/oauth/token".to_string(),
                client_id: "app_EMoamEEZ73f0CkXaXp7hrann".to_string(),
                client_secret: None,
                scope: Some("openid email profile offline_access".to_string()),
            }),
            "claude" => Some(ProviderOAuthConfig {
                authorize_url: "https://claude.ai/oauth/authorize".to_string(),
                token_url: "https://console.anthropic.com/v1/oauth/token".to_string(),
                client_id: "9d1c250a-e61b-44d9-88ed-5944d1962f5e".to_string(),
                client_secret: None,
                scope: Some("org:create_api_key user:profile user:inference".to_string()),
            }),
            "gemini" | "gemini-cli" | "vertex" | "aistudio" => Some(ProviderOAuthConfig {
                authorize_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
                token_url: "https://oauth2.googleapis.com/token".to_string(),
                client_id: String::new(),
                client_secret: None,
                scope: Some(
                    "https://www.googleapis.com/auth/cloud-platform https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile".to_string(),
                ),
            }),
            "iflow" => Some(ProviderOAuthConfig {
                authorize_url: "https://iflow.cn/oauth".to_string(),
                token_url: "https://iflow.cn/oauth/token".to_string(),
                client_id: "10009311001".to_string(),
                client_secret: None,
                scope: None,
            }),
            "qwen" => Some(ProviderOAuthConfig {
                authorize_url: "https://chat.qwen.ai/api/v1/oauth2/device/code".to_string(),
                token_url: "https://chat.qwen.ai/api/v1/oauth2/token".to_string(),
                client_id: "f0304373b74a44d2b584a3fb70ca9e56".to_string(),
                client_secret: None,
                scope: Some("openid profile email model.completion".to_string()),
            }),
            "kimi" => Some(ProviderOAuthConfig {
                authorize_url: "https://auth.kimi.com/api/oauth/device_authorization".to_string(),
                token_url: "https://auth.kimi.com/api/oauth/token".to_string(),
                client_id: "17e5f671-d194-4dfb-9706-5516cb48c098".to_string(),
                client_secret: None,
                scope: None,
            }),
            "antigravity" => Some(ProviderOAuthConfig {
                authorize_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
                token_url: "https://oauth2.googleapis.com/token".to_string(),
                client_id: String::new(),
                client_secret: None,
                scope: Some(
                    "https://www.googleapis.com/auth/cloud-platform https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile https://www.googleapis.com/auth/cclog https://www.googleapis.com/auth/experimentsandconfigs".to_string(),
                ),
            }),
            _ => None,
        }
    }

    fn parse_provider_oauth_config(
        provider_type: &str,
        auth_config: &Option<JsonValue>,
    ) -> Result<ProviderOAuthConfig> {
        let oauth_obj = auth_config
            .as_ref()
            .and_then(|v| v.as_object())
            .and_then(|obj| obj.get("oauth"))
            .and_then(|v| v.as_object());

        let defaults = Self::default_provider_oauth_config(provider_type);

        let field_from_request = |key: &str| {
            oauth_obj
                .and_then(|obj| obj.get(key))
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(ToString::to_string)
        };

        let authorize_url = field_from_request("authorize_url")
            .or_else(|| defaults.as_ref().map(|cfg| cfg.authorize_url.clone()))
            .ok_or_else(|| {
                Error::ValidationError(format!(
                    "OAuth authorize_url is required for {} (no backend default found)",
                    provider_type
                ))
            })?;

        let token_url = field_from_request("token_url")
            .or_else(|| defaults.as_ref().map(|cfg| cfg.token_url.clone()))
            .ok_or_else(|| {
                Error::ValidationError(format!(
                    "OAuth token_url is required for {} (no backend default found)",
                    provider_type
                ))
            })?;

        let client_id = field_from_request("client_id")
            .or_else(|| defaults.as_ref().map(|cfg| cfg.client_id.clone()))
            .ok_or_else(|| {
                Error::ValidationError(format!(
                    "OAuth client_id is required for {} (no backend default found)",
                    provider_type
                ))
            })?;

        let client_secret = field_from_request("client_secret")
            .or_else(|| defaults.as_ref().and_then(|cfg| cfg.client_secret.clone()));

        let scope = field_from_request("scope")
            .or_else(|| defaults.as_ref().and_then(|cfg| cfg.scope.clone()));

        Ok(ProviderOAuthConfig {
            authorize_url,
            token_url,
            client_id,
            client_secret,
            scope,
        })
    }

    fn is_device_code_provider(provider_type: &str) -> bool {
        matches!(provider_type, "qwen" | "kimi")
    }

    async fn persist_provider_oauth_token(
        &self,
        provider: &ProviderConfig,
        provider_id: &str,
        provider_type: &str,
        oauth_cfg: &ProviderOAuthConfig,
        token_data: ProviderOAuthTokenData,
    ) -> Result<ProviderOAuthStatusResponse> {
        let expires_at = if token_data.expires_in > 0 {
            Some((Self::now_ts() + token_data.expires_in).to_string())
        } else {
            None
        };

        let mut auth_obj = provider
            .auth_config
            .clone()
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();

        let mut hasher = Sha256::new();
        if !token_data.id_token.trim().is_empty() {
            hasher.update(token_data.id_token.as_bytes());
        } else {
            hasher.update(token_data.access_token.as_bytes());
        }
        let digest = format!("{:x}", hasher.finalize());
        let account_key = format!("oauth-{}", &digest[..12]);

        let mut accounts = provider_accounts_from_auth_config(&provider.auth_config, provider_type);
        let now_rfc3339 = Self::now_rfc3339();

        if let Some(existing) = accounts.iter_mut().find(|a| a.account_key == account_key) {
            existing.access_token = token_data.access_token.clone();
            existing.refresh_token = token_data.refresh_token.clone();
            existing.id_token = token_data.id_token.clone();
            existing.token_type = token_data.token_type.clone();
            existing.account_id = oauth_cfg.client_id.clone();
            existing.access_token_expires_at = expires_at.clone().unwrap_or_default();
            existing.updated_at = now_rfc3339.clone();
            existing.enabled = true;
            if existing.weight == 0 {
                existing.weight = 1;
            }
        } else {
            accounts.push(ProviderOAuthAccount {
                account_key: account_key.clone(),
                access_token: token_data.access_token.clone(),
                refresh_token: token_data.refresh_token.clone(),
                id_token: token_data.id_token.clone(),
                token_type: if token_data.token_type.trim().is_empty() {
                    "Bearer".to_string()
                } else {
                    token_data.token_type.clone()
                },
                email: String::new(),
                account_id: oauth_cfg.client_id.clone(),
                access_token_expires_at: expires_at.clone().unwrap_or_default(),
                updated_at: now_rfc3339.clone(),
                enabled: true,
                weight: 1,
            });
        }

        let selected_account = accounts
            .iter()
            .find(|a| a.account_key == account_key)
            .cloned()
            .or_else(|| accounts.first().cloned())
            .ok_or_else(|| Error::BusinessError("No OAuth account available".to_string()))?;

        auth_obj.insert(
            provider_oauth_accounts_key(provider_type),
            serde_json::to_value(&accounts).map_err(|e| {
                Error::BusinessError(format!("Serialize OAuth accounts failed: {}", e))
            })?,
        );
        auth_obj.insert(
            Self::provider_oauth_key(provider_type),
            serde_json::to_value(selected_account).map_err(|e| {
                Error::BusinessError(format!("Serialize OAuth account failed: {}", e))
            })?,
        );

        let updated_provider = self
            .update_provider_auth_config(provider_id, JsonValue::Object(auth_obj))
            .await?;
        Ok(Self::build_provider_oauth_status(
            provider_type,
            &updated_provider,
        ))
    }

    fn parse_device_code_response(body: JsonValue) -> Result<ProviderOAuthDeviceCodeData> {
        let get_text = |keys: &[&str]| -> String {
            for key in keys {
                if let Some(value) = body.get(*key).and_then(|v| v.as_str()) {
                    let trimmed = value.trim();
                    if !trimmed.is_empty() {
                        return trimmed.to_string();
                    }
                }
            }
            String::new()
        };

        let get_i64 = |keys: &[&str], default: i64| -> i64 {
            for key in keys {
                if let Some(value) = body.get(*key).and_then(|v| v.as_i64()) {
                    return value;
                }
            }
            default
        };

        let data = ProviderOAuthDeviceCodeData {
            device_code: get_text(&["device_code"]),
            user_code: get_text(&["user_code", "code"]),
            verification_uri: get_text(&["verification_uri", "verification_url", "verify_url"]),
            verification_uri_complete: get_text(&[
                "verification_uri_complete",
                "verification_url_complete",
            ]),
            expires_in: get_i64(&["expires_in"], 600),
            interval: get_i64(&["interval"], 5),
        };

        if data.device_code.trim().is_empty() {
            return Err(Error::BusinessError(
                "OAuth device code response missing device_code".to_string(),
            ));
        }

        Ok(data)
    }

    async fn request_provider_device_code(
        provider_type: &str,
        oauth_cfg: &ProviderOAuthConfig,
        code_challenge: &str,
        device_id: &str,
    ) -> Result<ProviderOAuthDeviceCodeData> {
        let mut form = vec![("client_id", oauth_cfg.client_id.clone())];
        if let Some(scope) = oauth_cfg.scope.clone()
            && !scope.trim().is_empty()
        {
            form.push(("scope", scope));
        }

        if provider_type == "qwen" {
            form.push(("code_challenge", code_challenge.to_string()));
            form.push(("code_challenge_method", "S256".to_string()));
        }

        let mut req = reqwest::Client::new()
            .post(&oauth_cfg.authorize_url)
            .form(&form);
        if provider_type == "kimi" {
            req = req
                .header("X-Msh-Platform", "cli-proxy-api")
                .header("X-Msh-Version", "1.0.0")
                .header(
                    "X-Msh-Device-Name",
                    format!("rsllm-{}", std::env::consts::OS),
                )
                .header(
                    "X-Msh-Device-Model",
                    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
                )
                .header("X-Msh-Device-Id", device_id);
        }

        let resp = req.send().await.map_err(|e| {
            Error::BusinessError(format!("OAuth device authorization request failed: {}", e))
        })?;
        let status = resp.status();
        let body: JsonValue = resp.json().await.map_err(|e| {
            Error::BusinessError(format!("OAuth device response parse failed: {}", e))
        })?;

        if !status.is_success() {
            return Err(Error::BusinessError(format!(
                "OAuth device authorization failed: status={} body={}",
                status, body
            )));
        }

        Self::parse_device_code_response(body)
    }

    fn classify_device_poll_error(
        err: &ProviderOAuthErrorPayload,
        default_retry_after_seconds: i64,
    ) -> ProviderOAuthDevicePollOutcome {
        let code = err.error.trim().to_lowercase();
        let desc = err.error_description.trim();
        let message = if desc.is_empty() {
            None
        } else {
            Some(desc.to_string())
        };

        match code.as_str() {
            "authorization_pending" => ProviderOAuthDevicePollOutcome::Pending {
                message,
                retry_after_seconds: default_retry_after_seconds.max(1),
            },
            "slow_down" => ProviderOAuthDevicePollOutcome::Pending {
                message,
                retry_after_seconds: (default_retry_after_seconds + 5).max(2),
            },
            "expired_token" | "access_denied" | "invalid_grant" => {
                ProviderOAuthDevicePollOutcome::Failed(
                    message.unwrap_or_else(|| format!("OAuth 设备码授权失败: {}", code)),
                )
            }
            _ => ProviderOAuthDevicePollOutcome::Failed(
                message.unwrap_or_else(|| format!("OAuth token exchange failed: {}", code)),
            ),
        }
    }

    async fn poll_provider_device_token(
        provider_type: &str,
        oauth_cfg: &ProviderOAuthConfig,
        session: &ProviderOAuthSession,
        provider_auth_config: &Option<JsonValue>,
    ) -> Result<ProviderOAuthDevicePollOutcome> {
        let mut form = vec![
            (
                "grant_type",
                "urn:ietf:params:oauth:grant-type:device_code".to_string(),
            ),
            ("client_id", oauth_cfg.client_id.clone()),
            ("device_code", session.device_code.clone()),
        ];

        if provider_type == "qwen" {
            form.push(("code_verifier", session.code_verifier.clone()));
        }
        if let Some(secret) = oauth_cfg.client_secret.clone()
            && !secret.trim().is_empty()
        {
            form.push(("client_secret", secret));
        }

        let mut req = reqwest::Client::new()
            .post(&oauth_cfg.token_url)
            .form(&form);

        // 支持通过 auth_config.injection.basic_username/basic_password 以 Basic 方式提交 client credentials
        if let Some(inj) = provider_auth_config
            .as_ref()
            .and_then(|cfg| cfg.get("injection"))
        {
            let username = inj
                .get("basic_username")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|v| !v.is_empty());
            let password = inj
                .get("basic_password")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|v| !v.is_empty());

            if let (Some(u), Some(p)) = (username, password) {
                req = req.basic_auth(u, Some(p));
            }
        }

        if provider_type == "kimi" {
            req = req
                .header("X-Msh-Platform", "cli-proxy-api")
                .header("X-Msh-Version", "1.0.0")
                .header(
                    "X-Msh-Device-Name",
                    format!("rsllm-{}", std::env::consts::OS),
                )
                .header(
                    "X-Msh-Device-Model",
                    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
                )
                .header("X-Msh-Device-Id", &session.device_id);
        }

        let resp = req.send().await.map_err(|e| {
            Error::BusinessError(format!("OAuth device token request failed: {}", e))
        })?;

        let status = resp.status();
        let body: JsonValue = resp.json().await.map_err(|e| {
            Error::BusinessError(format!("OAuth token response parse failed: {}", e))
        })?;

        if let Ok(err) = serde_json::from_value::<ProviderOAuthErrorPayload>(body.clone())
            && !err.error.trim().is_empty()
        {
            return Ok(Self::classify_device_poll_error(
                &err,
                session.poll_interval_seconds,
            ));
        }

        if !status.is_success() {
            return Ok(ProviderOAuthDevicePollOutcome::Failed(format!(
                "OAuth token exchange failed: status={} body={}",
                status, body
            )));
        }

        let token_data: ProviderOAuthTokenData = serde_json::from_value(body)
            .map_err(|e| Error::BusinessError(format!("OAuth token parse failed: {}", e)))?;
        Ok(ProviderOAuthDevicePollOutcome::Success(token_data))
    }

    fn build_provider_oauth_status(
        provider_type: &str,
        provider: &ProviderConfig,
    ) -> ProviderOAuthStatusResponse {
        let provider_id = provider.id.clone().unwrap_or_default();
        let accounts = provider_accounts_from_auth_config(&provider.auth_config, provider_type);

        let account_summaries: Vec<ProviderOAuthAccountSummary> = accounts
            .iter()
            .map(|account| {
                let metrics =
                    get_provider_account_metrics(&provider_id, provider_type, &account.account_key);
                ProviderOAuthAccountSummary {
                    account_key: account.account_key.clone(),
                    enabled: account.enabled,
                    weight: account.weight,
                    needs_reauth: provider_account_needs_reauth(account),
                    total_requests: metrics.total_requests,
                    success_requests: metrics.success_requests,
                    failed_requests: metrics.failed_requests,
                    qps: metrics.qps,
                    error_rate: metrics.error_rate,

                    total_input_tokens: Some(metrics.total_input_tokens),
                    total_output_tokens: Some(metrics.total_output_tokens),
                    total_tokens: Some(metrics.total_tokens),
                    last_minute_input_tokens: Some(metrics.last_minute_input_tokens),
                    last_minute_output_tokens: Some(metrics.last_minute_output_tokens),
                    last_minute_total_tokens: Some(metrics.last_minute_total_tokens),

                    email: (!account.email.trim().is_empty()).then(|| account.email.clone()),
                    account_id: (!account.account_id.trim().is_empty())
                        .then(|| account.account_id.clone()),
                    access_token_expires_at: (!account.access_token_expires_at.trim().is_empty())
                        .then(|| account.access_token_expires_at.clone()),
                    updated_at: (!account.updated_at.trim().is_empty())
                        .then(|| account.updated_at.clone()),
                }
            })
            .collect();

        let active_account_count = accounts
            .iter()
            .filter(|a| {
                a.enabled && !a.access_token.trim().is_empty() && !provider_account_needs_reauth(a)
            })
            .count() as u64;
        let connected = active_account_count > 0;
        let account_count = accounts.len() as u64;

        let primary = accounts
            .iter()
            .find(|a| {
                a.enabled && !a.access_token.trim().is_empty() && !provider_account_needs_reauth(a)
            })
            .or_else(|| accounts.first());

        ProviderOAuthStatusResponse {
            provider_type: provider_type.to_string(),
            connected,
            needs_reauth: account_count == 0 || active_account_count == 0,
            token_type: primary.map(|a| a.token_type.clone()),
            email: primary.and_then(|a| (!a.email.trim().is_empty()).then(|| a.email.clone())),
            account_id: primary
                .and_then(|a| (!a.account_id.trim().is_empty()).then(|| a.account_id.clone())),
            access_token_expires_at: primary.and_then(|a| {
                (!a.access_token_expires_at.trim().is_empty())
                    .then(|| a.access_token_expires_at.clone())
            }),
            updated_at: primary
                .and_then(|a| (!a.updated_at.trim().is_empty()).then(|| a.updated_at.clone())),
            account_count: Some(account_count),
            active_account_count: Some(active_account_count),
            accounts: Some(account_summaries),
        }
    }

    fn build_provider_frontend_redirect_url(
        frontend_redirect_uri: &str,
        provider_id: &str,
        provider_type: &str,
        success: bool,
        message: Option<&str>,
    ) -> String {
        if let Ok(mut url) = Url::parse(frontend_redirect_uri) {
            url.query_pairs_mut()
                .append_pair("provider_oauth", "1")
                .append_pair("provider_id", provider_id)
                .append_pair("provider_type", provider_type)
                .append_pair("result", if success { "success" } else { "error" });
            if let Some(msg) = message
                && !msg.trim().is_empty()
            {
                url.query_pairs_mut().append_pair("message", msg);
            }
            return url.to_string();
        }

        let separator = if frontend_redirect_uri.contains('?') {
            '&'
        } else {
            '?'
        };
        let mut redirect_url = format!(
            "{}{}provider_oauth=1&provider_id={}&provider_type={}&result={}",
            frontend_redirect_uri,
            separator,
            provider_id,
            provider_type,
            if success { "success" } else { "error" }
        );
        if let Some(msg) = message
            && !msg.trim().is_empty()
        {
            redirect_url.push_str("&message=");
            redirect_url.push_str(msg);
        }
        redirect_url
    }
}

#[async_trait]
impl ProviderConfigService for ProviderConfigServiceImpl {
    async fn create_provider(&self, req: CreateProviderRequest) -> Result<ProviderConfig> {
        // 验证必填字段
        Self::validate_required_text("provider_code", &req.provider_code)?;
        if req.name.is_empty() {
            return Err(Error::ValidationError(
                "Provider name cannot be empty".to_string(),
            ));
        }
        if req.provider_type.is_empty() {
            return Err(Error::ValidationError(
                "Provider type cannot be empty".to_string(),
            ));
        }

        let normalized_provider_type =
            Self::validate_and_normalize_provider_type(&req.provider_type)?;

        let api_base = req
            .api_base
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .or_else(|| {
                Self::default_api_base_for_provider_type(&normalized_provider_type)
                    .map(ToString::to_string)
            });

        if api_base.is_none() {
            return Err(Error::ValidationError(
                "API base cannot be empty".to_string(),
            ));
        }

        // 检查供应商名称是否已存在
        match self.get_provider_by_name(&req.name).await {
            Ok(_) => {
                return Err(Error::ValidationError(format!(
                    "Provider name '{}' already exists",
                    req.name
                )));
            }
            Err(Error::NotFound(_)) => {}
            Err(e) => return Err(e),
        }

        // 检查供应商代码是否已存在
        match self.get_provider_by_code(&req.provider_code).await {
            Ok(_) => {
                return Err(Error::ValidationError(format!(
                    "Provider code '{}' already exists",
                    req.provider_code
                )));
            }
            Err(Error::NotFound(_)) => {}
            Err(e) => return Err(e),
        }

        // 保存原始API密钥用于注册到ProviderRegistry
        let api_key_decrypted = req.api_key.clone();

        // 加密API密钥（如果提供）
        let api_key_encrypted = if let Some(api_key) = req.api_key {
            log::info!(
                "[ProviderConfigService] Encrypting API key for new provider {} (length: {})",
                req.provider_code,
                api_key.len()
            );
            let encrypted = self.encryption_service.encrypt(&api_key)?;
            log::info!(
                "[ProviderConfigService] API key encrypted successfully (encrypted length: {})",
                encrypted.len()
            );
            Some(encrypted)
        } else {
            None
        };

        let provider = ProviderConfig {
            id: Some(ulid::Ulid::new().to_string()),
            provider_code: req.provider_code,
            name: req.name,
            provider_type: normalized_provider_type,
            api_base: api_base.unwrap_or_default(),
            api_key_encrypted,
            auth_config: req.auth_config,
            circuit_breaker_enabled: req.circuit_breaker_enabled,
            failure_threshold: req.failure_threshold,
            status: req.status,
            description: req.description,
            documentation_url: req.documentation_url,
            created_at: Some(DateTime::now().to_string()),
            updated_at: Some(DateTime::now().to_string()),
        };

        // 保存到数据库
        let result = ProviderConfig::insert(pool!(), &provider).await;
        match result {
            Ok(_) => {
                // 注册到 ProviderRegistry
                let mut registry = self.ctx.provider_registry.write().await;
                match ProviderConfigDTO::try_from(&provider) {
                    Ok(provider_dto) => {
                        if let Err(e) = registry.register_provider(provider_dto, api_key_decrypted)
                        {
                            log::error!("[rsllm] Failed to register provider to registry: {}", e);
                        }
                    }
                    Err(e) => {
                        log::error!(
                            "[rsllm] Failed to convert provider to DTO when registering: {}",
                            e
                        );
                    }
                }
                Ok(provider)
            }
            Err(e) => Err(Error::DatabaseError(e.to_string())),
        }
    }

    async fn get_provider(&self, id: &str) -> Result<ProviderConfig> {
        let sql = "SELECT * FROM provider_config WHERE id = ? LIMIT 1";
        let result = rbatis::RBatis::query(pool!(), sql, vec![rbs::Value::String(id.to_string())])
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;

        let value = result
            .as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| Error::NotFound(format!("Provider with id '{}' not found", id)))?;

        let json_value = serde_json::to_value(value).unwrap_or_default();
        serde_json::from_value(json_value).map_err(|e| Error::DatabaseError(e.to_string()))
    }

    async fn get_provider_by_code(&self, provider_code: &str) -> Result<ProviderConfig> {
        let provider = ProviderConfig::select_by_provider_code(pool!(), provider_code)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?
            .ok_or_else(|| {
                Error::NotFound(format!("Provider with code '{}' not found", provider_code))
            })?;
        Ok(provider)
    }

    async fn get_provider_by_name(&self, name: &str) -> Result<ProviderConfig> {
        let sql = "SELECT * FROM provider_config WHERE name = ? LIMIT 1";
        let result =
            rbatis::RBatis::query(pool!(), sql, vec![rbs::Value::String(name.to_string())])
                .await
                .map_err(|e| Error::DatabaseError(e.to_string()))?;

        let value = result
            .as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| Error::NotFound(format!("Provider with name '{}' not found", name)))?;

        let json_value = serde_json::to_value(value).unwrap_or_default();
        serde_json::from_value(json_value).map_err(|e| Error::DatabaseError(e.to_string()))
    }

    async fn list_providers(&self, req: ListProvidersRequest) -> Result<ListProvidersResponse> {
        let page = req.page.unwrap_or(1).max(1);
        let size = req.size.unwrap_or(20).max(1);
        let offset = (page - 1) * size;
        let normalized_provider_type =
            Self::normalize_optional_provider_type_for_query(req.provider_type.as_deref())?;
        let mut where_clauses = Vec::new();
        let mut params = Vec::new();

        if let Some(provider_type) = &normalized_provider_type {
            where_clauses.push("LOWER(TRIM(provider_type)) = ?".to_string());
            params.push(rbs::value!(provider_type));
        } else {
            let placeholders = vec!["?"; Self::listable_provider_types().len()].join(", ");
            where_clauses.push(format!("LOWER(TRIM(provider_type)) IN ({})", placeholders));
            for provider_type in Self::listable_provider_types() {
                params.push(rbs::value!(provider_type));
            }
        }

        if let Some(status) = &req.status {
            where_clauses.push("status = ?".to_string());
            params.push(rbs::value!(status));
        }

        if let Some(keyword) = &req.keyword {
            let keyword = keyword.trim().to_lowercase();
            if !keyword.is_empty() {
                where_clauses.push(
                    "(LOWER(name) LIKE ? OR LOWER(COALESCE(description, '')) LIKE ?)".to_string(),
                );
                let fuzzy_keyword = format!("%{}%", keyword);
                params.push(rbs::value!(fuzzy_keyword.clone()));
                params.push(rbs::value!(fuzzy_keyword));
            }
        }

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_clauses.join(" AND "))
        };

        let count_sql = format!("SELECT COUNT(*) AS count FROM provider_config{}", where_sql);
        let total_raw = pool!()
            .query(&count_sql, params.clone())
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;
        let total = Self::parse_query_count(&total_raw);

        let mut paged_params = params;
        paged_params.push(rbs::value!(size));
        paged_params.push(rbs::value!(offset));

        let list_sql = format!(
            "SELECT * FROM provider_config{} ORDER BY COALESCE(updated_at, created_at) DESC LIMIT ? OFFSET ?",
            where_sql
        );
        let rows = pool!()
            .query(&list_sql, paged_params)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;

        let items = Self::parse_query_providers(&rows)
            .into_iter()
            .map(|provider| {
                let provider_type =
                    Self::normalize_provider_type_for_query_compatible(&provider.provider_type);
                let oauth_status = Self::provider_supports_oauth_summary(&provider_type)
                    .then(|| Self::build_provider_oauth_status(&provider_type, &provider));
                ProviderListItem {
                    provider,
                    oauth_status,
                }
            })
            .collect();

        Ok(ListProvidersResponse {
            items,
            total,
            page,
            size,
        })
    }

    async fn update_provider(
        &self,
        id: &str,
        req: UpdateProviderRequest,
    ) -> Result<ProviderConfig> {
        // 获取现有供应商
        let mut provider = self.get_provider(id).await?;

        // 保存原始API密钥用于更新ProviderRegistry
        let api_key_decrypted = req.api_key.clone();

        // 更新字段
        if let Some(provider_code) = req.provider_code {
            Self::validate_required_text("provider_code", &provider_code)?;
            let provider_code = provider_code.trim().to_string();

            // 验证新的provider_code不与其他供应商冲突
            if provider_code != provider.provider_code {
                match self.get_provider_by_code(&provider_code).await {
                    Ok(_) => {
                        return Err(Error::ValidationError(format!(
                            "Provider code '{}' already exists",
                            provider_code
                        )));
                    }
                    Err(Error::NotFound(_)) => {}
                    Err(e) => return Err(e),
                }
                provider.provider_code = provider_code;
            }
        }
        if let Some(name) = req.name {
            provider.name = name;
        }
        if let Some(provider_type) = req.provider_type {
            provider.provider_type = Self::validate_and_normalize_provider_type(&provider_type)?;
        }
        if let Some(api_base) = req.api_base {
            provider.api_base = api_base.trim().to_string();
        }

        if Self::provider_type_requires_api_base(&provider.provider_type)
            && provider.api_base.trim().is_empty()
        {
            return Err(Error::ValidationError(
                "API base cannot be empty".to_string(),
            ));
        }
        if let Some(api_key) = req.api_key {
            log::info!(
                "[ProviderConfigService] Encrypting API key for provider {} (length: {})",
                provider.provider_code,
                api_key.len()
            );
            let encrypted = self.encryption_service.encrypt(&api_key)?;
            log::info!(
                "[ProviderConfigService] API key encrypted successfully (encrypted length: {})",
                encrypted.len()
            );
            provider.api_key_encrypted = Some(encrypted);
        }
        if let Some(auth_config) = req.auth_config {
            provider.auth_config = Some(auth_config);
        }
        if let Some(circuit_breaker_enabled) = req.circuit_breaker_enabled {
            provider.circuit_breaker_enabled = Some(circuit_breaker_enabled);
        }
        if let Some(failure_threshold) = req.failure_threshold {
            provider.failure_threshold = Some(failure_threshold);
        }
        if let Some(status) = req.status {
            provider.status = Some(status);
        }
        if let Some(description) = req.description {
            provider.description = Some(description);
        }
        if let Some(documentation_url) = req.documentation_url {
            provider.documentation_url = Some(documentation_url);
        }

        provider.updated_at = Some(DateTime::now().to_string());

        // 更新数据库
        let sql = "UPDATE provider_config SET provider_code = ?, name = ?, provider_type = ?, api_base = ?, api_key_encrypted = ?, auth_config = ?, circuit_breaker_enabled = ?, failure_threshold = ?, status = ?, description = ?, documentation_url = ?, updated_at = ? WHERE id = ?";
        rbatis::RBatis::exec(
            pool!(),
            sql,
            vec![
                rbs::Value::String(provider.provider_code.clone()),
                rbs::Value::String(provider.name.clone()),
                rbs::Value::String(provider.provider_type.clone()),
                rbs::Value::String(provider.api_base.clone()),
                rbs::Value::String(provider.api_key_encrypted.clone().unwrap_or_default()),
                rbs::Value::String(provider.auth_config.clone().unwrap_or_default().to_string()),
                rbs::Value::Bool(provider.circuit_breaker_enabled.unwrap_or_default()),
                rbs::Value::I32(provider.failure_threshold.unwrap_or_default()),
                rbs::Value::String(provider.status.clone().unwrap_or_default()),
                rbs::Value::String(provider.description.clone().unwrap_or_default()),
                rbs::Value::String(provider.documentation_url.clone().unwrap_or_default()),
                rbs::Value::String(provider.updated_at.clone().unwrap_or_default().to_string()),
                rbs::Value::String(provider.id.clone().unwrap_or_default()),
            ],
        )
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

        // 更新 ProviderRegistry
        let mut registry = self.ctx.provider_registry.write().await;
        match ProviderConfigDTO::try_from(&provider) {
            Ok(provider_dto) => {
                if let Err(e) = registry.update_provider(provider_dto, api_key_decrypted) {
                    log::error!("[rsllm] Failed to update provider in registry: {}", e);
                }
            }
            Err(e) => {
                log::error!(
                    "[rsllm] Failed to convert provider to DTO when updating registry: {}",
                    e
                );
            }
        }

        Ok(provider)
    }

    async fn delete_provider(&self, id: &str) -> Result<()> {
        let provider = self.get_provider(id).await?;
        let provider_id = provider
            .id
            .ok_or_else(|| Error::BusinessError("Provider ID is missing".to_string()))?;
        let provider_code = provider.provider_code.clone();

        // 先删除对应的供应商模型关系
        let mapping_sql = "DELETE FROM model_provider_mapping WHERE provider_id = ?";
        rbatis::RBatis::exec(
            pool!(),
            mapping_sql,
            vec![rbs::Value::String(provider_id.clone())],
        )
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

        let sql = "DELETE FROM provider_config WHERE id = ?";
        rbatis::RBatis::exec(pool!(), sql, vec![rbs::Value::String(provider_id.clone())])
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;

        // 从 ProviderRegistry 注销
        let mut registry = self.ctx.provider_registry.write().await;
        if let Err(e) = registry.unregister_provider(&provider_code) {
            log::error!("[rsllm] Failed to unregister provider from registry: {}", e);
        }

        // 清除缓存
        self.ctx.update_api_key_cache(provider_id, String::new());

        Ok(())
    }

    fn get_decrypted_api_key(&self, provider_id: &str, encrypted_key: &str) -> Result<String> {
        // 尝试从缓存中获取
        {
            let cache = self.ctx.api_key_cache.read().map_err(|e| {
                Error::DatabaseError(format!("Failed to acquire cache lock: {}", e))
            })?;
            if let Some(key) = cache.get(provider_id) {
                return Ok(key.clone());
            }
        }

        // 缓存不存在，需要解密
        let decrypted_key = self.encryption_service.decrypt(encrypted_key)?;

        // 更新缓存
        self.ctx
            .update_api_key_cache(provider_id.to_string(), decrypted_key.clone());

        Ok(decrypted_key)
    }

    async fn initialize_provider_cache(&self) -> Result<()> {
        let all_providers: Vec<ProviderConfig> = ProviderConfig::select_all(pool!())
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;

        for provider in all_providers {
            if !Self::has_valid_provider_type(&provider) {
                log::error!(
                    "[ProviderConfigService] Skipping provider with invalid provider_type in cache init: code={}, name={}, provider_type={}",
                    provider.provider_code,
                    provider.name,
                    provider.provider_type
                );
                continue;
            }

            if let Some(encrypted_key) = &provider.api_key_encrypted {
                let provider_id = provider.id.clone().unwrap_or_else(|| {
                    log::warn!("[rsllm] Provider missing ID: {}", provider.name);
                    String::new()
                });

                if provider_id.is_empty() {
                    continue;
                }

                // 解密API密钥
                match self.encryption_service.decrypt(encrypted_key) {
                    Ok(decrypted_key) => {
                        // 更新缓存
                        self.ctx.update_api_key_cache(provider_id, decrypted_key);
                        log::info!(
                            "[rsllm] Initialized API key cache for provider: {}",
                            provider.name
                        );
                    }
                    Err(e) => {
                        log::error!(
                            "[rsllm] Failed to decrypt API key for provider {}: {}",
                            provider.name,
                            e
                        );
                    }
                }
            }
        }

        Ok(())
    }

    async fn start_provider_oauth(
        &self,
        provider_id: &str,
        req: ProviderOAuthStartRequest,
    ) -> Result<ProviderOAuthStartResponse> {
        let provider = self.get_provider(provider_id).await?;
        let provider_type = Self::validate_provider_oauth_target(&provider)?;

        let oauth_cfg = Self::parse_provider_oauth_config(&provider_type, &provider.auth_config)?;
        let state = Self::generate_url_safe_random(32);
        let (code_verifier, code_challenge) = Self::generate_pkce_pair();
        let expires_at = Self::now_ts() + PROVIDER_OAUTH_SESSION_EXPIRE_MINUTES * 60;

        if Self::is_device_code_provider(&provider_type) {
            let device_id = Self::generate_url_safe_random(24);
            let device_data = Self::request_provider_device_code(
                &provider_type,
                &oauth_cfg,
                &code_challenge,
                &device_id,
            )
            .await?;

            let device_expires_at = if device_data.expires_in > 0 {
                (Self::now_ts() + device_data.expires_in).min(expires_at)
            } else {
                expires_at
            };

            {
                let mut sessions = provider_oauth_sessions().write().await;
                sessions.insert(
                    state.clone(),
                    ProviderOAuthSession {
                        provider_id: provider_id.to_string(),
                        provider_type: provider_type.clone(),
                        code_verifier,
                        flow_type: "device_code".to_string(),
                        device_code: device_data.device_code.clone(),
                        device_id,
                        redirect_uri: String::new(),
                        frontend_redirect_uri: req.redirect_uri,
                        poll_interval_seconds: device_data.interval.max(1),
                        expires_at: device_expires_at,
                    },
                );
            }

            return Ok(ProviderOAuthStartResponse {
                provider_type,
                authorize_url: String::new(),
                state,
                expires_at: Self::ts_to_rfc3339(device_expires_at),
                flow_type: Some("device_code".to_string()),
                device_code: Some(device_data.device_code),
                user_code: Some(device_data.user_code),
                verification_uri: Some(device_data.verification_uri),
                verification_uri_complete: Some(device_data.verification_uri_complete),
                interval_seconds: Some(device_data.interval.max(1)),
            });
        }

        let oauth_redirect_uri = self.get_provider_oauth_callback_url(&provider_type)?;

        {
            let mut sessions = provider_oauth_sessions().write().await;
            sessions.insert(
                state.clone(),
                ProviderOAuthSession {
                    provider_id: provider_id.to_string(),
                    provider_type: provider_type.clone(),
                    code_verifier,
                    flow_type: "authorization_code".to_string(),
                    device_code: String::new(),
                    device_id: String::new(),
                    redirect_uri: oauth_redirect_uri.clone(),
                    frontend_redirect_uri: req.redirect_uri.clone(),
                    poll_interval_seconds: 5,
                    expires_at,
                },
            );
        }

        let mut query = vec![
            ("response_type", "code".to_string()),
            ("client_id", oauth_cfg.client_id),
            ("redirect_uri", oauth_redirect_uri),
            ("state", state.clone()),
            ("code_challenge", code_challenge),
            ("code_challenge_method", "S256".to_string()),
        ];
        if let Some(scope) = oauth_cfg.scope {
            query.push(("scope", scope));
        }

        let authorize_url =
            Url::parse_with_params(&oauth_cfg.authorize_url, query.iter().map(|(k, v)| (*k, v)))
                .map_err(|e| Error::ValidationError(format!("Invalid authorize URL: {}", e)))?
                .to_string();

        Ok(ProviderOAuthStartResponse {
            provider_type,
            authorize_url,
            state,
            expires_at: Self::ts_to_rfc3339(expires_at),
            flow_type: Some("authorization_code".to_string()),
            device_code: None,
            user_code: None,
            verification_uri: None,
            verification_uri_complete: None,
            interval_seconds: None,
        })
    }

    async fn complete_provider_oauth(
        &self,
        provider_id: &str,
        req: ProviderOAuthCallbackRequest,
    ) -> Result<ProviderOAuthStatusResponse> {
        let provider = self.get_provider(provider_id).await?;
        let provider_type = Self::validate_provider_oauth_target(&provider)?;

        let session = {
            let mut sessions = provider_oauth_sessions().write().await;
            sessions.remove(&req.state)
        }
        .ok_or_else(|| Error::ValidationError("Invalid OAuth state".to_string()))?;

        if session.provider_id != provider_id {
            return Err(Error::ValidationError(
                "OAuth state does not match provider".to_string(),
            ));
        }
        if session.provider_type != provider_type {
            return Err(Error::ValidationError(
                "OAuth state does not match provider type".to_string(),
            ));
        }
        if session.flow_type != "authorization_code" {
            return Err(Error::ValidationError(
                "OAuth flow type mismatch for callback".to_string(),
            ));
        }
        if Self::now_ts() > session.expires_at {
            return Err(Error::ValidationError("OAuth state expired".to_string()));
        }

        let oauth_cfg = Self::parse_provider_oauth_config(&provider_type, &provider.auth_config)?;

        let mut form = vec![
            ("grant_type", "authorization_code".to_string()),
            ("code", req.code),
            ("redirect_uri", req.redirect_uri),
            ("client_id", oauth_cfg.client_id.clone()),
            ("code_verifier", session.code_verifier),
        ];
        if let Some(secret) = oauth_cfg.client_secret.clone() {
            form.push(("client_secret", secret));
        }

        let mut token_req = reqwest::Client::new()
            .post(&oauth_cfg.token_url)
            .form(&form);

        // 支持通过 auth_config.injection.basic_username/basic_password 以 Basic 方式提交 client credentials
        if let Some(inj) = provider
            .auth_config
            .as_ref()
            .and_then(|cfg| cfg.get("injection"))
        {
            let username = inj
                .get("basic_username")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|v| !v.is_empty());
            let password = inj
                .get("basic_password")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|v| !v.is_empty());

            if let (Some(u), Some(p)) = (username, password) {
                token_req = token_req.basic_auth(u, Some(p));
            }
        }

        let token_resp = token_req
            .send()
            .await
            .map_err(|e| Error::BusinessError(format!("OAuth token request failed: {}", e)))?;

        if !token_resp.status().is_success() {
            let status = token_resp.status();
            let body = token_resp.text().await.unwrap_or_default();
            return Err(Error::BusinessError(format!(
                "OAuth token exchange failed: status={} body={}",
                status, body
            )));
        }

        let token_data: ProviderOAuthTokenData = token_resp
            .json()
            .await
            .map_err(|e| Error::BusinessError(format!("OAuth token parse failed: {}", e)))?;

        self.persist_provider_oauth_token(
            &provider,
            provider_id,
            &provider_type,
            &oauth_cfg,
            token_data,
        )
        .await
    }

    async fn poll_provider_oauth_device(
        &self,
        provider_id: &str,
        req: ProviderOAuthDevicePollRequest,
    ) -> Result<ProviderOAuthDevicePollResponse> {
        let provider = self.get_provider(provider_id).await?;
        let provider_type = Self::validate_provider_oauth_target(&provider)?;
        if !Self::is_device_code_provider(&provider_type) {
            return Err(Error::ValidationError(
                "Provider does not support device-code OAuth poll".to_string(),
            ));
        }

        let state = req.state.trim().to_string();
        if state.is_empty() {
            return Err(Error::ValidationError("state is required".to_string()));
        }

        let mut sessions = provider_oauth_sessions().write().await;
        let session = sessions
            .get(&state)
            .cloned()
            .ok_or_else(|| Error::ValidationError("Invalid OAuth state".to_string()))?;

        if session.provider_id != provider_id {
            return Err(Error::ValidationError(
                "OAuth state does not match provider".to_string(),
            ));
        }
        if session.provider_type != provider_type {
            return Err(Error::ValidationError(
                "OAuth state does not match provider type".to_string(),
            ));
        }
        if session.flow_type != "device_code" {
            return Err(Error::ValidationError(
                "OAuth flow type mismatch for device poll".to_string(),
            ));
        }
        if Self::now_ts() > session.expires_at {
            sessions.remove(&state);
            return Ok(ProviderOAuthDevicePollResponse {
                status: "failed".to_string(),
                message: Some("OAuth 设备码已过期，请重新发起登录".to_string()),
                retry_after_seconds: None,
                oauth_status: None,
            });
        }

        let oauth_cfg = Self::parse_provider_oauth_config(&provider_type, &provider.auth_config)?;
        match Self::poll_provider_device_token(
            &provider_type,
            &oauth_cfg,
            &session,
            &provider.auth_config,
        )
        .await?
        {
            ProviderOAuthDevicePollOutcome::Pending {
                message,
                retry_after_seconds,
            } => {
                if let Some(stored) = sessions.get_mut(&state) {
                    stored.poll_interval_seconds = retry_after_seconds.max(1);
                }
                Ok(ProviderOAuthDevicePollResponse {
                    status: "pending".to_string(),
                    message,
                    retry_after_seconds: Some(retry_after_seconds.max(1)),
                    oauth_status: None,
                })
            }
            ProviderOAuthDevicePollOutcome::Failed(message) => {
                sessions.remove(&state);
                Ok(ProviderOAuthDevicePollResponse {
                    status: "failed".to_string(),
                    message: Some(message),
                    retry_after_seconds: None,
                    oauth_status: None,
                })
            }
            ProviderOAuthDevicePollOutcome::Success(token_data) => {
                sessions.remove(&state);
                drop(sessions);
                let status = self
                    .persist_provider_oauth_token(
                        &provider,
                        provider_id,
                        &provider_type,
                        &oauth_cfg,
                        token_data,
                    )
                    .await?;
                Ok(ProviderOAuthDevicePollResponse {
                    status: "success".to_string(),
                    message: Some("OAuth 设备码登录成功".to_string()),
                    retry_after_seconds: None,
                    oauth_status: Some(status),
                })
            }
        }
    }

    async fn get_provider_oauth_status(
        &self,
        provider_id: &str,
    ) -> Result<ProviderOAuthStatusResponse> {
        let provider = self.get_provider(provider_id).await?;
        let provider_type = Self::validate_provider_oauth_target(&provider)?;

        Ok(Self::build_provider_oauth_status(&provider_type, &provider))
    }

    async fn disconnect_provider_oauth(
        &self,
        provider_id: &str,
        req: ProviderOAuthDisconnectRequest,
    ) -> Result<ProviderOAuthStatusResponse> {
        let provider = self.get_provider(provider_id).await?;
        let provider_type = Self::validate_provider_oauth_target(&provider)?;

        let mut auth_obj = provider
            .auth_config
            .clone()
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();

        let account_key = req.account_key.unwrap_or_default().trim().to_string();
        if account_key.is_empty() {
            auth_obj.remove(&Self::provider_oauth_key(&provider_type));
            auth_obj.remove(&provider_oauth_accounts_key(&provider_type));
        } else {
            let mut accounts =
                provider_accounts_from_auth_config(&provider.auth_config, &provider_type);
            if accounts.is_empty() {
                return Err(Error::ValidationError(
                    "no oauth accounts found for provider".to_string(),
                ));
            }

            let before_len = accounts.len();
            accounts.retain(|account| account.account_key != account_key);
            if accounts.len() == before_len {
                return Err(Error::ValidationError(format!(
                    "account_key '{}' not found",
                    account_key
                )));
            }

            if accounts.is_empty() {
                auth_obj.remove(&Self::provider_oauth_key(&provider_type));
                auth_obj.remove(&provider_oauth_accounts_key(&provider_type));
            } else {
                auth_obj.insert(
                    provider_oauth_accounts_key(&provider_type),
                    serde_json::to_value(&accounts).map_err(|e| {
                        Error::BusinessError(format!("Serialize OAuth accounts failed: {}", e))
                    })?,
                );

                if let Some(primary) = accounts
                    .iter()
                    .find(|a| a.enabled && !a.access_token.trim().is_empty())
                    .or_else(|| accounts.first())
                    .cloned()
                {
                    auth_obj.insert(
                        Self::provider_oauth_key(&provider_type),
                        serde_json::to_value(primary).map_err(|e| {
                            Error::BusinessError(format!("Serialize OAuth account failed: {}", e))
                        })?,
                    );
                }
            }
        }

        let updated_provider = self
            .update_provider_auth_config(provider_id, JsonValue::Object(auth_obj))
            .await?;
        Ok(Self::build_provider_oauth_status(
            &provider_type,
            &updated_provider,
        ))
    }

    async fn update_provider_oauth_accounts(
        &self,
        provider_id: &str,
        req: ProviderOAuthAccountsUpdateRequest,
    ) -> Result<ProviderOAuthStatusResponse> {
        let provider = self.get_provider(provider_id).await?;
        let provider_type = Self::validate_provider_oauth_target(&provider)?;

        if req.accounts.is_empty() {
            return Err(Error::ValidationError(
                "accounts cannot be empty".to_string(),
            ));
        }

        let mut settings_map: HashMap<String, ProviderOAuthAccountSetting> = HashMap::new();
        for setting in req.accounts {
            let account_key = setting.account_key.trim().to_string();
            if account_key.is_empty() {
                return Err(Error::ValidationError(
                    "account_key cannot be empty".to_string(),
                ));
            }
            if setting.weight == 0 {
                return Err(Error::ValidationError(
                    "weight must be greater than 0".to_string(),
                ));
            }
            if settings_map
                .insert(
                    account_key.clone(),
                    ProviderOAuthAccountSetting {
                        account_key,
                        enabled: setting.enabled,
                        weight: setting.weight,
                    },
                )
                .is_some()
            {
                return Err(Error::ValidationError(
                    "duplicate account_key in request".to_string(),
                ));
            }
        }

        let mut accounts =
            provider_accounts_from_auth_config(&provider.auth_config, &provider_type);
        if accounts.is_empty() {
            return Err(Error::ValidationError(
                "no oauth accounts found for provider".to_string(),
            ));
        }

        for account in &mut accounts {
            if let Some(setting) = settings_map.get(&account.account_key) {
                account.enabled = setting.enabled;
                account.weight = setting.weight.max(1);
                account.updated_at = Self::now_rfc3339();
            }
        }

        let mut auth_obj = provider
            .auth_config
            .clone()
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();
        auth_obj.insert(
            provider_oauth_accounts_key(&provider_type),
            serde_json::to_value(&accounts).map_err(|e| {
                Error::BusinessError(format!("Serialize OAuth accounts failed: {}", e))
            })?,
        );

        if let Some(primary) = accounts
            .iter()
            .find(|a| a.enabled && !a.access_token.trim().is_empty())
            .or_else(|| accounts.first())
            .cloned()
        {
            auth_obj.insert(
                Self::provider_oauth_key(&provider_type),
                serde_json::to_value(primary).map_err(|e| {
                    Error::BusinessError(format!("Serialize OAuth account failed: {}", e))
                })?,
            );
        }

        let updated_provider = self
            .update_provider_auth_config(provider_id, JsonValue::Object(auth_obj))
            .await?;
        Ok(Self::build_provider_oauth_status(
            &provider_type,
            &updated_provider,
        ))
    }

    async fn import_provider_oauth_accounts(
        &self,
        provider_id: &str,
        req: ProviderOAuthAccountsImportRequest,
    ) -> Result<ProviderOAuthStatusResponse> {
        let provider = self.get_provider(provider_id).await?;
        let provider_type = Self::validate_provider_oauth_target(&provider)?;

        if req.accounts.is_empty() {
            return Err(Error::ValidationError(
                "accounts cannot be empty".to_string(),
            ));
        }

        let mut import_map: HashMap<String, ProviderOAuthAccount> = HashMap::new();
        for item in req.accounts {
            let account_key = item.account_key.trim().to_string();
            if account_key.is_empty() {
                return Err(Error::ValidationError(
                    "account_key cannot be empty".to_string(),
                ));
            }
            let access_token = item.access_token.trim().to_string();
            if access_token.is_empty() {
                return Err(Error::ValidationError(format!(
                    "access_token cannot be empty for account_key '{}'",
                    account_key
                )));
            }

            let token_type = if item.token_type.trim().is_empty() {
                "Bearer".to_string()
            } else {
                item.token_type.trim().to_string()
            };

            if import_map
                .insert(
                    account_key.clone(),
                    ProviderOAuthAccount {
                        account_key,
                        access_token,
                        refresh_token: item.refresh_token.trim().to_string(),
                        id_token: item.id_token.trim().to_string(),
                        token_type,
                        email: item.email.trim().to_string(),
                        account_id: item.account_id.trim().to_string(),
                        access_token_expires_at: item.access_token_expires_at.trim().to_string(),
                        updated_at: Self::now_rfc3339(),
                        enabled: item.enabled,
                        weight: item.weight.max(1),
                    },
                )
                .is_some()
            {
                return Err(Error::ValidationError(
                    "duplicate account_key in request".to_string(),
                ));
            }
        }

        let mut accounts =
            provider_accounts_from_auth_config(&provider.auth_config, &provider_type);

        for account in &mut accounts {
            if let Some(imported) = import_map.remove(&account.account_key) {
                *account = imported;
            }
        }

        accounts.extend(import_map.into_values());

        let mut auth_obj = provider
            .auth_config
            .clone()
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();

        auth_obj.insert(
            provider_oauth_accounts_key(&provider_type),
            serde_json::to_value(&accounts).map_err(|e| {
                Error::BusinessError(format!("Serialize OAuth accounts failed: {}", e))
            })?,
        );

        if let Some(primary) = accounts
            .iter()
            .find(|a| a.enabled && !a.access_token.trim().is_empty())
            .or_else(|| accounts.first())
            .cloned()
        {
            auth_obj.insert(
                Self::provider_oauth_key(&provider_type),
                serde_json::to_value(primary).map_err(|e| {
                    Error::BusinessError(format!("Serialize OAuth account failed: {}", e))
                })?,
            );
        }

        let updated_provider = self
            .update_provider_auth_config(provider_id, JsonValue::Object(auth_obj))
            .await?;

        Ok(Self::build_provider_oauth_status(
            &provider_type,
            &updated_provider,
        ))
    }

    async fn complete_provider_oauth_public_callback(
        &self,
        state: &str,
        code: Option<&str>,
        error: Option<&str>,
        expected_provider_type: Option<&str>,
    ) -> Result<String> {
        let state = state.trim();
        if state.is_empty() {
            return Err(Error::ValidationError(
                "OAuth state is required".to_string(),
            ));
        }

        let session = {
            let sessions = provider_oauth_sessions().read().await;
            sessions.get(state).cloned()
        }
        .ok_or_else(|| Error::ValidationError("Invalid or expired OAuth state".to_string()))?;

        if let Some(expected_provider_type) = expected_provider_type {
            let expected = expected_provider_type.trim().to_lowercase();
            if !expected.is_empty() && expected != session.provider_type {
                let mut sessions = provider_oauth_sessions().write().await;
                sessions.remove(state);
                return Ok(Self::build_provider_frontend_redirect_url(
                    &session.frontend_redirect_uri,
                    &session.provider_id,
                    &session.provider_type,
                    false,
                    Some("OAuth provider type mismatch"),
                ));
            }
        }

        if Self::now_ts() > session.expires_at {
            let mut sessions = provider_oauth_sessions().write().await;
            sessions.remove(state);
            return Ok(Self::build_provider_frontend_redirect_url(
                &session.frontend_redirect_uri,
                &session.provider_id,
                &session.provider_type,
                false,
                Some("OAuth state expired"),
            ));
        }

        if let Some(err_msg) = error
            && !err_msg.trim().is_empty()
        {
            let mut sessions = provider_oauth_sessions().write().await;
            sessions.remove(state);
            return Ok(Self::build_provider_frontend_redirect_url(
                &session.frontend_redirect_uri,
                &session.provider_id,
                &session.provider_type,
                false,
                Some(err_msg.trim()),
            ));
        }

        let code = code
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| Error::ValidationError("OAuth code is required".to_string()))?;

        let result = self
            .complete_provider_oauth(
                &session.provider_id,
                ProviderOAuthCallbackRequest {
                    code: code.to_string(),
                    state: state.to_string(),
                    redirect_uri: session.redirect_uri.clone(),
                },
            )
            .await;

        match result {
            Ok(_) => Ok(Self::build_provider_frontend_redirect_url(
                &session.frontend_redirect_uri,
                &session.provider_id,
                &session.provider_type,
                true,
                None,
            )),
            Err(e) => Ok(Self::build_provider_frontend_redirect_url(
                &session.frontend_redirect_uri,
                &session.provider_id,
                &session.provider_type,
                false,
                Some(&e.to_string()),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ListProvidersRequest, ProviderConfigServiceImpl};
    use crate::domain::table::ai_hub::provider_config::ProviderConfig;
    use rbs::{Value, value::map::ValueMap};
    use serde_json::json;

    fn build_provider(provider_type: &str) -> ProviderConfig {
        ProviderConfig {
            provider_code: "code-1".to_string(),
            name: "name-1".to_string(),
            provider_type: provider_type.to_string(),
            api_base: "https://example.com".to_string(),
            ..ProviderConfig::default()
        }
    }

    fn build_query_row(entries: Vec<(&str, Value)>) -> Value {
        let mut map = ValueMap::new();
        for (key, value) in entries {
            map.insert(key.into(), value);
        }
        Value::Map(map)
    }

    #[test]
    fn validate_and_normalize_provider_type_should_accept_custom() {
        let normalized = ProviderConfigServiceImpl::validate_and_normalize_provider_type("custom")
            .expect("custom should be accepted");
        assert_eq!(normalized, "custom");
    }

    #[test]
    fn validate_and_normalize_provider_type_should_accept_openai() {
        let normalized = ProviderConfigServiceImpl::validate_and_normalize_provider_type("openai")
            .expect("openai should be accepted");
        assert_eq!(normalized, "openai");
    }

    #[test]
    fn validate_and_normalize_provider_type_should_accept_codex() {
        let normalized = ProviderConfigServiceImpl::validate_and_normalize_provider_type("codex")
            .expect("codex should be accepted");
        assert_eq!(normalized, "codex");
    }

    #[test]
    fn validate_and_normalize_provider_type_should_accept_all_new_types() {
        let new_types = [
            ("claude", "claude"),
            ("gemini", "gemini"),
            ("gemini-cli", "gemini-cli"),
            ("vertex", "vertex"),
            ("aistudio", "aistudio"),
            ("qwen", "qwen"),
            ("kimi", "kimi"),
            ("iflow", "iflow"),
            ("antigravity", "antigravity"),
        ];
        for (input, expected) in new_types {
            let normalized = ProviderConfigServiceImpl::validate_and_normalize_provider_type(input)
                .unwrap_or_else(|_| panic!("{} should be accepted", input));
            assert_eq!(normalized, expected);
        }
    }

    #[test]
    fn validate_and_normalize_provider_type_should_canonicalize_openai_case() {
        let normalized =
            ProviderConfigServiceImpl::validate_and_normalize_provider_type("  OpenAI  ")
                .expect("openai should be accepted case-insensitively with whitespace");
        assert_eq!(normalized, "openai");
    }

    #[test]
    fn validate_and_normalize_provider_type_should_canonicalize_case() {
        let normalized =
            ProviderConfigServiceImpl::validate_and_normalize_provider_type("OpenAI-Compatible")
                .expect("openai-compatible should be accepted case-insensitively");
        assert_eq!(normalized, "openai-compatible");
    }

    #[test]
    fn validate_and_normalize_provider_type_should_reject_unknown_type() {
        let result =
            ProviderConfigServiceImpl::validate_and_normalize_provider_type("unknown-provider");
        assert!(result.is_err());
    }

    #[test]
    fn validate_and_normalize_provider_type_should_reject_blank_input() {
        let result = ProviderConfigServiceImpl::validate_and_normalize_provider_type("   ");
        assert!(result.is_err());
    }

    #[test]
    fn validate_and_normalize_optional_provider_type_should_canonicalize_case() {
        let normalized = ProviderConfigServiceImpl::validate_and_normalize_optional_provider_type(
            Some("OpenAI-Compatible"),
        )
        .expect("openai-compatible should be accepted case-insensitively");
        assert_eq!(normalized, Some("openai-compatible".to_string()));
    }

    #[test]
    fn validate_and_normalize_optional_provider_type_should_canonicalize_surrounded_whitespace() {
        let normalized = ProviderConfigServiceImpl::validate_and_normalize_optional_provider_type(
            Some("  OpenAI  "),
        )
        .expect("openai should be accepted case-insensitively with whitespace");
        assert_eq!(normalized, Some("openai".to_string()));
    }

    #[test]
    fn validate_and_normalize_optional_provider_type_should_accept_codex() {
        let normalized = ProviderConfigServiceImpl::validate_and_normalize_optional_provider_type(
            Some("  CoDeX  "),
        )
        .expect("codex should be accepted case-insensitively with whitespace");
        assert_eq!(normalized, Some("codex".to_string()));
    }

    #[test]
    fn validate_and_normalize_optional_provider_type_should_reject_unknown_type() {
        let result = ProviderConfigServiceImpl::validate_and_normalize_optional_provider_type(
            Some("unknown-provider"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn validate_and_normalize_optional_provider_type_should_reject_blank_input() {
        let result =
            ProviderConfigServiceImpl::validate_and_normalize_optional_provider_type(Some("   "));
        assert!(result.is_err());
    }

    #[test]
    fn validate_and_normalize_optional_provider_type_should_keep_none() {
        let normalized =
            ProviderConfigServiceImpl::validate_and_normalize_optional_provider_type(None)
                .expect("none provider_type should be kept as none");
        assert_eq!(normalized, None);
    }

    #[test]
    fn normalize_optional_provider_type_for_query_should_canonicalize_known_type() {
        let normalized = ProviderConfigServiceImpl::normalize_optional_provider_type_for_query(
            Some("  OpenAI-Compatible  "),
        )
        .expect("known provider_type should be normalized");
        assert_eq!(normalized, Some("openai-compatible".to_string()));
    }

    #[test]
    fn normalize_optional_provider_type_for_query_should_keep_unknown_type_query_compatible() {
        let normalized = ProviderConfigServiceImpl::normalize_optional_provider_type_for_query(
            Some("  Claude  "),
        )
        .expect("unknown provider_type query should stay compatible");
        assert_eq!(normalized, Some("claude".to_string()));
    }

    #[test]
    fn normalize_optional_provider_type_for_query_should_reject_empty_input() {
        let result =
            ProviderConfigServiceImpl::normalize_optional_provider_type_for_query(Some("   "));
        assert!(result.is_err());
    }

    #[test]
    fn normalize_optional_provider_type_for_query_should_keep_none() {
        let normalized =
            ProviderConfigServiceImpl::normalize_optional_provider_type_for_query(None)
                .expect("none provider_type query should be kept as none");
        assert_eq!(normalized, None);
    }

    #[test]
    fn normalize_provider_type_for_query_compatible_should_canonicalize_known_type() {
        let normalized = ProviderConfigServiceImpl::normalize_provider_type_for_query_compatible(
            "  OpenAI-Compatible  ",
        );
        assert_eq!(normalized, "openai-compatible");
    }

    #[test]
    fn normalize_provider_type_for_query_compatible_should_keep_unknown_type_query_compatible() {
        let normalized =
            ProviderConfigServiceImpl::normalize_provider_type_for_query_compatible("  Claude  ");
        assert_eq!(normalized, "claude");
    }

    #[test]
    fn normalize_provider_type_for_query_compatible_should_trim_empty_to_empty_string() {
        let normalized =
            ProviderConfigServiceImpl::normalize_provider_type_for_query_compatible("   ");
        assert_eq!(normalized, "");
    }

    #[test]
    fn has_valid_provider_type_should_accept_known_provider_type() {
        let provider = build_provider("openai-compatible");
        assert!(ProviderConfigServiceImpl::has_valid_provider_type(
            &provider
        ));
    }

    #[test]
    fn has_valid_provider_type_should_accept_case_insensitive_provider_type() {
        let provider = build_provider("OpenAI-Compatible");
        assert!(ProviderConfigServiceImpl::has_valid_provider_type(
            &provider
        ));
    }

    #[test]
    fn has_valid_provider_type_should_accept_surrounded_whitespace() {
        let provider = build_provider("  OpenAI-Compatible  ");
        assert!(ProviderConfigServiceImpl::has_valid_provider_type(
            &provider
        ));
    }

    #[test]
    fn has_valid_provider_type_should_accept_codex() {
        let provider = build_provider("CoDeX");
        assert!(ProviderConfigServiceImpl::has_valid_provider_type(
            &provider
        ));
    }

    #[test]
    fn has_valid_provider_type_should_accept_all_new_types() {
        let new_types = [
            "claude",
            "gemini",
            "gemini-cli",
            "vertex",
            "aistudio",
            "qwen",
            "kimi",
            "iflow",
            "antigravity",
        ];
        for provider_type in new_types {
            let provider = build_provider(provider_type);
            assert!(
                ProviderConfigServiceImpl::has_valid_provider_type(&provider),
                "{} should be valid",
                provider_type
            );
        }
    }

    #[test]
    fn has_valid_provider_type_should_reject_invalid_provider_type() {
        let provider = build_provider("unknown-provider");
        assert!(!ProviderConfigServiceImpl::has_valid_provider_type(
            &provider
        ));
    }

    #[test]
    fn normalize_provider_type_for_comparison_should_canonicalize_case_and_whitespace() {
        let normalized = ProviderConfigServiceImpl::normalize_provider_type_for_comparison(
            "  OpenAI-Compatible  ",
        );
        assert_eq!(normalized, Some("openai-compatible".to_string()));
    }

    #[test]
    fn normalize_provider_type_for_comparison_should_reject_unknown_type() {
        let normalized =
            ProviderConfigServiceImpl::normalize_provider_type_for_comparison("unknown-provider");
        assert_eq!(normalized, None);
    }

    #[test]
    fn default_api_base_for_provider_type_should_return_codex_default() {
        let default_api_base =
            ProviderConfigServiceImpl::default_api_base_for_provider_type("codex");
        assert_eq!(default_api_base, Some("https://api.openai.com/v1"));
    }

    #[test]
    fn default_api_base_for_provider_type_should_return_defaults_for_all_known_types() {
        let expected_defaults = [
            ("openai", "https://api.openai.com/v1"),
            ("codex", "https://api.openai.com/v1"),
            ("claude", "https://api.anthropic.com/v1"),
            ("gemini", "https://generativelanguage.googleapis.com/v1beta"),
            (
                "gemini-cli",
                "https://generativelanguage.googleapis.com/v1beta",
            ),
            (
                "aistudio",
                "https://generativelanguage.googleapis.com/v1beta",
            ),
            ("vertex", "https://us-central1-aiplatform.googleapis.com/v1"),
            ("qwen", "https://dashscope.aliyuncs.com/compatible-mode/v1"),
            ("kimi", "https://api.kimi.com/coding/v1"),
            ("iflow", "https://api.iflow.ai/v1"),
            ("antigravity", "https://api.antigravity.ai/v1"),
        ];
        for (provider_type, expected_url) in expected_defaults {
            let default_api_base =
                ProviderConfigServiceImpl::default_api_base_for_provider_type(provider_type);
            assert_eq!(
                default_api_base,
                Some(expected_url),
                "{} should have default {}",
                provider_type,
                expected_url
            );
        }
    }

    #[test]
    fn default_api_base_for_provider_type_should_not_return_default_for_custom() {
        let default_api_base =
            ProviderConfigServiceImpl::default_api_base_for_provider_type("custom");
        assert_eq!(default_api_base, None);
    }

    #[test]
    fn default_api_base_for_provider_type_should_not_return_default_for_openai_compatible() {
        let default_api_base =
            ProviderConfigServiceImpl::default_api_base_for_provider_type("openai-compatible");
        assert_eq!(default_api_base, None);
    }

    #[test]
    fn provider_supports_oauth_summary_should_only_enable_supported_types() {
        assert!(ProviderConfigServiceImpl::provider_supports_oauth_summary(
            "openai-compatible"
        ));
        assert!(ProviderConfigServiceImpl::provider_supports_oauth_summary(
            "codex"
        ));
        assert!(!ProviderConfigServiceImpl::provider_supports_oauth_summary(
            "unknown-provider"
        ));
    }

    #[test]
    fn parse_query_count_should_support_numeric_and_string_rows() {
        let numeric_count = Value::Array(vec![build_query_row(vec![("count", 3.into())])]);
        let string_count = Value::Array(vec![build_query_row(vec![("COUNT(*)", "7".into())])]);

        assert_eq!(
            ProviderConfigServiceImpl::parse_query_count(&numeric_count),
            3
        );
        assert_eq!(
            ProviderConfigServiceImpl::parse_query_count(&string_count),
            7
        );
    }

    #[test]
    fn parse_query_providers_should_fallback_to_default_for_unexpected_rows() {
        let rows = Value::Array(vec![Value::Null]);

        let providers = ProviderConfigServiceImpl::parse_query_providers(&rows);

        assert_eq!(providers.len(), 1);
        assert!(providers[0].provider_code.is_empty());
        assert!(providers[0].name.is_empty());
        assert!(providers[0].provider_type.is_empty());
        assert!(providers[0].api_base.is_empty());
        assert!(providers[0].status.is_none());
    }

    #[test]
    fn list_providers_request_should_deserialize_legacy_page_size_alias() {
        let request: ListProvidersRequest = serde_json::from_value(json!({
            "page": 2,
            "page_size": 50,
            "keyword": "openai"
        }))
        .expect("legacy page_size should deserialize into size");

        assert_eq!(request.page, Some(2));
        assert_eq!(request.size, Some(50));
        assert_eq!(request.keyword.as_deref(), Some("openai"));
    }
}
