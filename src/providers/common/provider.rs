// 通用OpenAI兼容供应商实现，支持多种供应商类型
use async_trait::async_trait;
use axum::http::StatusCode;
use futures::stream::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info};

use crate::domain::dto::chat::{ChatCompletionRequest, ChatCompletionResponse};
use crate::domain::dto::embeddings::EmbeddingsRequest;
use crate::domain::dto::{ProviderConfig, ProviderType};
use crate::domain::vo::embeddings::EmbeddingsResponse;
use crate::providers::provider::Provider;

/// 通用聊天补全请求结构，支持多种供应商
#[derive(Deserialize, Clone)]
struct CommonChatCompletionRequest {
    base: ChatCompletionRequest,
    reasoning_effort: Option<String>,
    extra_body: Option<serde_json::Value>,
}

impl Serialize for CommonChatCompletionRequest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(None)?;

        // 序列化 base 中的所有字段
        let base_json = serde_json::to_value(&self.base).map_err(serde::ser::Error::custom)?;
        if let serde_json::Value::Object(base_map) = base_json {
            for (key, value) in base_map {
                if !value.is_null() {
                    map.serialize_entry(&key, &value)?;
                }
            }
        }

        // 序列化 reasoning_effort
        if let Some(ref effort) = self.reasoning_effort {
            map.serialize_entry("reasoning_effort", effort)?;
        }

        // 合并 extra_body 中的所有字段到顶层
        if let Some(ref extra) = self.extra_body {
            if let serde_json::Value::Object(extra_map) = extra {
                for (key, value) in extra_map {
                    if !value.is_null() {
                        map.serialize_entry(&key, &value)?;
                    }
                }
            }
        }

        map.end()
    }
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

        // 提取extra_body用于透传
        let mut extra_body = base.extra_body.take();

        // 处理 thinking_budget 参数，支持字符串和数字
        if let Some(extra) = &mut extra_body {
            if let serde_json::Value::Object(map) = extra {
                if let Some(thinking_budget) = map.remove("thinking_budget") {
                    let converted_budget = match thinking_budget {
                        // 如果是字符串，转换为对应的数字
                        serde_json::Value::String(s) => match s.as_str() {
                            "high" => Some(serde_json::Value::Number(1024.into())),
                            "medium" => Some(serde_json::Value::Number(512.into())),
                            "low" => Some(serde_json::Value::Number(256.into())),
                            _ => None,
                        },
                        // 如果已经是数字，保持不变
                        serde_json::Value::Number(_) => Some(thinking_budget),
                        _ => None,
                    };

                    if let Some(budget) = converted_budget {
                        map.insert("thinking_budget".to_string(), budget);
                    }
                }
            }
        }

        Self {
            base,
            reasoning_effort,
            extra_body,
        }
    }
}

/// 通用OpenAI兼容供应商实现，支持多种供应商类型
pub struct CommonProvider {
    config: ProviderConfig,
    http_client: Client,
    decrypted_api_key: Option<String>,
}

impl CommonProvider {
    /// 获取API基础URL
    fn base_url(&self) -> String {
        if !self.config.api_base.is_empty() {
            return self.config.api_base.clone();
        }

        let default_url = match self.config.provider_type {
            ProviderType::OpenAI => "https://api.openai.com/v1",
            _ => "https://api.openai.com/v1",
        };

        default_url.to_string()
    }

    /// 获取API密钥
    fn api_key(&self) -> String {
        // 优先使用已解密的API密钥
        if let Some(ref decrypted_key) = self.decrypted_api_key {
            log::info!(
                "[Provider] Using decrypted API key for provider {} (length: {})",
                self.config.provider_code,
                decrypted_key.len()
            );
            return decrypted_key.clone();
        }

        // 如果没有解密的密钥，尝试从加密密钥中获取（但这不应该发生）
        if let Some(ref encrypted_key) = self.config.api_key_encrypted {
            log::warn!(
                "API key not decrypted for provider {}, using encrypted key",
                self.config.id
            );
            return encrypted_key.clone();
        }

        let key = self
            .config
            .config_details
            .get("api_key")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        log::warn!(
            "Using API key from config_details for provider {} (length: {})",
            self.config.provider_code,
            key.len()
        );
        key
    }

    /// 添加认证头，OpenAI和OpenAI Compatible都使用Bearer Token认证
    fn add_auth_headers(
        &self,
        request_builder: reqwest::RequestBuilder,
    ) -> reqwest::RequestBuilder {
        let api_key = self.api_key();

        log::info!(
            "[Provider] Adding Authorization header for provider {} (API key length: {})",
            self.config.provider_code,
            api_key.len()
        );

        // OpenAI和OpenAI Compatible都使用Bearer Token认证
        request_builder.header("Authorization", format!("Bearer {}", api_key))
    }

    /// 构建请求URL
    fn build_request_url(&self, endpoint: &str) -> String {
        format!("{}/{}", self.base_url(), endpoint)
    }
}

#[async_trait]
impl Provider for CommonProvider {
    /// 创建通用供应商实例
    fn new(config: &ProviderConfig) -> Self {
        let timeout_seconds = config
            .config_details
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(120);

        let timeout = Duration::from_secs(timeout_seconds);

        let http_client = Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            config: config.clone(),
            http_client,
            decrypted_api_key: None,
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

    /// 设置解密后的API密钥
    fn set_decrypted_api_key(&mut self, decrypted_key: String) {
        self.decrypted_api_key = Some(decrypted_key);
    }

    /// 聊天补全方法
    async fn chat_completions(
        &self,
        payload: ChatCompletionRequest,
        _model_config: &serde_json::Value,
    ) -> Result<ChatCompletionResponse, StatusCode> {
        // 转换为通用请求格式
        let common_request = CommonChatCompletionRequest::from(payload.clone());

        let request_url = self.build_request_url("chat/completions");
        log::info!(
            "[Provider] Sending chat completion request to: {}",
            request_url
        );
        log::info!("[Provider] Request model: {}", payload.model);

        let request_builder = self.http_client.post(&request_url);

        // 添加认证头
        let request_builder = self.add_auth_headers(request_builder);

        log::info!(
            "[Provider] Sending request with model: {}",
            common_request.base.model
        );
        
        // 打印完整的请求体用于调试
        let request_json = serde_json::to_string(&common_request).unwrap_or_default();
        log::info!("[Provider] Full request body: {}", request_json);

        let response = request_builder
            .json(&common_request)
            .send()
            .await
            .map_err(|e| {
                error!(
                    "{} provider API request error: {e}",
                    self.r#type().to_string()
                );
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let status = response.status();
        log::info!("[Provider] Response status: {}", status);

        if !status.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error body".to_string());
            log::error!("[Provider] Error response body: {}", error_body);
            return Err(
                StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
            );
        }

        if status.is_success() {
            // 检查是否为流式请求
            if payload.stream.unwrap_or(false) {
                // 支持流式响应
                info!("{} streaming response enabled", self.r#type().to_string());

                // 获取字节流
                let byte_stream = response.bytes_stream();

                // 将字节流转换为 SSE 事件流
                let stream = byte_stream
                    .map(move |bytes| {
                        match bytes {
                            Ok(chunk) => {
                                // 将字节块转换为字符串
                                let text = String::from_utf8_lossy(&chunk);
                                info!("Raw received data: {}", text);
                                let mut chunks = Vec::new();
                                // 解析 SSE 格式的数据，收集所有事件
                                for line in text.lines() {
                                    if let Some(data) = line.strip_prefix("data: ") {
                                        // 去掉 "data: " 前缀
                                        if data == "[DONE]" {
                                            // 流结束，返回一个空的 chunk 表示结束
                                            chunks.push(Ok(crate::domain::dto::ai_hub::streaming::ChatCompletionChunk {
                                                id: String::new(),
                                                object: Some("chat.completion.chunk".to_string()),
                                                created: 0,
                                                model: String::new(),
                                                choices: vec![],
                                                system_fingerprint: None,
                                                extra_fields: serde_json::Value::default(),
                                            }));
                                        } else {
                                            // 解析 JSON
                                            if let Ok(chunk) = serde_json::from_str::<crate::domain::dto::ai_hub::streaming::ChatCompletionChunk>(data) {
                                                // 调试日志：打印接收到的chunk
                                                info!("Received chunk: {}", serde_json::to_string(&chunk).unwrap_or_default());
                                                
                                                // 过滤空内容的chunk，但要保留包含额外字段的chunk
                                                let should_include = chunk.choices.iter().any(|choice| {
                                                    if let Some(ref delta) = choice.delta {
                                                        // 检查content是否非空
                                                        let has_content = delta.content.as_ref().map_or(false, |s| !s.is_empty());
                                                        
                                                        // 检查reasoning_content是否非空
                                                        let has_reasoning = delta.reasoning_content.as_ref().map_or(false, |s| !s.is_empty());
                                                        
                                                        // 检查extra_fields是否非空
                                                        let has_extra_fields = !delta.extra_fields.is_null() && !delta.extra_fields.as_object().map_or(true, |obj| obj.is_empty());
                                                        
                                                        // 检查是否有tool_calls
                                                        let has_tool_calls = delta.tool_calls.as_ref().map_or(false, |calls| !calls.is_empty());
                                                        
                                                        has_content || has_reasoning || has_extra_fields || has_tool_calls
                                                    } else {
                                                        false
                                                    }
                                                }) || chunk.choices.iter().any(|choice| choice.finish_reason.is_some()) || !chunk.extra_fields.is_null() && !chunk.extra_fields.as_object().map_or(true, |obj| obj.is_empty());
                                                
                                                info!("Chunk should_include: {}", should_include);
                                                
                                                if should_include {
                                                    chunks.push(Ok(chunk));
                                                }
                                            }
                                        }
                                    }
                                }
                                chunks
                            }
                            Err(e) => vec![Err(e)],
                        }
                    })
                    .flat_map(futures::stream::iter);

                // 返回流式响应
                Ok(ChatCompletionResponse::Stream(Box::pin(stream)))
            } else {
                // 获取响应内容作为JSON
                let chat_response: crate::domain::vo::chat::ChatCompletion =
                    response.json().await.map_err(|e| {
                        error!(
                            "{} provider API response error: {e}",
                            self.r#type().to_string()
                        );
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;

                // 返回非流式响应
                Ok(ChatCompletionResponse::NonStream(chat_response))
            }
        } else {
            let error_msg = response.text().await.unwrap_or_default();
            info!(
                "{} provider API request error: {}",
                self.r#type().to_string(),
                error_msg
            );
            Err(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
        }
    }

    /// 嵌入生成方法
    async fn embeddings(
        &self,
        payload: EmbeddingsRequest,
        _model_config: &serde_json::Value,
    ) -> Result<EmbeddingsResponse, StatusCode> {
        let request_builder = self.http_client.post(self.build_request_url("embeddings"));

        // 添加认证头
        let request_builder = self.add_auth_headers(request_builder);

        let response = request_builder.json(&payload).send().await.map_err(|e| {
            error!(
                "{} provider API request error: {e}",
                self.r#type().to_string()
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        let status = response.status();
        if status.is_success() {
            response.json().await.map_err(|e| {
                error!(
                    "{} provider API response error: {e}",
                    self.r#type().to_string()
                );
                StatusCode::INTERNAL_SERVER_ERROR
            })
        } else {
            let error_msg = response.text().await.unwrap_or_default();
            error!(
                "{} provider API request error: {}",
                self.r#type().to_string(),
                error_msg
            );
            Err(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dto::ai_hub::content::ChatCompletionMessage;

    #[test]
    fn test_common_chat_completion_request_serialization_with_extra_body() {
        use crate::domain::dto::ai_hub::content::ChatMessageContent;

        let request = ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some(ChatMessageContent::String("Hello".to_string())),
                name: None,
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                extra_fields: serde_json::Value::default(),
            }],
            stream: Some(false),
            temperature: Some(0.7),
            max_tokens: Some(100),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            user: None,
            logprobs: None,
            top_logprobs: None,
            n: None,
            max_completion_tokens: None,
            parallel_tool_calls: None,
            logit_bias: None,
            reasoning: None,
            extra_body: Some(serde_json::json!({
                "thinking_budget": 1000,
                "custom_param": "value"
            })),
            extra_fields: serde_json::Value::default(),
        };

        let common_request = CommonChatCompletionRequest::from(request);

        let serialized = serde_json::to_string(&common_request).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized["model"], "test-model");
        assert_eq!(deserialized["thinking_budget"], 1000);
        assert_eq!(deserialized["custom_param"], "value");
        assert_eq!(deserialized["messages"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_common_chat_completion_request_without_extra_body() {
        use crate::domain::dto::ai_hub::content::ChatMessageContent;

        let request = ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some(ChatMessageContent::String("Hello".to_string())),
                name: None,
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                extra_fields: serde_json::Value::default(),
            }],
            stream: Some(false),
            temperature: Some(0.7),
            max_tokens: Some(100),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            user: None,
            logprobs: None,
            top_logprobs: None,
            n: None,
            max_completion_tokens: None,
            parallel_tool_calls: None,
            logit_bias: None,
            reasoning: None,
            extra_body: None,
            extra_fields: serde_json::Value::default(),
        };

        let common_request = CommonChatCompletionRequest::from(request);

        let serialized = serde_json::to_string(&common_request).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized["model"], "test-model");
        assert!(
            deserialized.get("thinking_budget").is_none(),
            "thinking_budget should not be present when extra_body is None"
        );
        assert_eq!(deserialized["messages"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_common_chat_completion_request_extra_body_override() {
        use crate::domain::dto::ai_hub::content::ChatMessageContent;

        let request = ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some(ChatMessageContent::String("Hello".to_string())),
                name: None,
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                extra_fields: serde_json::Value::default(),
            }],
            stream: Some(false),
            temperature: Some(0.7),
            max_tokens: Some(100),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            user: None,
            logprobs: None,
            top_logprobs: None,
            n: None,
            max_completion_tokens: None,
            parallel_tool_calls: None,
            logit_bias: None,
            reasoning: None,
            extra_body: Some(serde_json::json!({
                "temperature": 1.0,
                "max_tokens": 200
            })),
            extra_fields: serde_json::Value::default(),
        };

        let common_request = CommonChatCompletionRequest::from(request);

        let serialized = serde_json::to_string(&common_request).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            deserialized["temperature"], 1.0,
            "extra_body should override base temperature"
        );
        assert_eq!(
            deserialized["max_tokens"], 200,
            "extra_body should override base max_tokens"
        );
    }

    #[test]
    fn test_thinking_budget_string_conversion_high() {
        use crate::domain::dto::ai_hub::content::ChatMessageContent;

        let request = ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some(ChatMessageContent::String("Hello".to_string())),
                name: None,
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                extra_fields: serde_json::Value::default(),
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            user: None,
            logprobs: None,
            top_logprobs: None,
            n: None,
            max_completion_tokens: None,
            parallel_tool_calls: None,
            logit_bias: None,
            reasoning: None,
            extra_body: Some(serde_json::json!({
                "thinking_budget": "high"
            })),
            extra_fields: serde_json::Value::default(),
        };

        let common_request = CommonChatCompletionRequest::from(request);

        let serialized = serde_json::to_string(&common_request).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            deserialized["thinking_budget"], 1024,
            "thinking_budget 'high' should be converted to 1024"
        );
    }

    #[test]
    fn test_thinking_budget_string_conversion_medium() {
        use crate::domain::dto::ai_hub::content::ChatMessageContent;

        let request = ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some(ChatMessageContent::String("Hello".to_string())),
                name: None,
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                extra_fields: serde_json::Value::default(),
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            user: None,
            logprobs: None,
            top_logprobs: None,
            n: None,
            max_completion_tokens: None,
            parallel_tool_calls: None,
            logit_bias: None,
            reasoning: None,
            extra_body: Some(serde_json::json!({
                "thinking_budget": "medium"
            })),
            extra_fields: serde_json::Value::default(),
        };

        let common_request = CommonChatCompletionRequest::from(request);

        let serialized = serde_json::to_string(&common_request).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            deserialized["thinking_budget"], 512,
            "thinking_budget 'medium' should be converted to 512"
        );
    }

    #[test]
    fn test_thinking_budget_string_conversion_low() {
        use crate::domain::dto::ai_hub::content::ChatMessageContent;

        let request = ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some(ChatMessageContent::String("Hello".to_string())),
                name: None,
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                extra_fields: serde_json::Value::default(),
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            user: None,
            logprobs: None,
            top_logprobs: None,
            n: None,
            max_completion_tokens: None,
            parallel_tool_calls: None,
            logit_bias: None,
            reasoning: None,
            extra_body: Some(serde_json::json!({
                "thinking_budget": "low"
            })),
            extra_fields: serde_json::Value::default(),
        };

        let common_request = CommonChatCompletionRequest::from(request);

        let serialized = serde_json::to_string(&common_request).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            deserialized["thinking_budget"], 256,
            "thinking_budget 'low' should be converted to 256"
        );
    }

    #[test]
    fn test_thinking_budget_number_unchanged() {
        use crate::domain::dto::ai_hub::content::ChatMessageContent;

        let request = ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some(ChatMessageContent::String("Hello".to_string())),
                name: None,
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                extra_fields: serde_json::Value::default(),
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            user: None,
            logprobs: None,
            top_logprobs: None,
            n: None,
            max_completion_tokens: None,
            parallel_tool_calls: None,
            logit_bias: None,
            reasoning: None,
            extra_body: Some(serde_json::json!({
                "thinking_budget": 2048
            })),
            extra_fields: serde_json::Value::default(),
        };

        let common_request = CommonChatCompletionRequest::from(request);

        let serialized = serde_json::to_string(&common_request).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            deserialized["thinking_budget"], 2048,
            "thinking_budget number should remain unchanged"
        );
    }

    #[test]
    fn test_thinking_budget_invalid_string_removed() {
        use crate::domain::dto::ai_hub::content::ChatMessageContent;

        let request = ChatCompletionRequest {
            model: "test-model".to_string(),
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some(ChatMessageContent::String("Hello".to_string())),
                name: None,
                reasoning_content: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                extra_fields: serde_json::Value::default(),
            }],
            stream: Some(false),
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            user: None,
            logprobs: None,
            top_logprobs: None,
            n: None,
            max_completion_tokens: None,
            parallel_tool_calls: None,
            logit_bias: None,
            reasoning: None,
            extra_body: Some(serde_json::json!({
                "thinking_budget": "invalid"
            })),
            extra_fields: serde_json::Value::default(),
        };

        let common_request = CommonChatCompletionRequest::from(request);

        let serialized = serde_json::to_string(&common_request).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert!(
            deserialized.get("thinking_budget").is_none(),
            "thinking_budget should be removed for invalid string"
        );
    }
}
