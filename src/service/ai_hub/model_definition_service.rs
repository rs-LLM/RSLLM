//! 模型定义服务模块
//! 负责管理AI模型的定义、配置和生命周期

use async_trait::async_trait;
use rbatis::rbdc::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::domain::table::ai_hub::model_definition::ModelDefinition;
use crate::error::{Result, Error};
use crate::service::ai_hub::encryption_service::EncryptionService;
use crate::pool;

/// 模型定义服务 Trait
#[async_trait]
pub trait ModelDefinitionService: Send + Sync {
    /// 创建模型定义
    async fn create_model(&self, req: CreateModelRequest) -> Result<ModelDefinition>;
    
    /// 获取模型定义
    async fn get_model(&self, id: &str) -> Result<ModelDefinition>;
    
    /// 通过key获取模型定义
    async fn get_model_by_key(&self, key: &str) -> Result<ModelDefinition>;
    
    /// 获取模型列表（支持分页和过滤）
    async fn list_models(&self, req: ListModelsRequest) -> Result<ListModelsResponse>;
    
    /// 更新模型定义
    async fn update_model(&self, id: &str, req: UpdateModelRequest) -> Result<ModelDefinition>;
    
    /// 删除模型定义
    async fn delete_model(&self, id: &str) -> Result<()>;
    
    /// 批量导入模型
    async fn bulk_import(&self, req: BulkImportRequest) -> Result<BulkImportResponse>;
}

/// 创建模型请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateModelRequest {
    pub key: String,
    pub name: String,
    pub model_type: String,
    pub provider_id: String,
    pub api_endpoint: Option<String>,
    pub api_key: Option<String>,
    pub input_price: f64,
    pub output_price: f64,
    pub currency: Option<String>,
    pub max_tokens_per_request: Option<i32>,
    pub max_requests_per_minute: Option<i32>,
    pub enabled: Option<bool>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub capabilities: Option<Vec<String>>,
}

/// 列表模型请求
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ListModelsRequest {
    pub page: Option<i64>,
    pub size: Option<i64>,
    pub provider_id: Option<String>,
    pub model_type: Option<String>,
    pub enabled: Option<bool>,
    pub status: Option<String>,
    pub keyword: Option<String>,
}

/// 列表模型响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListModelsResponse {
    pub items: Vec<ModelDefinition>,
    pub total: i64,
    pub page: i64,
    pub size: i64,
}

/// 更新模型请求
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateModelRequest {
    pub name: Option<String>,
    pub api_endpoint: Option<String>,
    pub api_key: Option<String>,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub currency: Option<String>,
    pub max_tokens_per_request: Option<i32>,
    pub max_requests_per_minute: Option<i32>,
    pub enabled: Option<bool>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub capabilities: Option<Vec<String>>,
}

/// 批量导入请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkImportRequest {
    pub provider_id: String,
    pub models: Vec<BulkImportModel>,
}

/// 批量导入模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkImportModel {
    pub key: String,
    pub name: String,
    pub model_type: String,
    pub input_price: f64,
    pub output_price: f64,
    pub capabilities: Option<Vec<String>>,
}

/// 批量导入响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkImportResponse {
    pub imported: i32,
    pub failed: i32,
    pub errors: Vec<String>,
}

/// 模型定义服务实现
pub struct ModelDefinitionServiceImpl {
    encryption_service: EncryptionService,
}

impl ModelDefinitionServiceImpl {
    pub fn new(encryption_service: EncryptionService) -> Self {
        Self { encryption_service }
    }
}

#[async_trait]
impl ModelDefinitionService for ModelDefinitionServiceImpl {
    async fn create_model(&self, req: CreateModelRequest) -> Result<ModelDefinition> {
        // 验证必填字段
        if req.key.is_empty() {
            return Err(Error::ValidationError("Model key cannot be empty".to_string()));
        }
        if req.name.is_empty() {
            return Err(Error::ValidationError("Model name cannot be empty".to_string()));
        }
        if req.model_type.is_empty() {
            return Err(Error::ValidationError("Model type cannot be empty".to_string()));
        }
        if req.provider_id.is_empty() {
            return Err(Error::ValidationError("Provider ID cannot be empty".to_string()));
        }

        // 检查模型key是否已存在
        if let Ok(_) = self.get_model_by_key(&req.key).await {
            return Err(Error::ValidationError(format!("Model key '{}' already exists", req.key)));
        }

        // 加密API密钥（如果提供）
        let api_key_encrypted = if let Some(api_key) = req.api_key {
            Some(self.encryption_service.encrypt(&api_key)?)
        } else {
            None
        };

        // 转换能力列表为JSON
        let capabilities = req.capabilities.map(|caps| {
            JsonValue::Array(caps.into_iter().map(JsonValue::String).collect())
        });

        let model = ModelDefinition {
            id: Some(uuid::Uuid::new_v4().to_string()),
            key: req.key,
            name: req.name,
            model_type: req.model_type,
            provider_id: req.provider_id,
            api_endpoint: req.api_endpoint,
            api_key_encrypted,
            input_price: req.input_price,
            output_price: req.output_price,
            currency: req.currency,
            max_tokens_per_request: req.max_tokens_per_request,
            max_requests_per_minute: req.max_requests_per_minute,
            enabled: req.enabled,
            status: req.status,
            description: req.description,
            capabilities,
            created_at: Some(DateTime::now()),
            updated_at: Some(DateTime::now()),
        };

        // 保存到数据库 - 使用 RBatis 的 insert 方法
        let result = ModelDefinition::insert(pool!(), &model).await;
        match result {
            Ok(_) => Ok(model),
            Err(e) => Err(Error::DatabaseError(e.to_string())),
        }
    }

    async fn get_model(&self, id: &str) -> Result<ModelDefinition> {
        let sql = "SELECT * FROM model_definition WHERE id = ? LIMIT 1";
        let result = rbatis::RBatis::query(pool!(), sql, vec![rbs::Value::String(id.to_string())]).await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;
        
        let value = result.as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| Error::NotFound(format!("Model with id '{}' not found", id)))?;
        
        let json_value = serde_json::to_value(value).unwrap_or_default();
        serde_json::from_value(json_value)
            .map_err(|e| Error::DatabaseError(e.to_string()))
    }

    async fn get_model_by_key(&self, key: &str) -> Result<ModelDefinition> {
        let sql = "SELECT * FROM model_definition WHERE key = ? LIMIT 1";
        let result = rbatis::RBatis::query(pool!(), sql, vec![rbs::Value::String(key.to_string())]).await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;
        
        let value = result.as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| Error::NotFound(format!("Model with key '{}' not found", key)))?;
        
        let json_value = serde_json::to_value(value).unwrap_or_default();
        serde_json::from_value(json_value)
            .map_err(|e| Error::DatabaseError(e.to_string()))
    }

    async fn list_models(&self, req: ListModelsRequest) -> Result<ListModelsResponse> {
        let page = req.page.unwrap_or(1);
        let size = req.size.unwrap_or(20);
        let offset = (page - 1) * size;

        // 简化查询 - 使用 select_all 然后手动过滤
        let all_models: Vec<ModelDefinition> = ModelDefinition::select_all(pool!()).await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;

        let mut filtered_models: Vec<ModelDefinition> = Vec::new();

        for model in all_models {
            // 过滤供应商
            if let Some(provider_id) = &req.provider_id {
                if &model.provider_id != provider_id {
                    continue;
                }
            }

            // 过滤模型类型
            if let Some(model_type) = &req.model_type {
                if &model.model_type != model_type {
                    continue;
                }
            }

            // 过滤启用状态
            if let Some(enabled) = req.enabled {
                if model.enabled.unwrap_or(false) != enabled {
                    continue;
                }
            }

            // 过滤状态
            if let Some(status) = &req.status {
                if model.status.as_ref() != Some(status) {
                    continue;
                }
            }

            // 关键词搜索
            if let Some(keyword) = &req.keyword {
                let keyword_lower = keyword.to_lowercase();
                let key_match = model.key.to_lowercase().contains(&keyword_lower);
                let name_match = model.name.to_lowercase().contains(&keyword_lower);
                if !key_match && !name_match {
                    continue;
                }
            }

            filtered_models.push(model);
        }

        let total = filtered_models.len() as i64;

        // 分页
        let start = offset as usize;
        let end = std::cmp::min(start + size as usize, filtered_models.len());
        let items = if start < filtered_models.len() {
            filtered_models[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok(ListModelsResponse {
            items,
            total,
            page,
            size,
        })
    }

    async fn update_model(&self, id: &str, req: UpdateModelRequest) -> Result<ModelDefinition> {
        // 获取现有模型
        let mut model = self.get_model(id).await?;

        // 更新字段
        if let Some(name) = req.name {
            model.name = name;
        }
        if let Some(api_endpoint) = req.api_endpoint {
            model.api_endpoint = Some(api_endpoint);
        }
        if let Some(api_key) = req.api_key {
            model.api_key_encrypted = Some(self.encryption_service.encrypt(&api_key)?);
        }
        if let Some(input_price) = req.input_price {
            model.input_price = input_price;
        }
        if let Some(output_price) = req.output_price {
            model.output_price = output_price;
        }
        if let Some(currency) = req.currency {
            model.currency = Some(currency);
        }
        if let Some(max_tokens_per_request) = req.max_tokens_per_request {
            model.max_tokens_per_request = Some(max_tokens_per_request);
        }
        if let Some(max_requests_per_minute) = req.max_requests_per_minute {
            model.max_requests_per_minute = Some(max_requests_per_minute);
        }
        if let Some(enabled) = req.enabled {
            model.enabled = Some(enabled);
        }
        if let Some(status) = req.status {
            model.status = Some(status);
        }
        if let Some(description) = req.description {
            model.description = Some(description);
        }
        if let Some(capabilities) = req.capabilities {
            model.capabilities = Some(JsonValue::Array(
                capabilities.into_iter().map(JsonValue::String).collect()
            ));
        }

        model.updated_at = Some(DateTime::now());

        // 更新数据库
        let sql = "UPDATE model_definition SET name = ?, api_endpoint = ?, api_key_encrypted = ?, input_price = ?, output_price = ?, currency = ?, max_tokens_per_request = ?, max_requests_per_minute = ?, enabled = ?, status = ?, description = ?, capabilities = ?, updated_at = ? WHERE id = ?";
        rbatis::RBatis::exec(pool!(), sql, vec![
            rbs::Value::String(model.name.clone()),
            rbs::Value::String(model.api_endpoint.clone().unwrap_or_default()),
            rbs::Value::String(model.api_key_encrypted.clone().unwrap_or_default()),
            rbs::Value::F64(model.input_price),
            rbs::Value::F64(model.output_price),
            rbs::Value::String(model.currency.clone().unwrap_or_default()),
            rbs::Value::I32(model.max_tokens_per_request.unwrap_or_default()),
            rbs::Value::I32(model.max_requests_per_minute.unwrap_or_default()),
            rbs::Value::Bool(model.enabled.unwrap_or_default()),
            rbs::Value::String(model.status.clone().unwrap_or_default()),
            rbs::Value::String(model.description.clone().unwrap_or_default()),
            rbs::Value::String(model.capabilities.clone().unwrap_or_default().to_string()),
            rbs::Value::String(model.updated_at.clone().unwrap_or_default().to_string()),
            rbs::Value::String(model.id.clone().unwrap_or_default())
        ]).await.map_err(|e| Error::DatabaseError(e.to_string()))?;

        Ok(model)
    }

    async fn delete_model(&self, id: &str) -> Result<()> {
        let model = self.get_model(id).await?;
        let model_id = model.id.ok_or_else(|| Error::BusinessError("Model ID is missing".to_string()))?;
        let sql = "DELETE FROM model_definition WHERE id = ?";
        rbatis::RBatis::exec(pool!(), sql, vec![rbs::Value::String(model_id)])
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn bulk_import(&self, req: BulkImportRequest) -> Result<BulkImportResponse> {
        let mut imported = 0;
        let mut failed = 0;
        let mut errors = Vec::new();

        for model_req in req.models {
            let create_req = CreateModelRequest {
                key: model_req.key.clone(),
                name: model_req.name.clone(),
                model_type: model_req.model_type.clone(),
                provider_id: req.provider_id.clone(),
                api_endpoint: None,
                api_key: None,
                input_price: model_req.input_price,
                output_price: model_req.output_price,
                currency: Some("USD".to_string()),
                max_tokens_per_request: None,
                max_requests_per_minute: None,
                enabled: Some(true),
                status: Some("active".to_string()),
                description: None,
                capabilities: model_req.capabilities,
            };

            match self.create_model(create_req).await {
                Ok(_) => imported += 1,
                Err(e) => {
                    failed += 1;
                    errors.push(format!("Failed to import model '{}': {}", model_req.key, e));
                }
            }
        }

        Ok(BulkImportResponse {
            imported,
            failed,
            errors,
        })
    }
}