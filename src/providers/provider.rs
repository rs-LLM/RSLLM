use async_trait::async_trait;
use axum::http::StatusCode;
use serde_json::Value;

use crate::domain::dto::chat::{ChatCompletionRequest, ChatCompletionResponse};
use crate::domain::dto::embeddings::EmbeddingsRequest;
use crate::domain::dto::{ProviderConfig, ProviderType};
use crate::domain::vo::embeddings::EmbeddingsResponse;

/// 供应商核心trait，定义供应商必须实现的方法
#[async_trait]
pub trait Provider: Send + Sync {
    /// 创建供应商实例
    fn new(config: &ProviderConfig) -> Self
    where
        Self: Sized;

    /// 获取供应商ID
    fn id(&self) -> String;

    /// 获取供应商名称
    fn name(&self) -> String;

    /// 获取供应商类型
    fn r#type(&self) -> ProviderType;

    /// 设置解密后的API密钥
    fn set_decrypted_api_key(&mut self, decrypted_key: String);

    /// 聊天补全方法
    async fn chat_completions(
        &self,
        payload: ChatCompletionRequest,
        model_config: &Value,
    ) -> Result<ChatCompletionResponse, StatusCode>;

    /// 嵌入生成方法
    async fn embeddings(
        &self,
        payload: EmbeddingsRequest,
        model_config: &Value,
    ) -> Result<EmbeddingsResponse, StatusCode>;
}
