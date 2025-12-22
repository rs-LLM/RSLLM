//! 供应商配置服务模块
//! 负责管理AI服务供应商的配置和生命周期

use async_trait::async_trait;
use rbatis::rbdc::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::domain::table::ai_hub::provider_config::ProviderConfig;
use crate::error::{Result, Error};
use crate::service::ai_hub::encryption_service::EncryptionService;
use crate::pool;

/// 供应商配置服务 Trait
#[async_trait]
pub trait ProviderConfigService: Send + Sync {
    /// 创建供应商配置
    async fn create_provider(&self, req: CreateProviderRequest) -> Result<ProviderConfig>;
    
    /// 获取供应商配置
    async fn get_provider(&self, id: &str) -> Result<ProviderConfig>;
    
    /// 通过名称获取供应商配置
    async fn get_provider_by_name(&self, name: &str) -> Result<ProviderConfig>;
    
    /// 获取供应商列表（支持分页和过滤）
    async fn list_providers(&self, req: ListProvidersRequest) -> Result<ListProvidersResponse>;
    
    /// 更新供应商配置
    async fn update_provider(&self, id: &str, req: UpdateProviderRequest) -> Result<ProviderConfig>;
    
    /// 删除供应商配置
    async fn delete_provider(&self, id: &str) -> Result<()>;
}

/// 创建供应商请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProviderRequest {
    pub name: String,
    pub provider_type: String,
    pub api_base: String,
    pub api_key: Option<String>,
    pub auth_type: Option<String>,
    pub auth_config: Option<JsonValue>,
    pub default_input_price: Option<f64>,
    pub default_output_price: Option<f64>,
    pub rate_limit_enabled: Option<bool>,
    pub max_concurrent_requests: Option<i32>,
    pub circuit_breaker_enabled: Option<bool>,
    pub failure_threshold: Option<i32>,
    pub timeout_seconds: Option<i32>,
    pub enabled: Option<bool>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub documentation_url: Option<String>,
}

/// 列表供应商请求
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ListProvidersRequest {
    pub page: Option<i64>,
    pub size: Option<i64>,
    pub provider_type: Option<String>,
    pub enabled: Option<bool>,
    pub status: Option<String>,
    pub keyword: Option<String>,
}

/// 列表供应商响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProvidersResponse {
    pub items: Vec<ProviderConfig>,
    pub total: i64,
    pub page: i64,
    pub size: i64,
}

/// 更新供应商请求
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateProviderRequest {
    pub name: Option<String>,
    pub api_base: Option<String>,
    pub api_key: Option<String>,
    pub auth_type: Option<String>,
    pub auth_config: Option<JsonValue>,
    pub default_input_price: Option<f64>,
    pub default_output_price: Option<f64>,
    pub rate_limit_enabled: Option<bool>,
    pub max_concurrent_requests: Option<i32>,
    pub circuit_breaker_enabled: Option<bool>,
    pub failure_threshold: Option<i32>,
    pub timeout_seconds: Option<i32>,
    pub enabled: Option<bool>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub documentation_url: Option<String>,
}

/// 供应商配置服务实现
pub struct ProviderConfigServiceImpl {
    encryption_service: EncryptionService,
}

impl ProviderConfigServiceImpl {
    pub fn new(encryption_service: EncryptionService) -> Self {
        Self { encryption_service }
    }
}

#[async_trait]
impl ProviderConfigService for ProviderConfigServiceImpl {
    async fn create_provider(&self, req: CreateProviderRequest) -> Result<ProviderConfig> {
        // 验证必填字段
        if req.name.is_empty() {
            return Err(Error::ValidationError("Provider name cannot be empty".to_string()));
        }
        if req.provider_type.is_empty() {
            return Err(Error::ValidationError("Provider type cannot be empty".to_string()));
        }
        if req.api_base.is_empty() {
            return Err(Error::ValidationError("API base cannot be empty".to_string()));
        }

        // 检查供应商名称是否已存在
        if let Ok(_) = self.get_provider_by_name(&req.name).await {
            return Err(Error::ValidationError(format!("Provider name '{}' already exists", req.name)));
        }

        // 加密API密钥（如果提供）
        let api_key_encrypted = if let Some(api_key) = req.api_key {
            Some(self.encryption_service.encrypt(&api_key)?)
        } else {
            None
        };

        let provider = ProviderConfig {
            id: Some(uuid::Uuid::new_v4().to_string()),
            name: req.name,
            provider_type: req.provider_type,
            api_base: req.api_base,
            api_key_encrypted,
            auth_type: req.auth_type,
            auth_config: req.auth_config,
            default_input_price: req.default_input_price,
            default_output_price: req.default_output_price,
            rate_limit_enabled: req.rate_limit_enabled,
            max_concurrent_requests: req.max_concurrent_requests,
            circuit_breaker_enabled: req.circuit_breaker_enabled,
            failure_threshold: req.failure_threshold,
            timeout_seconds: req.timeout_seconds,
            enabled: req.enabled,
            status: req.status,
            description: req.description,
            documentation_url: req.documentation_url,
            created_at: Some(DateTime::now()),
            updated_at: Some(DateTime::now()),
        };

        // 保存到数据库
        let result = ProviderConfig::insert(pool!(), &provider).await;
        match result {
            Ok(_) => Ok(provider),
            Err(e) => Err(Error::DatabaseError(e.to_string())),
        }
    }

    async fn get_provider(&self, id: &str) -> Result<ProviderConfig> {
        let sql = "SELECT * FROM provider_config WHERE id = ? LIMIT 1";
        let result = rbatis::RBatis::query(pool!(), sql, vec![rbs::Value::String(id.to_string())]).await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;
        
        let value = result.as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| Error::NotFound(format!("Provider with id '{}' not found", id)))?;
        
        let json_value = serde_json::to_value(value).unwrap_or_default();
        serde_json::from_value(json_value)
            .map_err(|e| Error::DatabaseError(e.to_string()))
    }

    async fn get_provider_by_name(&self, name: &str) -> Result<ProviderConfig> {
        let sql = "SELECT * FROM provider_config WHERE name = ? LIMIT 1";
        let result = rbatis::RBatis::query(pool!(), sql, vec![rbs::Value::String(name.to_string())]).await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;
        
        let value = result.as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| Error::NotFound(format!("Provider with name '{}' not found", name)))?;
        
        let json_value = serde_json::to_value(value).unwrap_or_default();
        serde_json::from_value(json_value)
            .map_err(|e| Error::DatabaseError(e.to_string()))
    }

    async fn list_providers(&self, req: ListProvidersRequest) -> Result<ListProvidersResponse> {
        let page = req.page.unwrap_or(1);
        let size = req.size.unwrap_or(20);
        let offset = (page - 1) * size;

        // 简化查询 - 使用 select_all 然后手动过滤
        let all_providers: Vec<ProviderConfig> = ProviderConfig::select_all(pool!()).await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;

        let mut filtered_providers: Vec<ProviderConfig> = Vec::new();

        for provider in all_providers {
            // 过滤供应商类型
            if let Some(provider_type) = &req.provider_type {
                if &provider.provider_type != provider_type {
                    continue;
                }
            }

            // 过滤启用状态
            if let Some(enabled) = req.enabled {
                if provider.enabled.unwrap_or(false) != enabled {
                    continue;
                }
            }

            // 过滤状态
            if let Some(status) = &req.status {
                if provider.status.as_ref() != Some(status) {
                    continue;
                }
            }

            // 关键词搜索
            if let Some(keyword) = &req.keyword {
                let keyword_lower = keyword.to_lowercase();
                let name_match = provider.name.to_lowercase().contains(&keyword_lower);
                let desc_match = provider.description.as_ref()
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

    async fn update_provider(&self, id: &str, req: UpdateProviderRequest) -> Result<ProviderConfig> {
        // 获取现有供应商
        let mut provider = self.get_provider(id).await?;

        // 更新字段
        if let Some(name) = req.name {
            provider.name = name;
        }
        if let Some(api_base) = req.api_base {
            provider.api_base = api_base;
        }
        if let Some(api_key) = req.api_key {
            provider.api_key_encrypted = Some(self.encryption_service.encrypt(&api_key)?);
        }
        if let Some(auth_type) = req.auth_type {
            provider.auth_type = Some(auth_type);
        }
        if let Some(auth_config) = req.auth_config {
            provider.auth_config = Some(auth_config);
        }
        if let Some(default_input_price) = req.default_input_price {
            provider.default_input_price = Some(default_input_price);
        }
        if let Some(default_output_price) = req.default_output_price {
            provider.default_output_price = Some(default_output_price);
        }
        if let Some(rate_limit_enabled) = req.rate_limit_enabled {
            provider.rate_limit_enabled = Some(rate_limit_enabled);
        }
        if let Some(max_concurrent_requests) = req.max_concurrent_requests {
            provider.max_concurrent_requests = Some(max_concurrent_requests);
        }
        if let Some(circuit_breaker_enabled) = req.circuit_breaker_enabled {
            provider.circuit_breaker_enabled = Some(circuit_breaker_enabled);
        }
        if let Some(failure_threshold) = req.failure_threshold {
            provider.failure_threshold = Some(failure_threshold);
        }
        if let Some(timeout_seconds) = req.timeout_seconds {
            provider.timeout_seconds = Some(timeout_seconds);
        }
        if let Some(enabled) = req.enabled {
            provider.enabled = Some(enabled);
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
        let sql = "UPDATE provider_config SET name = ?, provider_type = ?, api_base = ?, api_key_encrypted = ?, auth_type = ?, auth_config = ?, default_input_price = ?, default_output_price = ?, rate_limit_enabled = ?, max_concurrent_requests = ?, circuit_breaker_enabled = ?, failure_threshold = ?, timeout_seconds = ?, enabled = ?, status = ?, description = ?, documentation_url = ?, updated_at = ? WHERE id = ?";
        rbatis::RBatis::exec(pool!(), sql, vec![
            rbs::Value::String(provider.name.clone()),
            rbs::Value::String(provider.provider_type.clone()),
            rbs::Value::String(provider.api_base.clone()),
            rbs::Value::String(provider.api_key_encrypted.clone().unwrap_or_default()),
            rbs::Value::String(provider.auth_type.clone().unwrap_or_default()),
            rbs::Value::String(provider.auth_config.clone().unwrap_or_default().to_string()),
            rbs::Value::F64(provider.default_input_price.unwrap_or_default()),
            rbs::Value::F64(provider.default_output_price.unwrap_or_default()),
            rbs::Value::Bool(provider.rate_limit_enabled.unwrap_or_default()),
            rbs::Value::I32(provider.max_concurrent_requests.unwrap_or_default()),
            rbs::Value::Bool(provider.circuit_breaker_enabled.unwrap_or_default()),
            rbs::Value::I32(provider.failure_threshold.unwrap_or_default()),
            rbs::Value::I32(provider.timeout_seconds.unwrap_or_default()),
            rbs::Value::Bool(provider.enabled.unwrap_or_default()),
            rbs::Value::String(provider.status.clone().unwrap_or_default()),
            rbs::Value::String(provider.description.clone().unwrap_or_default()),
            rbs::Value::String(provider.documentation_url.clone().unwrap_or_default()),
            rbs::Value::String(provider.updated_at.clone().unwrap_or_default().to_string()),
            rbs::Value::String(provider.id.clone().unwrap_or_default())
        ]).await.map_err(|e| Error::DatabaseError(e.to_string()))?;

        Ok(provider)
    }

    async fn delete_provider(&self, id: &str) -> Result<()> {
        let provider = self.get_provider(id).await?;
        let provider_id = provider.id.ok_or_else(|| Error::BusinessError("Provider ID is missing".to_string()))?;
        let sql = "DELETE FROM provider_config WHERE id = ?";
        rbatis::RBatis::exec(pool!(), sql, vec![rbs::Value::String(provider_id)])
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;
        Ok(())
    }
}