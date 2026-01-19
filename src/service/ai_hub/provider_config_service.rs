//! 供应商配置服务模块
//! 负责管理AI服务供应商的配置和生命周期

use async_trait::async_trait;
use rbatis::rbdc::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::context::ServiceContext;
use crate::domain::dto::provider::ProviderConfig as ProviderConfigDTO;
use crate::domain::table::ai_hub::provider_config::ProviderConfig;
use crate::error::{Error, Result};
use crate::pool;
use crate::service::ai_hub::encryption_service::EncryptionService;
use std::sync::Arc;

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
}

/// 创建供应商请求
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateProviderRequest {
    pub provider_code: String,
    pub name: String,
    pub provider_type: String,
    pub api_base: String,
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
pub struct ListProvidersResponse {
    pub items: Vec<ProviderConfig>,
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
}

#[async_trait]
impl ProviderConfigService for ProviderConfigServiceImpl {
    async fn create_provider(&self, req: CreateProviderRequest) -> Result<ProviderConfig> {
        // 验证必填字段
        if req.provider_code.is_empty() {
            return Err(Error::ValidationError(
                "Provider code cannot be empty".to_string(),
            ));
        }
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
        if req.api_base.is_empty() {
            return Err(Error::ValidationError(
                "API base cannot be empty".to_string(),
            ));
        }

        // 检查供应商名称是否已存在
        if (self.get_provider_by_name(&req.name).await).is_ok() {
            return Err(Error::ValidationError(format!(
                "Provider name '{}' already exists",
                req.name
            )));
        }

        // 检查供应商代码是否已存在
        if (self.get_provider_by_code(&req.provider_code).await).is_ok() {
            return Err(Error::ValidationError(format!(
                "Provider code '{}' already exists",
                req.provider_code
            )));
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
            provider_type: req.provider_type,
            api_base: req.api_base,
            api_key_encrypted,
            auth_config: req.auth_config,
            circuit_breaker_enabled: req.circuit_breaker_enabled,
            failure_threshold: req.failure_threshold,
            status: req.status,
            description: req.description,
            documentation_url: req.documentation_url,
            created_at: Some(DateTime::now()),
            updated_at: Some(DateTime::now()),
        };

        // 保存到数据库
        let result = ProviderConfig::insert(pool!(), &provider).await;
        match result {
            Ok(_) => {
                // 注册到 ProviderRegistry
                let provider_dto: ProviderConfigDTO = (&provider).into();
                let mut registry = self.ctx.provider_registry.write().await;
                if let Err(e) = registry.register_provider(provider_dto, api_key_decrypted) {
                    log::error!("[rsllm] Failed to register provider to registry: {}", e);
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
        let page = req.page.unwrap_or(1);
        let size = req.size.unwrap_or(20);
        let offset = (page - 1) * size;

        // 简化查询 - 使用 select_all 然后手动过滤
        let all_providers: Vec<ProviderConfig> = ProviderConfig::select_all(pool!())
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;

        let mut filtered_providers: Vec<ProviderConfig> = Vec::new();

        for provider in all_providers {
            // 过滤供应商类型
            if let Some(provider_type) = &req.provider_type
                && &provider.provider_type != provider_type
            {
                continue;
            }

            // 过滤状态
            if let Some(status) = &req.status
                && provider.status.as_ref() != Some(status)
            {
                continue;
            }

            // 关键词搜索
            if let Some(keyword) = &req.keyword {
                let keyword_lower = keyword.to_lowercase();
                let name_match = provider.name.to_lowercase().contains(&keyword_lower);
                let desc_match = provider
                    .description
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(&keyword_lower))
                    .unwrap_or(false);
                if !name_match && !desc_match {
                    continue;
                }
            }

            filtered_providers.push(provider);
        }

        let total = filtered_providers.len() as i64;

        // 分页
        let start = offset as usize;
        let end = std::cmp::min(start + size as usize, filtered_providers.len());
        let items = if start < filtered_providers.len() {
            filtered_providers[start..end].to_vec()
        } else {
            Vec::new()
        };

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
            // 验证新的provider_code不与其他供应商冲突
            if provider_code != provider.provider_code {
                if (self.get_provider_by_code(&provider_code).await).is_ok() {
                    return Err(Error::ValidationError(format!(
                        "Provider code '{}' already exists",
                        provider_code
                    )));
                }
                provider.provider_code = provider_code;
            }
        }
        if let Some(name) = req.name {
            provider.name = name;
        }
        if let Some(provider_type) = req.provider_type {
            provider.provider_type = provider_type;
        }
        if let Some(api_base) = req.api_base {
            provider.api_base = api_base;
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

        provider.updated_at = Some(DateTime::now());

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
        let provider_dto: ProviderConfigDTO = (&provider).into();
        let mut registry = self.ctx.provider_registry.write().await;
        if let Err(e) = registry.update_provider(provider_dto, api_key_decrypted) {
            log::error!("[rsllm] Failed to update provider in registry: {}", e);
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
        rbatis::RBatis::exec(pool!(), mapping_sql, vec![rbs::Value::String(provider_id.clone())])
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
}
