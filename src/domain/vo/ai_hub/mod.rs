//! AI Hub 视图对象模块。
//! 导出聊天、嵌入、用量、响应与统计分析场景使用的视图对象子模块。

// 用途：导出AI Hub 视图对象相关模块

// 用途：导出聊天完成相关视图对象
// 说明：包含聊天对话响应等视图对象
pub mod chat;
pub use chat::*;

pub mod messages;
pub use messages::*;

// 用途：导出向量嵌入相关视图对象
// 说明：包含嵌入响应等视图对象
pub mod embeddings;
pub use embeddings::*;

// 用途：导出使用统计相关视图对象
// 说明：包含token使用量、费用统计等视图对象
pub mod usage;
pub use usage::*;

// 用途：导出响应相关视图对象
// 说明：包含模型列表响应等视图对象
pub mod responses;
pub use responses::*;

pub mod models;
pub use models::*;

// 用途：导出用量记录相关视图对象
pub mod usage_log;
pub use usage_log::*;

// 用途：导出用户配额相关视图对象
pub mod user_quota;
pub use user_quota::*;

// 用途：导出统计分析相关视图对象
pub mod analytics;
pub use analytics::*;

// 用途：导出管理员统计相关视图对象
pub mod admin_stats;
pub use admin_stats::*;

// 用途：导出用户统计相关视图对象
pub mod user_stats;
pub use user_stats::*;

// 用途：导出交易相关视图对象
pub mod transaction;
pub use transaction::*;

// 用途：导出流式响应相关视图对象
pub mod streaming;
pub use streaming::{
    ChatCompletionChunk, ChatCompletionChunkChoice, ChatCompletionChunkDelta, QuotaWarning,
    WebSocketChatRequest, WebSocketMessageType,
};
