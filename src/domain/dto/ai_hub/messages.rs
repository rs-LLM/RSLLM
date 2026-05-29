use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::chat::{ChatCompletionRequest, ToolChoice, ToolDefinition, ToolFunction};
use super::content::{
    ChatCompletionMessage, ChatMessageContent, ChatMessageContentPart, ChatMessageFunction,
    ChatMessageToolCall,
};

/// /v1/messages 兼容请求。
///
/// 当前优先完整支持文本类 content block，并将其映射到内部 ChatCompletionRequest。
/// 对暂不支持的 block 类型返回显式错误，避免静默忽略。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClaudeMessagesRequest {
    pub model: String,
    pub messages: Vec<ClaudeMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<ClaudeSystemPrompt>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ClaudeToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ClaudeToolChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClaudeMessage {
    pub role: String,
    pub content: ClaudeMessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum ClaudeMessageContent {
    Text(String),
    Blocks(Vec<ClaudeContentBlockInput>),
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum ClaudeSystemPrompt {
    Text(String),
    Blocks(Vec<ClaudeSystemContentBlockInput>),
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClaudeContentBlockInput {
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    #[serde(flatten, default = "serde_json::Value::default")]
    pub extra_fields: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClaudeSystemContentBlockInput {
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(flatten, default = "serde_json::Value::default")]
    pub extra_fields: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClaudeToolDefinition {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum ClaudeToolChoice {
    Mode(ClaudeToolChoiceMode),
    Tool(ClaudeNamedToolChoice),
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClaudeToolChoiceMode {
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClaudeNamedToolChoice {
    #[serde(rename = "type")]
    pub r#type: String,
    pub name: String,
}

impl ClaudeMessagesRequest {
    pub fn into_chat_request(self) -> Result<ChatCompletionRequest, String> {
        let mut mapped_messages = Vec::new();

        if let Some(system) = self.system {
            mapped_messages.push(ChatCompletionMessage {
                role: "system".to_string(),
                content: Some(system.try_into_chat_content()?),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                reasoning_content: None,
                extra_fields: serde_json::Value::default(),
            });
        }

        for message in self.messages {
            mapped_messages.push(message.try_into_chat_message()?);
        }

        let anthropic_tools = self.tools.clone();
        let anthropic_tool_choice = self.tool_choice.clone();

        Ok(ChatCompletionRequest {
            model: self.model,
            messages: mapped_messages,
            temperature: self.temperature,
            top_p: None,
            n: None,
            stream: self.stream,
            stop: self.stop_sequences,
            max_tokens: Some(self.max_tokens),
            max_completion_tokens: None,
            parallel_tool_calls: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            tool_choice: anthropic_tool_choice
                .map(|choice| choice.into_openai_tool_choice())
                .transpose()?,
            tools: anthropic_tools.map(|tools| {
                tools
                    .into_iter()
                    .map(|tool| tool.into_openai_tool())
                    .collect::<Vec<_>>()
            }),
            user: None,
            logprobs: None,
            top_logprobs: None,
            response_format: None,
            reasoning: None,
            extra_body: None,
            extra_fields: serde_json::json!({
                "anthropic_metadata": self.metadata,
            }),
        })
    }
}

impl ClaudeMessage {
    fn try_into_chat_message(self) -> Result<ChatCompletionMessage, String> {
        let role = self.role;
        let content = self.content;

        match role.as_str() {
            "assistant" => {
                let (content, tool_calls) = content.try_into_assistant_content_and_tool_calls()?;
                Ok(ChatCompletionMessage {
                    role,
                    content,
                    name: None,
                    tool_calls,
                    tool_call_id: None,
                    refusal: None,
                    reasoning_content: None,
                    extra_fields: serde_json::Value::default(),
                })
            }
            "user" => {
                let (content, tool_call_id) = content.try_into_user_content_and_tool_result()?;
                Ok(ChatCompletionMessage {
                    role,
                    content,
                    name: None,
                    tool_calls: None,
                    tool_call_id,
                    refusal: None,
                    reasoning_content: None,
                    extra_fields: serde_json::Value::default(),
                })
            }
            _ => Ok(ChatCompletionMessage {
                role,
                content: Some(content.try_into_chat_content()?),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                refusal: None,
                reasoning_content: None,
                extra_fields: serde_json::Value::default(),
            }),
        }
    }
}

impl ClaudeMessageContent {
    fn try_into_chat_content(self) -> Result<ChatMessageContent, String> {
        match self {
            Self::Text(text) => Ok(ChatMessageContent::String(text)),
            Self::Blocks(blocks) => Ok(ChatMessageContent::Array(
                blocks
                    .into_iter()
                    .map(ClaudeContentBlockInput::try_into_chat_part)
                    .collect::<Result<Vec<_>, _>>()?,
            )),
        }
    }

    fn try_into_assistant_content_and_tool_calls(
        self,
    ) -> Result<(Option<ChatMessageContent>, Option<Vec<ChatMessageToolCall>>), String> {
        match self {
            Self::Text(text) => Ok((Some(ChatMessageContent::String(text)), None)),
            Self::Blocks(blocks) => {
                let mut parts = Vec::new();
                let mut tool_calls = Vec::new();

                for block in blocks {
                    match block.r#type.as_str() {
                        "text" => parts.push(block.try_into_chat_part()?),
                        "tool_use" => tool_calls.push(block.try_into_tool_call()?),
                        unsupported => {
                            return Err(format!(
                                "Unsupported Claude assistant block type: {}",
                                unsupported
                            ));
                        }
                    }
                }

                let content = if parts.is_empty() {
                    None
                } else {
                    Some(ChatMessageContent::Array(parts))
                };
                let tool_calls = if tool_calls.is_empty() {
                    None
                } else {
                    Some(tool_calls)
                };

                Ok((content, tool_calls))
            }
        }
    }

    fn try_into_user_content_and_tool_result(
        self,
    ) -> Result<(Option<ChatMessageContent>, Option<String>), String> {
        match self {
            Self::Text(text) => Ok((Some(ChatMessageContent::String(text)), None)),
            Self::Blocks(blocks) => {
                let mut parts = Vec::new();
                let mut tool_result_id: Option<String> = None;

                for block in blocks {
                    match block.r#type.as_str() {
                        "text" => parts.push(block.try_into_chat_part()?),
                        "tool_result" => {
                            if tool_result_id.is_some() {
                                return Err(
                                    "Multiple Claude tool_result blocks in one user message are not supported"
                                        .to_string(),
                                );
                            }
                            let (part, tool_use_id) = block.try_into_tool_result_part()?;
                            parts.push(part);
                            tool_result_id = Some(tool_use_id);
                        }
                        unsupported => {
                            return Err(format!(
                                "Unsupported Claude user block type: {}",
                                unsupported
                            ));
                        }
                    }
                }

                let content = if parts.is_empty() {
                    None
                } else {
                    Some(ChatMessageContent::Array(parts))
                };

                Ok((content, tool_result_id))
            }
        }
    }
}

impl ClaudeSystemPrompt {
    fn try_into_chat_content(self) -> Result<ChatMessageContent, String> {
        match self {
            Self::Text(text) => Ok(ChatMessageContent::String(text)),
            Self::Blocks(blocks) => Ok(ChatMessageContent::Array(
                blocks
                    .into_iter()
                    .map(ClaudeSystemContentBlockInput::try_into_chat_part)
                    .collect::<Result<Vec<_>, _>>()?,
            )),
        }
    }
}

impl ClaudeContentBlockInput {
    fn try_into_chat_part(self) -> Result<ChatMessageContentPart, String> {
        match self.r#type.as_str() {
            "text" => Ok(ChatMessageContentPart {
                r#type: "text".to_string(),
                text: Some(self.text.unwrap_or_default()),
                image_url: None,
            }),
            unsupported => Err(format!(
                "Unsupported Claude content block type: {}. Currently only text blocks are supported",
                unsupported
            )),
        }
    }

    fn try_into_tool_call(self) -> Result<ChatMessageToolCall, String> {
        if self.r#type != "tool_use" {
            return Err(format!(
                "Unsupported Claude tool block type: {}",
                self.r#type
            ));
        }

        let id = self
            .id
            .ok_or_else(|| "Claude tool_use block missing id".to_string())?;
        let name = self
            .name
            .ok_or_else(|| "Claude tool_use block missing name".to_string())?;
        let arguments = self
            .input
            .unwrap_or_else(|| serde_json::json!({}))
            .to_string();

        Ok(ChatMessageToolCall {
            id,
            function: ChatMessageFunction { name, arguments },
            r#type: "function".to_string(),
        })
    }

    fn try_into_tool_result_part(self) -> Result<(ChatMessageContentPart, String), String> {
        if self.r#type != "tool_result" {
            return Err(format!(
                "Unsupported Claude tool result block type: {}",
                self.r#type
            ));
        }

        let tool_use_id = self
            .tool_use_id
            .ok_or_else(|| "Claude tool_result block missing tool_use_id".to_string())?;

        Ok((
            ChatMessageContentPart {
                r#type: "text".to_string(),
                text: Some(
                    self.content
                        .or(self.text)
                        .unwrap_or_else(|| self.extra_fields.to_string()),
                ),
                image_url: None,
            },
            tool_use_id,
        ))
    }
}

impl ClaudeSystemContentBlockInput {
    fn try_into_chat_part(self) -> Result<ChatMessageContentPart, String> {
        match self.r#type.as_str() {
            "text" => Ok(ChatMessageContentPart {
                r#type: "text".to_string(),
                text: Some(self.text.unwrap_or_default()),
                image_url: None,
            }),
            unsupported => Err(format!(
                "Unsupported Claude system block type: {}. Currently only text blocks are supported",
                unsupported
            )),
        }
    }
}

impl ClaudeToolDefinition {
    fn into_openai_tool(self) -> ToolDefinition {
        ToolDefinition {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: self.name,
                description: self.description,
                parameters: self.input_schema,
            },
        }
    }
}

impl ClaudeToolChoice {
    fn into_openai_tool_choice(self) -> Result<ToolChoice, String> {
        match self {
            Self::Mode(mode) => match mode.r#type.as_str() {
                "auto" => Ok(ToolChoice::Auto),
                "any" => Ok(ToolChoice::Auto),
                "none" => Ok(ToolChoice::None),
                unsupported => Err(format!(
                    "Unsupported Claude tool_choice type: {}",
                    unsupported
                )),
            },
            Self::Tool(tool) => {
                if tool.r#type != "tool" {
                    return Err(format!(
                        "Unsupported Claude named tool_choice type: {}",
                        tool.r#type
                    ));
                }
                Ok(ToolChoice::Named(tool.name))
            }
        }
    }
}
