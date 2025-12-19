// 通用OpenAI兼容供应商实现，支持多种供应商类型
use async_trait::async_trait;
use axum::http::StatusCode;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::domain::dto::chat::{ChatCompletionRequest, ChatCompletionResponse};
use crate::domain::dto::completion::CompletionRequest;
use crate::domain::dto::embeddings::EmbeddingsRequest;
use crate::domain::dto::{ProviderConfig, ProviderType};
use crate::domain::vo::completion::CompletionResponse;
use crate::domain::vo::embeddings::EmbeddingsResponse;
use crate::providers::provider::Provider;

/// 通用聊天补全请求结构，支持多种供应商
#[derive(Serialize, Deserialize, Clone)]
struct CommonChatCompletionRequest {
    #[serde(flatten)]
    base: ChatCompletionRequest,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<String>,
}

impl From<ChatCompletionRequest> for CommonChatCompletionRequest {
    fn from(mut base: ChatCompletionRequest) -> Self {
        // 处理max_completion_tokens逻辑 - 如果提供了max_completion_tokens且>0，则使用它，否则回退到max_tokens
        base.max_completion_tokens = match (base.max_completion_tokens, base.max_tokens) {
            (Some(v), _) if v > 0 => Some(v),
            (_, Some(v)) if v > 0 => Some(v),
            _ => None,
        };

        base.max_tokens = None;

        // 移除reasoning字段，根据供应商类型决定是否添加reasoning_effort
        let _reasoning = base.reasoning.take();

        // 初始化reasoning_effort为None，根据供应商类型决定是否设置
        let reasoning_effort = None;

        Self {
            base,
            reasoning_effort,
        }
    }
}

/// 通用OpenAI兼容供应商实现，支持多种供应商类型
pub struct CommonProvider {
    config: ProviderConfig,
    http_client: Client,
}

impl CommonProvider {
    /// 获取API基础URL
    fn base_url(&self) -> String {
        let default_url = match self.config.provider_type {
            ProviderType::OpenAI => "https://api.openai.com/v1",
            ProviderType::Azure => "",
            ProviderType::Bedrock => "",
            _ => "https://api.openai.com/v1",
        };
        
        self.config.config_details
            .get("base_url")
            .and_then(|v| v.as_str())
            .unwrap_or(default_url)
            .to_string()
    }
    
    /// 获取API密钥
    fn api_key(&self) -> String {
        self.config.config_details
            .get("api_key")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    }
    
    /// 获取API密钥类型
    fn auth_type(&self) -> String {
        self.config.config_details
            .get("auth_type")
            .and_then(|v| v.as_str())
            .unwrap_or("bearer")
            .to_lowercase()
    }
    
    /// 获取部署名称（Azure专用）
    fn deployment_name(&self) -> String {
        self.config.config_details
            .get("deployment_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    }
    
    /// 构建请求URL，根据不同供应商类型处理不同的URL格式
    fn build_request_url(&self, endpoint: &str) -> String {
        match self.config.provider_type {
            ProviderType::Azure => {
                // Azure格式：{base_url}/openai/deployments/{deployment_name}/{endpoint}?api-version=2024-02-15-preview
                format!("{}/openai/deployments/{}/{}/?api-version=2024-02-15-preview", 
                       self.base_url(), self.deployment_name(), endpoint)
            },
            _ => {
                // 通用格式：{base_url}/{endpoint}
                format!("{}/{}", self.base_url(), endpoint)
            },
        }
    }
    
    /// 添加认证头，根据不同供应商类型处理不同的认证方式
    fn add_auth_headers(&self, request_builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let api_key = self.api_key();
        
        match self.config.provider_type {
            ProviderType::Azure => {
                // Azure使用api-key认证
                request_builder.header("api-key", api_key)
            },
            ProviderType::OpenAI | ProviderType::Custom => {
                // OpenAI和Custom使用Bearer认证
                request_builder.header("Authorization", format!("Bearer {}", api_key))
            },
            _ => {
                // 其他供应商根据auth_type处理
                match self.auth_type().as_str() {
                    "bearer" => {
                        request_builder.header("Authorization", format!("Bearer {}", api_key))
                    },
                    "api_key" => {
                        request_builder.header("X-API-Key", api_key)
                    },
                    _ => {
                        // 默认使用bearer认证
                        request_builder.header("Authorization", format!("Bearer {}", api_key))
                    },
                }
            },
        }
    }
}

#[async_trait]
impl Provider for CommonProvider {
    /// 创建通用供应商实例
    fn new(config: &ProviderConfig) -> Self {
        Self {
            config: config.clone(),
            http_client: Client::new(),
        }
    }
    
    /// 获取供应商ID
    fn id(&self) -> String {
        self.config.id.clone()
    }
    
    /// 获取供应商名称
    fn name(&self) -> String {
        self.config.name.clone()
    }
    
    /// 获取供应商类型
    fn r#type(&self) -> ProviderType {
        self.config.provider_type.clone()
    }
    
    /// 聊天补全方法
    async fn chat_completions(
        &self,
        payload: ChatCompletionRequest,
        _model_config: &serde_json::Value,
    ) -> Result<ChatCompletionResponse, StatusCode> {
        // 转换为通用请求格式
        let common_request = CommonChatCompletionRequest::from(payload.clone());

        let request_builder = self
            .http_client
            .post(self.build_request_url("chat/completions"));
        
        // 添加认证头
        let request_builder = self.add_auth_headers(request_builder);

        let response = request_builder
            .json(&common_request)
            .send()
            .await
            .map_err(|e| {
                error!("{} provider API request error: {e}", self.r#type().to_string());
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let status = response.status();
        if status.is_success() {
            // 检查是否为流式请求
            if payload.stream.unwrap_or(false) {
                // 暂时不支持流式响应，后续再实现
                error!("{} streaming response not supported yet", self.r#type().to_string());
                Err(StatusCode::NOT_IMPLEMENTED)
            } else {
                // 获取响应内容作为JSON
                let chat_response: crate::domain::vo::chat::ChatCompletion = response
                    .json()
                    .await
                    .map_err(|e| {
                        error!("{} provider API response error: {e}", self.r#type().to_string());
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;
                
                // 返回非流式响应
                Ok(ChatCompletionResponse::NonStream(chat_response))
            }
        } else {
            let error_msg = response.text().await.unwrap_or_default();
            info!("{} provider API request error: {}", self.r#type().to_string(), error_msg);
            Err(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
        }
    }

    /// 文本补全方法
    async fn completions(
        &self,
        payload: CompletionRequest,
        _model_config: &serde_json::Value,
    ) -> Result<CompletionResponse, StatusCode> {
        let request_builder = self
            .http_client
            .post(self.build_request_url("completions"));
        
        // 添加认证头
        let request_builder = self.add_auth_headers(request_builder);

        let response = request_builder
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                error!("{} provider API request error: {e}", self.r#type().to_string());
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let status = response.status();
        if status.is_success() {
            response.json().await.map_err(|e| {
                error!("{} provider API response error: {e}", self.r#type().to_string());
                StatusCode::INTERNAL_SERVER_ERROR
            })
        } else {
            let error_msg = response.text().await.unwrap_or_default();
            error!("{} provider API request error: {}", self.r#type().to_string(), error_msg);
            Err(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
        }
    }

    /// 嵌入生成方法
    async fn embeddings(
        &self,
        payload: EmbeddingsRequest,
        _model_config: &serde_json::Value,
    ) -> Result<EmbeddingsResponse, StatusCode> {
        let request_builder = self
            .http_client
            .post(self.build_request_url("embeddings"));
        
        // 添加认证头
        let request_builder = self.add_auth_headers(request_builder);

        let response = request_builder
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                error!("{} provider API request error: {e}", self.r#type().to_string());
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let status = response.status();
        if status.is_success() {
            response.json().await.map_err(|e| {
                error!("{} provider API response error: {e}", self.r#type().to_string());
                StatusCode::INTERNAL_SERVER_ERROR
            })
        } else {
            let error_msg = response.text().await.unwrap_or_default();
            error!("{} provider API request error: {}", self.r#type().to_string(), error_msg);
            Err(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}


