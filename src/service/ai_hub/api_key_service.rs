//! API密钥管理服务模块
//! 提供API密钥的创建、验证、查询和管理功能
use crate::domain::dto::ai_hub::api_key::{
    ApiKeyQueryDTO, ApiKeyResponse, ApiKeyValidationResult, CreateApiKeyDTO, ListApiKeysResponse,
    UpdateApiKeyDTO,
};
use crate::domain::table::ai_hub::api_key::ApiKey;
use crate::domain::table::basic::SysUser;
use crate::error::{ApplicationError, ApplicationResult};
use crate::pool;
use rbatis::rbdc::DateTime;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

const API_KEY_VALIDATION_CACHE_TTL: Duration = Duration::from_secs(30);

#[derive(Clone)]
struct CachedApiKeyValidation {
    expires_at: Instant,
    result: ApiKeyValidationResult,
}

/// API密钥管理服务
///
/// 负责API密钥的创建、验证、查询和管理
#[derive(Clone)]
pub struct ApiKeyService {
    validation_cache: Arc<RwLock<HashMap<String, CachedApiKeyValidation>>>,
}

impl ApiKeyService {
    pub fn new() -> Self {
        Self {
            validation_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn invalidate_validation_cache(&self) {
        self.validation_cache.write().await.clear();
    }

    /// 创建新的API密钥
    pub async fn create_api_key(
        &self,
        dto: CreateApiKeyDTO,
        current_user_id: String,
    ) -> ApplicationResult<ApiKeyResponse> {
        let _user = SysUser::select_by_map(pool!(), rbs::value! { "id": &current_user_id })
            .await?
            .first()
            .cloned()
            .ok_or_else(|| ApplicationError::NotFound {
                message: format!("User {} not found", current_user_id),
                resource: Some("user".to_string()),
                id: Some(current_user_id.clone()),
            })?;

        let key = ApiKey::generate_key();
        let key_hash = ApiKey::hash_key(&key);
        let key_prefix = format!("sk-{}", &key.chars().skip(3).take(8).collect::<String>());

        let quota_limit = dto.quota_limit.as_ref().and_then(|s| s.parse::<f64>().ok());

        let model_rate_limits_value = if let Some(limits) = &dto.model_rate_limits {
            serde_json::to_value(limits).ok()
        } else {
            None
        };

        let api_key = ApiKey {
            id: Some(ulid::Ulid::new().to_string()),
            key: key.clone(),
            key_hash,
            user_id: current_user_id.clone(),
            name: Some(dto.name),
            description: dto.description,
            prefix: Some(key_prefix.clone()),
            quota_limit,
            quota_period: dto.quota_period,
            quota_used: Some(0.0),
            quota_reset_time: None,
            last_used_time: None,
            enabled: Some(true),
            status: dto.status.or(Some("active".to_string())),
            rate_limit_enabled: Some(dto.rate_limit_enabled),
            max_requests_per_minute: dto.max_requests_per_minute,
            max_tokens_per_minute: dto.max_tokens_per_minute,
            model_rate_limits: model_rate_limits_value,
            permissions: dto.permissions,
            created_at: Some(DateTime::now()),
            updated_at: Some(DateTime::now()),
        };

        let id = api_key
            .id
            .clone()
            .ok_or_else(|| ApplicationError::BusinessError {
                message: "Failed to generate API key ID".to_string(),
                code: Some("API_KEY_ID_GENERATION_FAILED".to_string()),
                context: Some(
                    "Failed to generate API key ID after successful creation".to_string(),
                ),
            })?;

        ApiKey::insert(pool!(), &api_key).await?;
        self.invalidate_validation_cache().await;

        Ok(ApiKeyResponse {
            id,
            key,
            prefix: key_prefix,
            name: api_key.name,
            quota_limit: api_key.quota_limit.map(|d| d.to_string()),
            quota_period: api_key.quota_period,
            quota_used: api_key.quota_used.map(|d| d.to_string()),
            quota_reset_time: api_key.quota_reset_time.map(|dt| dt.to_string()),
            last_used_time: api_key.last_used_time.map(|dt| dt.to_string()),
            enabled: api_key.enabled,
            status: api_key.status,
            created_at: api_key
                .created_at
                .map(|dt| dt.to_string())
                .unwrap_or_default(),
            user_id: current_user_id,
        })
    }

    /// 验证API密钥
    pub async fn validate_api_key(
        &self,
        api_key: &str,
    ) -> ApplicationResult<ApiKeyValidationResult> {
        if !api_key.starts_with("sk-") {
            return Ok(ApiKeyValidationResult {
                valid: false,
                api_key_id: None,
                user_id: None,
                user_level: None,
                error: Some("Invalid API key format".to_string()),
            });
        }

        let key_hash = ApiKey::hash_key(api_key);
        if let Some(cached) = self.validation_cache.read().await.get(&key_hash).cloned()
            && cached.expires_at > Instant::now()
        {
            return Ok(cached.result);
        }

        let keys = ApiKey::select_by_map(pool!(), rbs::value! { "key_hash": &key_hash }).await?;

        let result = if keys.is_empty() {
            ApiKeyValidationResult {
                valid: false,
                api_key_id: None,
                user_id: None,
                user_level: None,
                error: Some("API key not found".to_string()),
            }
        } else {
            let key = &keys[0];

            if key.enabled != Some(true) {
                ApiKeyValidationResult {
                    valid: false,
                    api_key_id: key.id.clone(),
                    user_id: Some(key.user_id.clone()),
                    user_level: None,
                    error: Some("API key is disabled".to_string()),
                }
            } else if key.status.as_deref() != Some("active") {
                ApiKeyValidationResult {
                    valid: false,
                    api_key_id: key.id.clone(),
                    user_id: Some(key.user_id.clone()),
                    user_level: None,
                    error: Some(format!("API key status is: {:?}", key.status)),
                }
            } else {
                let user = SysUser::select_by_map(pool!(), rbs::value! { "id": &key.user_id })
                    .await?
                    .first()
                    .cloned()
                    .ok_or_else(|| ApplicationError::NotFound {
                        message: format!("User {} not found", key.user_id),
                        resource: Some("user".to_string()),
                        id: Some(key.user_id.clone()),
                    })?;

                ApiKeyValidationResult {
                    valid: true,
                    api_key_id: key.id.clone(),
                    user_id: Some(key.user_id.clone()),
                    user_level: user.user_level,
                    error: None,
                }
            }
        };

        self.validation_cache.write().await.insert(
            key_hash,
            CachedApiKeyValidation {
                expires_at: Instant::now() + API_KEY_VALIDATION_CACHE_TTL,
                result: result.clone(),
            },
        );

        Ok(result)
    }

    /// 更新API密钥
    pub async fn update_api_key(
        &self,
        id: &str,
        dto: UpdateApiKeyDTO,
    ) -> ApplicationResult<ApiKeyResponse> {
        let existing = ApiKey::select_by_map(pool!(), rbs::value! { "id": id })
            .await?
            .first()
            .cloned()
            .ok_or_else(|| ApplicationError::NotFound {
                message: format!("API key {} not found", id),
                resource: Some("api_key".to_string()),
                id: Some(id.to_string()),
            })?;

        let quota_limit = dto.quota_limit.as_ref().and_then(|s| s.parse::<f64>().ok());

        let updated_key = ApiKey {
            id: existing.id,
            key: existing.key,
            key_hash: existing.key_hash,
            user_id: existing.user_id,
            name: dto.name.or(existing.name),
            description: dto.description.or(existing.description),
            prefix: existing.prefix,
            quota_limit: quota_limit.or(existing.quota_limit),
            quota_period: dto.quota_period.or(existing.quota_period),
            quota_used: existing.quota_used,
            quota_reset_time: existing.quota_reset_time,
            last_used_time: existing.last_used_time,
            enabled: dto.enabled.or(existing.enabled),
            status: existing.status,
            rate_limit_enabled: existing.rate_limit_enabled,
            max_requests_per_minute: existing.max_requests_per_minute,
            max_tokens_per_minute: existing.max_tokens_per_minute,
            model_rate_limits: existing.model_rate_limits,
            permissions: dto.permissions.or(existing.permissions),
            created_at: existing.created_at,
            updated_at: Some(DateTime::now()),
        };

        ApiKey::update_by_map(pool!(), &updated_key, rbs::value! { "id": id }).await?;
        self.invalidate_validation_cache().await;

        Ok(ApiKeyResponse {
            id: updated_key
                .id
                .ok_or_else(|| ApplicationError::BusinessError {
                    message: "API key missing id".to_string(),
                    code: Some("MISSING_ID".to_string()),
                    context: None,
                })?,
            key: updated_key.key,
            prefix: updated_key.prefix.unwrap_or_default(),
            name: updated_key.name,
            quota_limit: updated_key.quota_limit.map(|d| d.to_string()),
            quota_period: updated_key.quota_period,
            quota_used: updated_key.quota_used.map(|d| d.to_string()),
            quota_reset_time: updated_key.quota_reset_time.map(|dt| dt.to_string()),
            last_used_time: updated_key.last_used_time.map(|dt| dt.to_string()),
            enabled: updated_key.enabled,
            status: updated_key.status,
            created_at: updated_key
                .created_at
                .map(|dt| dt.to_string())
                .unwrap_or_default(),
            user_id: updated_key.user_id,
        })
    }

    /// 删除API密钥
    pub async fn delete_api_key(&self, id: &str) -> ApplicationResult<()> {
        ApiKey::delete_by_map(pool!(), rbs::value! { "id": id }).await?;
        self.invalidate_validation_cache().await;

        Ok(())
    }

    /// 获取API密钥列表
    pub async fn list_api_keys(
        &self,
        dto: ApiKeyQueryDTO,
    ) -> ApplicationResult<ListApiKeysResponse> {
        let page = dto.page.unwrap_or(1);
        let page_size = dto.page_size.unwrap_or(10);
        let offset = ((page - 1) * page_size) as usize;

        let mut conditions = rbs::value! {};

        if let Some(user_id) = &dto.user_id {
            conditions["user_id"] = rbs::value!(user_id);
        }

        if let Some(status) = &dto.status {
            conditions["status"] = rbs::value!(status);
        }

        if let Some(enabled) = dto.enabled {
            conditions["enabled"] = rbs::value!(if enabled { 1 } else { 0 });
        }

        let mut keys = if conditions.is_empty() {
            ApiKey::select_all(pool!()).await?
        } else {
            ApiKey::select_by_map(pool!(), conditions).await?
        };

        let total = keys.len() as i64;

        if let Some(search_term) = &dto.search {
            let search_lower = search_term.to_lowercase();
            keys.retain(|key| {
                if key.user_id.to_lowercase().contains(&search_lower) {
                    return true;
                }
                if let Some(name) = &key.name
                    && name.to_lowercase().contains(&search_lower)
                {
                    return true;
                }
                false
            });
        }

        let paginated_keys: Vec<ApiKey> = keys
            .into_iter()
            .skip(offset)
            .take(page_size as usize)
            .collect();

        let items: Vec<ApiKeyResponse> = paginated_keys
            .into_iter()
            .filter_map(|key| {
                Some(ApiKeyResponse {
                    id: key.id?,
                    key: key.key,
                    prefix: key.prefix.unwrap_or_default(),
                    name: key.name,
                    quota_limit: key.quota_limit.map(|d| d.to_string()),
                    quota_period: key.quota_period,
                    quota_used: key.quota_used.map(|d| d.to_string()),
                    quota_reset_time: key.quota_reset_time.as_ref().map(|dt| dt.to_string()),
                    last_used_time: key.last_used_time.as_ref().map(|dt| dt.to_string()),
                    enabled: key.enabled,
                    status: key.status,
                    created_at: key.created_at.map(|dt| dt.to_string()).unwrap_or_default(),
                    user_id: key.user_id,
                })
            })
            .collect();

        Ok(ListApiKeysResponse { items, total })
    }

    /// 根据ID获取API密钥
    pub async fn get_api_key(&self, id: &str) -> ApplicationResult<ApiKeyResponse> {
        let keys = ApiKey::select_by_map(pool!(), rbs::value! { "id": id }).await?;

        let key = keys.first().ok_or_else(|| ApplicationError::NotFound {
            message: format!("API key {} not found", id),
            resource: Some("api_key".to_string()),
            id: Some(id.to_string()),
        })?;

        let key = key.clone();

        Ok(ApiKeyResponse {
            id: key.id.ok_or_else(|| ApplicationError::BusinessError {
                message: "API key missing id".to_string(),
                code: Some("MISSING_ID".to_string()),
                context: None,
            })?,
            key: key.key,
            prefix: key.prefix.unwrap_or_default(),
            name: key.name,
            quota_limit: key.quota_limit.map(|d| d.to_string()),
            quota_period: key.quota_period,
            quota_used: key.quota_used.map(|d| d.to_string()),
            quota_reset_time: key.quota_reset_time.as_ref().map(|dt| dt.to_string()),
            last_used_time: key.last_used_time.as_ref().map(|dt| dt.to_string()),
            enabled: key.enabled,
            status: key.status,
            created_at: key.created_at.map(|dt| dt.to_string()).unwrap_or_default(),
            user_id: key.user_id,
        })
    }

    /// 更新API密钥使用时间
    pub async fn update_last_used(&self, key_hash: &str) -> ApplicationResult<()> {
        let keys = ApiKey::select_by_map(pool!(), rbs::value! { "key_hash": key_hash }).await?;

        if let Some(key) = keys.first()
            && let Some(id) = &key.id
        {
            ApiKey::update_by_map(
                pool!(),
                &ApiKey {
                    id: Some(id.clone()),
                    key: key.key.clone(),
                    key_hash: key.key_hash.clone(),
                    user_id: key.user_id.clone(),
                    name: key.name.clone(),
                    description: key.description.clone(),
                    prefix: key.prefix.clone(),
                    quota_limit: key.quota_limit,
                    quota_period: key.quota_period.clone(),
                    quota_used: key.quota_used,
                    quota_reset_time: key.quota_reset_time.clone(),
                    last_used_time: Some(DateTime::now()),
                    enabled: key.enabled,
                    status: key.status.clone(),
                    rate_limit_enabled: key.rate_limit_enabled,
                    max_requests_per_minute: key.max_requests_per_minute,
                    max_tokens_per_minute: key.max_tokens_per_minute,
                    model_rate_limits: key.model_rate_limits.clone(),
                    permissions: key.permissions.clone(),
                    created_at: key.created_at.clone(),
                    updated_at: Some(DateTime::now()),
                },
                rbs::value! { "id": id },
            )
            .await?;
        }

        Ok(())
    }

    /// 更新API密钥额度使用量
    pub async fn update_quota_used(&self, id: &str, quota_used: f64) -> ApplicationResult<()> {
        let keys = ApiKey::select_by_map(pool!(), rbs::value! { "id": id }).await?;

        if let Some(key) = keys.first() {
            ApiKey::update_by_map(
                pool!(),
                &ApiKey {
                    id: Some(id.to_string()),
                    key: key.key.clone(),
                    key_hash: key.key_hash.clone(),
                    user_id: key.user_id.clone(),
                    name: key.name.clone(),
                    description: key.description.clone(),
                    prefix: key.prefix.clone(),
                    quota_limit: key.quota_limit,
                    quota_period: key.quota_period.clone(),
                    quota_used: Some(quota_used),
                    quota_reset_time: key.quota_reset_time.clone(),
                    last_used_time: key.last_used_time.clone(),
                    enabled: key.enabled,
                    status: key.status.clone(),
                    rate_limit_enabled: key.rate_limit_enabled,
                    max_requests_per_minute: key.max_requests_per_minute,
                    max_tokens_per_minute: key.max_tokens_per_minute,
                    model_rate_limits: key.model_rate_limits.clone(),
                    permissions: key.permissions.clone(),
                    created_at: key.created_at.clone(),
                    updated_at: Some(DateTime::now()),
                },
                rbs::value! { "id": id },
            )
            .await?;
        }

        Ok(())
    }

    /// 重置API密钥额度
    pub async fn reset_quota(&self, id: &str) -> ApplicationResult<()> {
        let keys = ApiKey::select_by_map(pool!(), rbs::value! { "id": id }).await?;

        if let Some(key) = keys.first() {
            let quota_period = key.quota_period.as_deref();
            let quota_reset_time = match quota_period {
                Some("daily") => Some(DateTime::now() + std::time::Duration::from_secs(86400)),
                Some("weekly") => Some(DateTime::now() + std::time::Duration::from_secs(604800)),
                Some("monthly") => Some(DateTime::now() + std::time::Duration::from_secs(2592000)),
                _ => None,
            };

            ApiKey::update_by_map(
                pool!(),
                &ApiKey {
                    id: Some(id.to_string()),
                    key: key.key.clone(),
                    key_hash: key.key_hash.clone(),
                    user_id: key.user_id.clone(),
                    name: key.name.clone(),
                    description: key.description.clone(),
                    prefix: key.prefix.clone(),
                    quota_limit: key.quota_limit,
                    quota_period: key.quota_period.clone(),
                    quota_used: Some(0.0),
                    quota_reset_time,
                    last_used_time: key.last_used_time.clone(),
                    enabled: key.enabled,
                    status: key.status.clone(),
                    rate_limit_enabled: key.rate_limit_enabled,
                    max_requests_per_minute: key.max_requests_per_minute,
                    max_tokens_per_minute: key.max_tokens_per_minute,
                    model_rate_limits: key.model_rate_limits.clone(),
                    permissions: key.permissions.clone(),
                    created_at: key.created_at.clone(),
                    updated_at: Some(DateTime::now()),
                },
                rbs::value! { "id": id },
            )
            .await?;
        }

        Ok(())
    }

    /// 检查API密钥额度是否充足
    pub async fn check_quota(&self, id: &str, required_quota: f64) -> ApplicationResult<bool> {
        let keys = ApiKey::select_by_map(pool!(), rbs::value! { "id": id }).await?;

        if let Some(key) = keys.first()
            && let Some(quota_limit) = &key.quota_limit
            && let Some(quota_used) = &key.quota_used
        {
            return Ok(quota_used + required_quota <= *quota_limit);
        }

        Ok(true)
    }

    /// 获取用户的所有API密钥数量
    pub async fn count_user_api_keys(&self, user_id: &str) -> ApplicationResult<i64> {
        let keys = ApiKey::select_by_map(pool!(), rbs::value! { "user_id": user_id }).await?;

        Ok(keys.len() as i64)
    }
}
