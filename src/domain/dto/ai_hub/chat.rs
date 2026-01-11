// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入哈希映射类型
// 说明：用于存储logit_bias等键值对数据
use std::collections::HashMap;
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：导入聊天消息内容相关结构体
// 说明：用于处理聊天消息的内容结构
use super::content::ChatCompletionMessage;
// 用途：导入响应格式相关结构体
// 说明：用于处理响应格式配置
use super::response_format::ResponseFormat;
// 用途：导入流式处理相关结构体
// 说明：用于处理流式响应数据
use super::streaming::ChatCompletionChunk;

// 用途：推理配置结构体
// 说明：用于配置AI模型的推理过程，如思考深度、token限制等
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct ReasoningConfig {
    // 用途：推理努力程度
    // 说明：控制推理的深度，可选值为"low"、"medium"、"high"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    // 用途：最大令牌数
    // 说明：推理过程的最大token限制，作为effort的替代方案
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    // 用途：是否排除推理过程
    // 说明：是否从响应中排除推理过程内容，默认false
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude: Option<bool>,
}

// 用途：工具选择枚举
// 说明：用于控制模型如何选择和调用工具
#[derive(Deserialize, Serialize, Clone, ToSchema)]
#[serde(untagged)]
pub enum ToolChoice {
    // 用途：禁用工具调用
    // 说明：模型不允许调用任何工具
    None,
    // 用途：自动选择工具
    // 说明：模型可以自动选择合适的工具进行调用
    Auto,
    // 用途：指定具体工具
    // 说明：指定特定工具的名称进行调用
    Named(String),
}

// 用途：工具定义结构体
// 说明：用于定义可用的工具及其参数模式
#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct ToolDefinition {
    // 用途：工具类型
    // 说明：工具的类型标识，固定为"function"
    #[serde(rename = "type")]
    pub r#type: String,
    // 用途：函数定义
    // 说明：工具函数的具体定义和参数模式
    pub function: ToolFunction,
}

// 用途：工具函数结构体
// 说明：用于定义工具函数的名称、描述和参数模式
#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct ToolFunction {
    // 用途：函数名称
    // 说明：工具函数的唯一标识符
    pub name: String,
    // 用途：函数描述
    // 说明：工具函数的功能描述和使用说明
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    // 用途：参数模式
    // 说明：工具函数的输入参数定义，JSON Schema格式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

// 用途：推理配置实现
// 说明：为推理配置提供验证和转换方法
impl ReasoningConfig {
    // 用途：验证配置参数
    // 说明：检查effort和max_tokens参数的有效性
    pub fn validate(&self) -> Result<(), String> {
        if self.effort.is_some() && self.max_tokens.is_some() {
            log::warn!("Both effort and max_tokens specified - prioritizing max_tokens");
        }

        // Only validate effort if max_tokens is not present (since max_tokens takes priority)
        if let Some(effort) = &self.effort {
            if effort.trim().is_empty() {
                if self.max_tokens.is_none() {
                    return Err("Effort cannot be empty string".to_string());
                }
            } else if self.max_tokens.is_none()
                && !["low", "medium", "high"].contains(&effort.as_str())
            {
                return Err("Invalid effort value. Must be 'low', 'medium', or 'high'".to_string());
            }
        }

        Ok(())
    }

    // 用途：转换为OpenAI effort参数
    // 说明：为OpenAI/Azure API生成effort参数，优先使用max_tokens
    pub fn to_openai_effort(&self) -> Option<String> {
        if self.max_tokens.is_some() {
            // If max_tokens is specified, don't use effort for OpenAI
            None
        } else {
            // Only return effort if it's not empty
            self.effort
                .as_ref()
                .filter(|e| !e.trim().is_empty())
                .cloned()
        }
    }

    // 用途：转换为Gemini thinking budget
    // 说明：为Google Vertex AI (Gemini) 生成thinking budget参数
    pub fn to_gemini_thinking_budget(&self) -> Option<i32> {
        self.max_tokens.map(|tokens| tokens as i32)
    }

    // 用途：生成思考提示词
    // 说明：为Anthropic/Bedrock生成自定义思考提示，优先使用max_tokens
    pub fn to_thinking_prompt(&self) -> Option<String> {
        if self.max_tokens.is_some() {
            // If max_tokens is specified, use a generic thinking prompt
            Some("Think through this step-by-step with detailed reasoning.".to_string())
        } else {
            match self.effort.as_deref() {
                Some(effort) if !effort.trim().is_empty() => match effort {
                    "high" => {
                        Some("Think through this step-by-step with detailed reasoning.".to_string())
                    }
                    "medium" => Some("Consider this problem thoughtfully.".to_string()),
                    "low" => Some("Think about this briefly.".to_string()),
                    _ => None,
                },
                _ => None,
            }
        }
    }
}

// 用途：聊天完成请求结构体
// 说明：用于表示聊天对话的请求参数，包含所有可配置的选项
#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct ChatCompletionRequest {
    // 用途：模型名称
    // 说明：用于生成聊天的AI模型标识符
    pub model: String,
    // 用途：消息列表
    // 说明：聊天对话的消息序列，包含用户和助手的历史消息
    pub messages: Vec<ChatCompletionMessage>,
    // 用途：温度参数
    // 说明：控制生成内容的随机性，值越高越随机
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    // 用途：核采样参数
    // 说明：控制词汇选择的多样性，与temperature配合使用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    // 用途：生成数量
    // 说明：并行生成的回答数量，用于多样化输出
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    // 用途：流式输出
    // 说明：是否启用流式响应，实时返回生成的内容
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    // 用途：停止词列表
    // 说明：遇到这些词汇时停止生成
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    // 用途：最大令牌数
    // 说明：生成内容允许的最大token数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    // 用途：最大完成令牌数
    // 说明：专门限制完成内容的token数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    // 用途：并行工具调用
    // 说明：是否允许并行执行多个工具调用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    // 用途：存在惩罚
    // 说明：减少内容重复的参数，数值越高越避免重复
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    // 用途：频率惩罚
    // 说明：根据词汇出现频率调整生成概率
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    // 用途：对数偏差
    // 说明：手动调整特定词汇的选择概率
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<String, i32>>,
    // 用途：工具选择
    // 说明：控制工具调用的策略和方式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    // 用途：工具定义列表
    // 说明：可用的工具定义，用于函数调用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    // 用途：用户标识
    // 说明：可选的用户标识符，用于跟踪和审计
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    // 用途：日志概率
    // 说明：是否返回token的日志概率信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<bool>,
    // 用途：顶级日志概率数
    // 说明：返回每个位置顶级候选token的数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u32>,
    // 用途：响应格式
    // 说明：指定响应的格式类型和结构约束
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    // 用途：推理配置
    // 说明：配置AI模型的推理过程参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ReasoningConfig>,
    // 用途：额外请求体
    // 说明：用于传递额外的请求参数，如 thinking_budget 等
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_body: Option<serde_json::Value>,
    // 用途：额外字段
    // 说明：用于接收和发送模型特定的额外字段，如小米模型的特定参数
    #[serde(default = "serde_json::Value::default", flatten)]
    pub extra_fields: serde_json::Value,
}

// 用途：聊天完成响应枚举
// 说明：支持流式和非流式两种响应模式，用于处理不同的AI响应类型
pub enum ChatCompletionResponse {
    // 用途：流式响应模式
    // 说明：用于实时流式数据接收和处理
    Stream(futures::stream::BoxStream<'static, Result<ChatCompletionChunk, reqwest::Error>>),
    // 用途：非流式响应模式
    // 说明：用于一次性完整响应处理
    NonStream(crate::domain::vo::chat::ChatCompletion),
}
