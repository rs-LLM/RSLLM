use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::chat::ChatCompletionRequest;
use super::content::{ChatCompletionMessage, ChatMessageContent};

/// /v1/completions 兼容请求（最小实现）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompletionAliasRequest {
    pub model: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

impl CompletionAliasRequest {
    pub fn into_chat_request(self) -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: self.model,
            messages: vec![ChatCompletionMessage {
                role: "user".to_string(),
                content: Some(ChatMessageContent::String(self.prompt)),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                reasoning_content: None,
                extra_fields: serde_json::Value::default(),
            }],
            temperature: self.temperature,
            top_p: self.top_p,
            n: self.n,
            stream: self.stream,
            stop: self.stop,
            max_tokens: self.max_tokens,
            max_completion_tokens: None,
            parallel_tool_calls: None,
            presence_penalty: self.presence_penalty,
            frequency_penalty: self.frequency_penalty,
            logit_bias: None,
            tool_choice: None,
            tools: None,
            user: self.user,
            logprobs: None,
            top_logprobs: None,
            response_format: None,
            reasoning: None,
            extra_body: None,
            extra_fields: serde_json::Value::default(),
        }
    }
}
