//! Responses API 视图对象模块。
//! 定义 Responses API 的状态、错误、输出与流式分块等响应结构，供 AI Hub 接口对外返回。

// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：导入聊天消息内容相关结构体
// 说明：用于处理聊天消息的内容结构
use crate::domain::dto::content::ChatCompletionMessage;
// 用途：导入使用统计相关结构体
// 说明：用于统计资源使用情况
use super::usage::Usage;

// 用途：响应状态枚举
// 说明：表示Responses API响应的不同状态
/// 响应状态枚举。
/// 表示 Responses API 在处理生命周期中的状态阶段。
#[derive(Deserialize, Serialize, Clone, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ResponseStatus {
    // 用途：进行中
    // 说明：响应正在生成中
    InProgress,
    // 用途：已完成
    // 说明：响应已成功生成
    Completed,
    // 用途：失败
    // 说明：响应生成失败
    Failed,
    // 用途：已取消
    // 说明：响应生成被取消
    Canceled,
    // 用途：不完整
    // 说明：响应生成未完成
    Incomplete,
}

// 用途：响应错误结构体
// 说明：表示Responses API的错误信息
/// 响应错误结构体。
/// 表示 Responses API 返回的错误类型、消息与可选上下文。
#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct ResponseError {
    // 用途：错误类型
    // 说明：错误的类型标识
    #[serde(rename = "type")]
    pub error_type: String,
    // 用途：错误消息
    // 说明：错误的详细描述
    pub message: String,
    // 用途：错误代码
    // 说明：错误的代码标识
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    // 用途：错误参数
    // 说明：导致错误的参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct OpenAIErrorResponse {
    pub error: ResponseError,
}

impl OpenAIErrorResponse {
    pub fn new(status_code: u16, message: impl Into<String>) -> Self {
        let code = status_code.to_string();
        Self {
            error: ResponseError {
                error_type: match status_code {
                    400 => "invalid_request_error",
                    401 => "authentication_error",
                    402 => "billing_error",
                    429 => "rate_limit_error",
                    _ => "server_error",
                }
                .to_string(),
                message: message.into(),
                code: Some(code),
                param: None,
            },
        }
    }
}

// 用途：响应输出内容枚举
// 说明：支持多种输出类型
/// 响应输出内容枚举。
/// 统一承载消息文本、纯文本或多模态输出的数据形态。
#[derive(Deserialize, Serialize, Clone, ToSchema)]
#[serde(untagged)]
pub enum ResponseOutput {
    // 用途：消息输出
    // 说明：包含角色和内容的消息输出
    Message(ChatCompletionMessage),
    // 用途：文本输出
    // 说明：简单的文本字符串输出
    Text(String),
    // 用途：多模态输出
    // 说明：包含文本、图像等多种媒体类型的输出
    MultiModal(Vec<serde_json::Value>),
}

// 用途：工具调用结构体
// 说明：表示工具调用的详细信息
/// 工具调用结构体。
/// 表示一次工具调用的标识、类型与函数调用参数。
#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct ResponseToolCall {
    // 用途：工具调用ID
    // 说明：工具调用的唯一标识符
    pub id: String,
    // 用途：工具类型
    // 说明：工具的类型标识，如"function"
    #[serde(rename = "type")]
    pub tool_type: String,
    // 用途：函数调用
    // 说明：函数调用的详细信息
    pub function: ResponseFunctionCall,
}

// 用途：函数调用结构体
// 说明：表示函数调用的详细信息
/// 函数调用结构体。
/// 表示工具调用中的函数名与序列化后的参数内容。
#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct ResponseFunctionCall {
    // 用途：函数名称
    // 说明：被调用的函数名称
    pub name: String,
    // 用途：函数参数
    // 说明：传递给函数的参数，JSON格式
    pub arguments: String,
}

// 用途：推理过程结构体
// 说明：表示模型的推理过程内容
/// 推理过程结构体。
/// 表示模型在响应过程中产出的推理文本与可选 token 计数。
#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct ResponseReasoning {
    // 用途：推理内容
    // 说明：模型的推理过程文本
    pub content: String,
    // 用途：推理令牌数
    // 说明：推理过程使用的token数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_count: Option<u32>,
}

// 用途：Responses API响应结构体
// 说明：用于表示Responses API的完整响应，这是OpenAI推荐的新API响应格式
/// Responses API 响应结构体。
/// 表示一次非流式响应的完整结果，包含状态、输出、用量与上下文信息。
#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct Response {
    // 用途：响应ID
    // 说明：当前响应的唯一标识符，用于追踪和审计
    pub id: String,
    // 用途：对象类型
    // 说明：标识响应对象的类型，通常为"response"
    pub object: String,
    // 用途：创建时间
    // 说明：响应创建的时间戳
    pub created: u64,
    // 用途：状态
    // 说明：响应的当前状态
    pub status: ResponseStatus,
    // 用途：错误信息
    // 说明：如果响应失败，包含错误详情
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
    // 用途：输出内容
    // 说明：模型生成的输出内容
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Vec<ResponseOutput>>,
    // 用途：输出文本
    // 说明：简化的文本输出，便于快速访问
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_text: Option<String>,
    // 用途：解析后的输出
    // 说明：结构化输出，当使用response_format时可用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_parsed: Option<serde_json::Value>,
    // 用途：使用统计
    // 说明：当前响应的资源使用统计信息
    pub usage: Usage,
    // 用途：推理过程
    // 说明：模型的推理过程内容
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<Vec<ResponseReasoning>>,
    // 用途：指令
    // 说明：用于指导模型行为的系统指令
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    // 用途：输入内容
    // 说明：请求的输入内容
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Vec<serde_json::Value>>,
    // 用途：并行工具调用
    // 说明：是否启用了并行工具调用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
    // 用途：工具调用
    // 说明：工具调用的详细信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<ResponseToolCall>,
    // 用途：前一个响应ID
    // 说明：链接到前一个响应以维护对话状态
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    // 用途：元数据
    // 说明：响应的附加元数据信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

// 用途：Responses API流式响应结构体
// 说明：用于表示Responses API的流式响应分块
/// Responses API 流式响应分块结构体。
/// 表示流式输出过程中的单个增量分块与状态信息。
#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct ResponseChunk {
    // 用途：响应ID
    // 说明：当前流式响应的唯一标识符
    pub id: String,
    // 用途：对象类型
    // 说明：标识响应对象的类型，通常为"response.chunk"
    pub object: String,
    // 用途：创建时间
    // 说明：响应创建的时间戳
    pub created: u64,
    // 用途：状态
    // 说明：当前流式响应的状态
    pub status: ResponseStatus,
    // 用途：增量内容
    // 说明：当前分块包含的增量内容
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<ResponseDelta>,
    // 用途：错误信息
    // 说明：如果响应失败，包含错误详情
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
}

// 用途：响应增量结构体
// 说明：表示流式响应中的增量内容
/// 响应增量结构体。
/// 表示流式响应分块中的内容增量、工具调用增量与完成原因。
#[derive(Deserialize, Serialize, Clone, ToSchema)]
pub struct ResponseDelta {
    // 用途：增量内容
    // 说明：增量输出的内容
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    // 用途：工具调用增量
    // 说明：工具调用的增量信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ResponseToolCall>>,
    // 用途：推理增量
    // 说明：推理过程的增量内容
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    // 用途：完成原因
    // 说明：响应完成的原因
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}
