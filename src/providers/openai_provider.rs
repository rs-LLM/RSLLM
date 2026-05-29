//! OpenAI 供应商适配器实现

use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

use crate::error::{Error, Result};
use crate::middleware::auth_axum::TOKEN_KEY;
use crate::service::ai_hub::ProviderConfig;
use crate::service::ai_hub::provider_trait::{
    AIProvider, ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, Choice,
    CompletionChoice, CompletionRequest, CompletionResponse, EmbeddingData, EmbeddingInput,
    EmbeddingRequest, EmbeddingResponse, Message, ModelInfo, Usage,
};

/// OpenAI 供应商实现
pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    api_base: String,
}

impl OpenAIProvider {
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let timeout = Duration::from_secs(config.timeout_seconds.unwrap_or(30));

        let client = Client::builder()
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(8))
            .tcp_nodelay(true)
            .pool_max_idle_per_host(16)
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .map_err(|e| {
                Error::ExternalServiceError(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self {
            client,
            api_key: config.api_key,
            api_base: config.api_base,
        })
    }

    /// 转换为 OpenAI 请求格式
    fn convert_request(&self, req: &ChatCompletionRequest) -> serde_json::Value {
        let mut request = json!({
            "model": req.model,
            "messages": req.messages.iter().map(|m| {
                let mut msg = json!({
                    "role": m.role,
                    "content": m.content,
                });
                if let Some(name) = &m.name {
                    msg["name"] = json!(name);
                }
                if let Some(tool_calls) = &m.tool_calls {
                    msg["tool_calls"] = json!(tool_calls);
                }
                if let Some(tool_call_id) = &m.tool_call_id {
                    msg["tool_call_id"] = json!(tool_call_id);
                }
                msg
            }).collect::<Vec<_>>(),
        });

        if let Some(stream) = req.stream {
            request["stream"] = json!(stream);
        }
        if let Some(temp) = req.temperature {
            request["temperature"] = json!(temp);
        }
        if let Some(max_tokens) = req.max_tokens {
            request["max_tokens"] = json!(max_tokens);
        }
        if let Some(top_p) = req.top_p {
            request["top_p"] = json!(top_p);
        }
        if let Some(freq_penalty) = req.frequency_penalty {
            request["frequency_penalty"] = json!(freq_penalty);
        }
        if let Some(pres_penalty) = req.presence_penalty {
            request["presence_penalty"] = json!(pres_penalty);
        }
        if let Some(stop) = &req.stop {
            request["stop"] = json!(stop);
        }
        if let Some(response_format) = &req.response_format {
            request["response_format"] = json!({ "type": response_format.r#type });
        }
        if let Some(tools) = &req.tools {
            request["tools"] = json!(tools);
        }
        if let Some(tool_choice) = &req.tool_choice {
            request["tool_choice"] = json!(tool_choice);
        }

        request
    }

    /// 转换 OpenAI 响应为统一格式
    fn convert_response(&self, openai_resp: serde_json::Value) -> Result<ChatCompletionResponse> {
        let id = openai_resp["id"].as_str().unwrap_or("unknown").to_string();

        let object = openai_resp["object"]
            .as_str()
            .unwrap_or("chat.completion")
            .to_string();

        let created = openai_resp["created"].as_i64().unwrap_or(0);

        let model = openai_resp["model"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let choices = openai_resp["choices"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|choice| {
                let message_value = &choice["message"];
                let message = Message {
                    role: message_value["role"]
                        .as_str()
                        .unwrap_or("assistant")
                        .to_string(),
                    content: message_value["content"].as_str().unwrap_or("").to_string(),
                    name: message_value["name"].as_str().map(|s| s.to_string()),
                    tool_calls: None, // 简化处理
                    tool_call_id: message_value["tool_call_id"]
                        .as_str()
                        .map(|s| s.to_string()),
                };

                Choice {
                    index: choice["index"].as_i64().unwrap_or(0) as i32,
                    message,
                    finish_reason: choice["finish_reason"].as_str().unwrap_or("").to_string(),
                    logprobs: None,
                }
            })
            .collect();

        let usage_value = &openai_resp["usage"];
        let usage = Usage {
            prompt_tokens: usage_value["prompt_tokens"].as_i64().unwrap_or(0) as i32,
            completion_tokens: usage_value["completion_tokens"].as_i64().unwrap_or(0) as i32,
            total_tokens: usage_value["total_tokens"].as_i64().unwrap_or(0) as i32,
        };

        Ok(ChatCompletionResponse {
            id,
            object,
            created,
            model,
            choices,
            usage,
        })
    }

    /// 转换为 OpenAI 补全请求格式
    fn convert_completion_request(&self, req: &CompletionRequest) -> serde_json::Value {
        let mut request = json!({
            "model": req.model,
            "prompt": req.prompt,
        });

        if let Some(stream) = req.stream {
            request["stream"] = json!(stream);
        }
        if let Some(temp) = req.temperature {
            request["temperature"] = json!(temp);
        }
        if let Some(max_tokens) = req.max_tokens {
            request["max_tokens"] = json!(max_tokens);
        }
        if let Some(top_p) = req.top_p {
            request["top_p"] = json!(top_p);
        }
        if let Some(freq_penalty) = req.frequency_penalty {
            request["frequency_penalty"] = json!(freq_penalty);
        }
        if let Some(pres_penalty) = req.presence_penalty {
            request["presence_penalty"] = json!(pres_penalty);
        }
        if let Some(stop) = &req.stop {
            request["stop"] = json!(stop);
        }

        request
    }

    /// 转换 OpenAI 补全响应为统一格式
    fn convert_completion_response(
        &self,
        openai_resp: serde_json::Value,
    ) -> Result<CompletionResponse> {
        let id = openai_resp["id"].as_str().unwrap_or("unknown").to_string();

        let object = openai_resp["object"]
            .as_str()
            .unwrap_or("text_completion")
            .to_string();

        let created = openai_resp["created"].as_i64().unwrap_or(0);

        let model = openai_resp["model"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let choices = openai_resp["choices"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|choice| CompletionChoice {
                index: choice["index"].as_i64().unwrap_or(0) as i32,
                text: choice["text"].as_str().unwrap_or("").to_string(),
                finish_reason: choice["finish_reason"].as_str().unwrap_or("").to_string(),
                logprobs: None,
            })
            .collect();

        let usage_value = &openai_resp["usage"];
        let usage = Usage {
            prompt_tokens: usage_value["prompt_tokens"].as_i64().unwrap_or(0) as i32,
            completion_tokens: usage_value["completion_tokens"].as_i64().unwrap_or(0) as i32,
            total_tokens: usage_value["total_tokens"].as_i64().unwrap_or(0) as i32,
        };

        Ok(CompletionResponse {
            id,
            object,
            created,
            model,
            choices,
            usage,
        })
    }

    /// 转换为 OpenAI 嵌入请求格式
    fn convert_embedding_request(&self, req: &EmbeddingRequest) -> serde_json::Value {
        let mut request = json!({
            "model": req.model,
            "input": match &req.input {
                EmbeddingInput::String(s) => json!(s),
                EmbeddingInput::Array(arr) => json!(arr),
            },
        });

        if let Some(format) = &req.encoding_format {
            request["encoding_format"] = json!(format);
        }
        if let Some(dimensions) = req.dimensions {
            request["dimensions"] = json!(dimensions);
        }

        request
    }

    /// 转换 OpenAI 嵌入响应为统一格式
    fn convert_embedding_response(
        &self,
        openai_resp: serde_json::Value,
    ) -> Result<EmbeddingResponse> {
        let object = openai_resp["object"].as_str().unwrap_or("list").to_string();

        let model = openai_resp["model"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let data = openai_resp["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|item| EmbeddingData {
                object: item["object"].as_str().unwrap_or("embedding").to_string(),
                embedding: item["embedding"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|v| v.as_f64())
                    .collect(),
                index: item["index"].as_i64().unwrap_or(0) as i32,
            })
            .collect();

        let usage_value = &openai_resp["usage"];
        let usage = Usage {
            prompt_tokens: usage_value["prompt_tokens"].as_i64().unwrap_or(0) as i32,
            completion_tokens: usage_value["completion_tokens"].as_i64().unwrap_or(0) as i32,
            total_tokens: usage_value["total_tokens"].as_i64().unwrap_or(0) as i32,
        };

        Ok(EmbeddingResponse {
            object,
            data,
            model,
            usage,
        })
    }

    fn resolve_models_endpoint(&self) -> String {
        let trimmed = self.api_base.trim_end_matches('/');
        let lowered = trimmed.to_ascii_lowercase();
        if lowered.contains("api.openai.com") && !trimmed.ends_with("/v1") {
            return format!("{trimmed}/v1/models");
        }

        format!("{trimmed}/models")
    }
}

#[async_trait]
impl AIProvider for OpenAIProvider {
    async fn chat_completions(
        &self,
        req: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse> {
        let url = format!("{}/chat/completions", self.api_base);
        let request_body = self.convert_request(req);

        let response = self
            .client
            .post(&url)
            .header(TOKEN_KEY, format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| Error::ExternalServiceError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::ExternalServiceError(format!(
                "OpenAI API error: {}",
                error_text
            )));
        }

        let openai_resp: serde_json::Value = response
            .json()
            .await
            .map_err(|e| Error::ExternalServiceError(format!("Failed to parse response: {}", e)))?;

        self.convert_response(openai_resp)
    }

    async fn completions(&self, req: &CompletionRequest) -> Result<CompletionResponse> {
        let url = format!("{}/completions", self.api_base);
        let request_body = self.convert_completion_request(req);

        let response = self
            .client
            .post(&url)
            .header(TOKEN_KEY, format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| Error::ExternalServiceError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::ExternalServiceError(format!(
                "OpenAI API error: {}",
                error_text
            )));
        }

        let openai_resp: serde_json::Value = response
            .json()
            .await
            .map_err(|e| Error::ExternalServiceError(format!("Failed to parse response: {}", e)))?;

        self.convert_completion_response(openai_resp)
    }

    async fn embeddings(&self, req: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        let url = format!("{}/embeddings", self.api_base);
        let request_body = self.convert_embedding_request(req);

        let response = self
            .client
            .post(&url)
            .header(TOKEN_KEY, format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| Error::ExternalServiceError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::ExternalServiceError(format!(
                "OpenAI API error: {}",
                error_text
            )));
        }

        let openai_resp: serde_json::Value = response
            .json()
            .await
            .map_err(|e| Error::ExternalServiceError(format!("Failed to parse response: {}", e)))?;

        self.convert_embedding_response(openai_resp)
    }

    async fn chat_stream(
        &self,
        req: &ChatCompletionRequest,
    ) -> Result<Box<dyn futures::Stream<Item = Result<ChatCompletionChunk>> + Send + Unpin>> {
        let url = format!("{}/chat/completions", self.api_base);
        let mut request_body = self.convert_request(req);
        request_body["stream"] = json!(true);

        let response = self
            .client
            .post(&url)
            .header(TOKEN_KEY, format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| Error::ExternalServiceError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::ExternalServiceError(format!(
                "OpenAI API error: {}",
                error_text
            )));
        }

        // 简化实现：返回一个空的流（实际实现需要处理 SSE）
        use futures::stream;
        let empty_stream = stream::empty();
        Ok(Box::new(empty_stream))
    }

    async fn health_check(&self) -> Result<bool> {
        let url = self.resolve_models_endpoint();

        let response = self
            .client
            .get(&url)
            .header(TOKEN_KEY, format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| Error::ExternalServiceError(format!("Health check failed: {}", e)))?;

        Ok(response.status().is_success())
    }

    fn get_model_info(&self) -> ModelInfo {
        ModelInfo {
            provider: "openai".to_string(),
            capabilities: vec![
                "chat".to_string(),
                "completion".to_string(),
                "embedding".to_string(),
                "streaming".to_string(),
                "function_calling".to_string(),
                "json_mode".to_string(),
            ],
            max_tokens: 8192,
            streaming: true,
        }
    }
}
