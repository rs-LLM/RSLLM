//! Claude 供应商适配器实现

use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;
use serde_json::json;

use crate::error::{Result, Error};
use crate::service::ai_hub::provider_trait::{
    AIProvider, ChatCompletionRequest, ChatCompletionResponse, CompletionRequest, 
    CompletionResponse, EmbeddingRequest, EmbeddingResponse, ModelInfo, 
    ChatCompletionChunk, Message, Choice, Usage, EmbeddingInput, EmbeddingData, ToolChoice
};
use crate::service::ai_hub::ProviderConfig;

/// Claude 供应商实现
pub struct ClaudeProvider {
    client: Client,
    api_key: String,
    api_base: String,
}

impl ClaudeProvider {
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let timeout = Duration::from_secs(config.timeout_seconds.unwrap_or(30));
        
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| Error::ExternalServiceError(format!("Failed to create HTTP client: {}", e)))?;
        
        Ok(Self {
            client,
            api_key: config.api_key,
            api_base: config.api_base,
        })
    }
    
    /// 转换为 Claude 请求格式
    fn convert_request(&self, req: &ChatCompletionRequest) -> serde_json::Value {
        let mut request = json!({
            "model": req.model,
            "messages": req.messages.iter().map(|m| {
                json!({
                    "role": m.role,
                    "content": m.content,
                })
            }).collect::<Vec<_>>(),
        });
        
        if let Some(max_tokens) = req.max_tokens {
            request["max_tokens"] = json!(max_tokens);
        } else {
            request["max_tokens"] = json!(1024); // Claude 默认值
        }
        
        if let Some(temp) = req.temperature {
            request["temperature"] = json!(temp);
        }
        if let Some(top_p) = req.top_p {
            request["top_p"] = json!(top_p);
        }
        
        // Claude 使用 system 字段而不是 system message
        if let Some(first_msg) = req.messages.first() {
            if first_msg.role == "system" {
                request["system"] = json!(first_msg.content);
                // 移除 system message 从 messages 数组
                if req.messages.len() > 1 {
                    let messages: Vec<_> = req.messages.iter()
                        .skip(1)
                        .map(|m| json!({ "role": m.role, "content": m.content }))
                        .collect();
                    request["messages"] = json!(messages);
                } else {
                    request["messages"] = json!([]);
                }
            }
        }
        
        // Claude 工具调用格式转换
        if let Some(tools) = &req.tools {
            let claude_tools: Vec<_> = tools.iter().map(|tool| {
                json!({
                    "name": tool.function.name,
                    "description": tool.function.description,
                    "input_schema": tool.function.parameters,
                })
            }).collect();
            request["tools"] = json!(claude_tools);
        }
        
        if let Some(tool_choice) = &req.tool_choice {
            match tool_choice {
                ToolChoice::String(_s) => {
                    request["tool_choice"] = json!({ "type": "auto" });
                }
                ToolChoice::Object(obj) => {
                    request["tool_choice"] = json!({
                        "type": obj.r#type,
                        "name": obj.function.name,
                    });
                }
            }
        }
        
        request
    }
    
    /// 转换 Claude 响应为统一格式
    fn convert_response(&self, claude_resp: serde_json::Value) -> Result<ChatCompletionResponse> {
        let id = claude_resp["id"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        
        let model = claude_resp["model"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        
        let created = claude_resp["created"]
            .as_i64()
            .unwrap_or(0);
        
        let choices = claude_resp["content"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .enumerate()
            .map(|(index, content)| {
                let message = Message {
                    role: "assistant".to_string(),
                    content: content["text"]
                        .as_str()
                        .unwrap_or("")
                        .to_string(),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                };
                
                Choice {
                    index: index as i32,
                    message,
                    finish_reason: claude_resp["stop_reason"]
                        .as_str()
                        .unwrap_or("")
                        .to_string(),
                    logprobs: None,
                }
            })
            .collect();
        
        let usage_value = &claude_resp["usage"];
        let usage = Usage {
            prompt_tokens: usage_value["input_tokens"].as_i64().unwrap_or(0) as i32,
            completion_tokens: usage_value["output_tokens"].as_i64().unwrap_or(0) as i32,
            total_tokens: usage_value["input_tokens"].as_i64().unwrap_or(0) as i32 + 
                         usage_value["output_tokens"].as_i64().unwrap_or(0) as i32,
        };
        
        Ok(ChatCompletionResponse {
            id,
            object: "chat.completion".to_string(),
            created,
            model,
            choices,
            usage,
        })
    }
    
    /// 转换为 Claude 嵌入请求格式（Claude 不直接支持嵌入，使用第三方服务）
    fn convert_embedding_request(&self, req: &EmbeddingRequest) -> serde_json::Value {
        // Claude 本身不支持嵌入，这里假设使用兼容的嵌入服务
        let mut request = json!({
            "model": req.model,
            "input": match &req.input {
                EmbeddingInput::String(s) => json!(vec![s]),
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
    
    /// 转换 Claude 嵌入响应为统一格式
    fn convert_embedding_response(&self, claude_resp: serde_json::Value) -> Result<EmbeddingResponse> {
        // 假设响应格式与 OpenAI 兼容
        let object = claude_resp["object"]
            .as_str()
            .unwrap_or("list")
            .to_string();
        
        let model = claude_resp["model"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        
        let data = claude_resp["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|item| {
                EmbeddingData {
                    object: item["object"]
                        .as_str()
                        .unwrap_or("embedding")
                        .to_string(),
                    embedding: item["embedding"]
                        .as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .filter_map(|v| v.as_f64())
                        .collect(),
                    index: item["index"].as_i64().unwrap_or(0) as i32,
                }
            })
            .collect();
        
        let usage_value = &claude_resp["usage"];
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
}

#[async_trait]
impl AIProvider for ClaudeProvider {
    async fn chat_completions(&self, req: &ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        let url = format!("{}/messages", self.api_base);
        let request_body = self.convert_request(req);
        
        let response = self.client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| Error::ExternalServiceError(format!("Request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::ExternalServiceError(format!("Claude API error: {}", error_text)));
        }
        
        let claude_resp: serde_json::Value = response.json().await
            .map_err(|e| Error::ExternalServiceError(format!("Failed to parse response: {}", e)))?;
        
        self.convert_response(claude_resp)
    }
    
    async fn completions(&self, _req: &CompletionRequest) -> Result<CompletionResponse> {
        // Claude 不直接支持传统的补全接口，使用聊天接口模拟
        Err(Error::BusinessError("Claude does not support completions endpoint. Use chat_completions instead.".to_string()))
    }
    
    async fn embeddings(&self, req: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        // Claude 本身不支持嵌入，这里假设使用兼容的嵌入服务
        let url = format!("{}/embeddings", self.api_base);
        let request_body = self.convert_embedding_request(req);
        
        let response = self.client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| Error::ExternalServiceError(format!("Request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::ExternalServiceError(format!("Claude API error: {}", error_text)));
        }
        
        let claude_resp: serde_json::Value = response.json().await
            .map_err(|e| Error::ExternalServiceError(format!("Failed to parse response: {}", e)))?;
        
        self.convert_embedding_response(claude_resp)
    }
    
    async fn chat_stream(&self, req: &ChatCompletionRequest) -> Result<Box<dyn futures::Stream<Item = Result<ChatCompletionChunk>> + Send + Unpin>> {
        let url = format!("{}/messages", self.api_base);
        let mut request_body = self.convert_request(req);
        request_body["stream"] = json!(true);
        
        let response = self.client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| Error::ExternalServiceError(format!("Request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::ExternalServiceError(format!("Claude API error: {}", error_text)));
        }
        
        // 简化实现：返回一个空的流（实际实现需要处理 SSE）
        use futures::stream;
        let empty_stream = stream::empty();
        Ok(Box::new(empty_stream))
    }
    
    async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/models", self.api_base);
        
        let response = self.client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await
            .map_err(|e| Error::ExternalServiceError(format!("Health check failed: {}", e)))?;
        
        Ok(response.status().is_success())
    }
    
    fn get_model_info(&self) -> ModelInfo {
        ModelInfo {
            provider: "claude".to_string(),
            capabilities: vec![
                "chat".to_string(),
                "streaming".to_string(),
                "function_calling".to_string(),
                "large_context".to_string(),
            ],
            max_tokens: 4096,
            streaming: true,
        }
    }
}