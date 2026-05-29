use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClaudeMessageResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub role: String,
    pub content: Vec<ClaudeContentBlock>,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
    pub usage: ClaudeUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClaudeContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClaudeUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClaudeErrorResponse {
    pub error: ClaudeErrorBody,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClaudeErrorBody {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClaudeMessageStartEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub message: ClaudeMessageResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClaudeContentBlockStartEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub index: u32,
    pub content_block: ClaudeContentBlock,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClaudeContentBlockDeltaEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub index: u32,
    pub delta: ClaudeContentDelta,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClaudeContentDelta {
    #[serde(rename = "type")]
    pub delta_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClaudeContentBlockStopEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClaudeMessageDeltaEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub delta: ClaudeMessageDelta,
    pub usage: ClaudeUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClaudeMessageDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClaudeMessageStopEvent {
    #[serde(rename = "type")]
    pub event_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClaudeErrorEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub error: ClaudeErrorBody,
}
