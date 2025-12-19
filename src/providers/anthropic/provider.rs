// Anthropic供应商实现
use async_trait::async_trait;
use axum::http::StatusCode;
use reqwest::Client;
use tracing::error;

use crate::domain::dto::chat::{ChatCompletionRequest, ChatCompletionResponse};
use crate::domain::dto::completion::CompletionRequest;
use crate::domain::dto::embeddings::EmbeddingsRequest;
use crate::domain::dto::{ProviderConfig, ProviderType};
use crate::domain::vo::completion::CompletionResponse;
use crate::domain::vo::embeddings::EmbeddingsResponse;
use crate::providers::provider::Provider;

/// Anthropic供应商实现
pub struct AnthropicProvider {
    config: ProviderConfig,
    _http_client: Client,
}



#[async_trait]
impl Provider for AnthropicProvider {
    /// 创建Anthropic供应商实例
    fn new(config: &ProviderConfig) -> Self {
        Self {
            config: config.clone(),
            _http_client: Client::new(),
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
        ProviderType::Anthropic
    }
    
    /// 聊天补全方法
    async fn chat_completions(
        &self,
        _payload: ChatCompletionRequest,
        _model_config: &serde_json::Value,
    ) -> Result<ChatCompletionResponse, StatusCode> {
        // 简单实现，后续可扩展为完整的Anthropic API调用
        error!("Anthropic chat_completions not fully implemented yet");
        Err(StatusCode::NOT_IMPLEMENTED)
    }
    
    /// 文本补全方法
    async fn completions(
        &self,
        _payload: CompletionRequest,
        _model_config: &serde_json::Value,
    ) -> Result<CompletionResponse, StatusCode> {
        // 简单实现，后续可扩展为完整的Anthropic API调用
        error!("Anthropic completions not fully implemented yet");
        Err(StatusCode::NOT_IMPLEMENTED)
    }
    
    /// 嵌入生成方法
    async fn embeddings(
        &self,
        _payload: EmbeddingsRequest,
        _model_config: &serde_json::Value,
    ) -> Result<EmbeddingsResponse, StatusCode> {
        // 简单实现，后续可扩展为完整的Anthropic API调用
        error!("Anthropic embeddings not fully implemented yet");
        Err(StatusCode::NOT_IMPLEMENTED)
    }
}
