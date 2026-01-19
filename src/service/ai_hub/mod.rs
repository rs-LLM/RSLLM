// 用途：声明配额管理服务模块
// 原因：处理用户配额相关业务逻辑
mod quota_service;

// 用途：声明计费服务模块
// 原因：处理费用计算和配额检查
mod billing_service;

// 用途：声明用量记录服务模块
// 原因：处理用量记录查询、统计和详情查看
mod usage_log_service;

// 用途：声明统计分析服务模块
// 原因：处理用量统计和分析
mod analytics_service;

// 用途：声明管理员统计服务模块
// 原因：处理管理员面板统计数据聚合和缓存
mod admin_stats_service;

// 用途：声明用户统计服务模块
// 原因：处理用户面板统计数据聚合和缓存
mod user_stats_service;

// 用途：声明token计数服务模块
// 原因：处理多模态内容的token计数
pub mod token_counter;

// 用途：声明安全加密服务模块
// 原因：提供AES-256-GCM加密、解密和哈希功能
pub mod encryption_service;

// 用途：声明Provider Trait定义模块
// 原因：定义统一的AI服务提供商接口
pub mod provider_trait;

// 用途：声明Provider工厂模块
// 原因：管理AI服务提供商的创建和配置
pub mod provider_factory;

// 用途：声明供应商配置服务模块
// 原因：管理AI服务供应商的配置和生命周期
mod provider_config_service;

// 用途：声明交易服务模块
// 原因：处理用户余额交易记录相关业务逻辑
pub mod transaction_service;

// 用途：声明余额服务模块
// 原因：处理用户余额管理相关业务逻辑
mod balance_service;

// 用途：声明用户等级服务模块
// 原因：处理用户等级管理相关业务逻辑
mod user_level_service;

// 用途：声明等级与模型速率限制服务模块
// 原因：处理等级与模型速率限制相关业务逻辑
mod user_level_model_rate_limit_service;

// 用途：声明API密钥服务模块
// 原因：处理API密钥管理相关业务逻辑
mod api_key_service;

// 用途：声明速率限制服务模块
// 原因：处理RPM/TPM速率限制相关业务逻辑
pub mod rate_limit_service;

// 用途：导出配额管理服务
// 原因：允许其他模块访问配额管理功能
pub use quota_service::*;

// 用途：导出计费服务
// 原因：允许其他模块访问计费功能
pub use billing_service::CalculateAndCheckParams;
pub use billing_service::CalculatedFee;
pub use billing_service::ListUsageLogsParams;
pub use billing_service::QuickBillParams;
pub use billing_service::*;

// 用途：导出用量记录服务
// 原因：允许其他模块访问用量记录查询和统计功能
pub use usage_log_service::*;

// 用途：导出统计分析服务
// 原因：允许其他模块访问统计分析功能
pub use analytics_service::*;

// 用途：导出管理员统计服务
// 原因：允许其他模块访问管理员统计功能
pub use admin_stats_service::*;

// 用途：导出用户统计服务
// 原因：允许其他模块访问用户统计功能
pub use user_stats_service::*;

// 用途：导出token计数服务
// 原因：允许其他模块访问token计数功能
pub use token_counter::*;

// 用途：导出安全加密服务
// 原因：允许其他模块访问加密功能
pub use encryption_service::*;

// 用途：导出供应商配置服务
// 原因：允许其他模块访问供应商管理功能
pub use provider_config_service::*;

// 用途：导出交易服务
// 原因：允许其他模块访问交易功能
pub use transaction_service::*;

// 用途：导出余额服务
// 原因：允许其他模块访问余额功能
pub use balance_service::*;

// 用途：导出用户等级服务
// 原因：允许其他模块访问用户等级管理功能
pub use user_level_service::*;

// 用途：导出等级与模型速率限制服务
// 原因：允许其他模块访问等级与模型速率限制功能
pub use user_level_model_rate_limit_service::*;

// 用途：导出API密钥服务
// 原因：允许其他模块访问API密钥管理功能
pub use api_key_service::*;

// 用途：导出速率限制服务
// 原因：允许其他模块访问速率限制功能
pub use rate_limit_service::*;

// 用途：导出Provider相关类型
// 原因：供外部模块使用
pub use provider_trait::{
    AIProvider, ChatCompletionChunk, ChatCompletionRequest as ProviderChatCompletionRequest,
    ChatCompletionResponse as ProviderChatCompletionResponse, Choice, ChunkChoice,
    CompletionChoice, CompletionRequest, CompletionResponse, Delta, EmbeddingData, EmbeddingInput,
    EmbeddingRequest, EmbeddingResponse, FunctionCall, FunctionChoice, FunctionDefinition,
    Logprobs, Message, ModelInfo, ResponseFormat, ToolCall, ToolChoice, ToolDefinition,
    Usage as ProviderUsage,
};

pub use provider_factory::{
    GenericProviderFactory, ProviderConfig, ProviderFactory, ProviderWrapper,
};

// 用途：导出DTO类型
// 原因：供控制器使用
// 明确导出DTO chat类型，避免与VO冲突
pub use crate::domain::dto::ai_hub::chat::{
    ChatCompletionRequest as DtoChatCompletionRequest,
    ChatCompletionResponse as DtoChatCompletionResponse, ReasoningConfig,
};
pub use crate::domain::dto::ai_hub::content::{
    ChatCompletionMessage, ChatMessageContent, ChatMessageContentPart,
};
pub use crate::domain::dto::ai_hub::embeddings::*;
pub use crate::domain::dto::ai_hub::logprob::*;
// 明确导出DTO streaming类型，避免与VO冲突
pub use crate::domain::dto::ai_hub::analytics::*;
pub use crate::domain::dto::ai_hub::streaming::{
    ChatCompletionChunk as DtoChatCompletionChunk,
    ChatCompletionChunkChoice as DtoChatCompletionChunkChoice,
};
pub use crate::domain::dto::ai_hub::transaction::*;
pub use crate::domain::dto::ai_hub::usage_log::*;
pub use crate::domain::dto::ai_hub::user_quota::*;

// 用途：导出VO类型
// 原因：供控制器使用
// 明确导出VO chat类型，避免与DTO冲突
pub use crate::domain::vo::ai_hub::analytics::*;
pub use crate::domain::vo::ai_hub::chat::{
    ChatCompletion as VoChatCompletion, ChatCompletionChoice as VoChatCompletionChoice,
};
pub use crate::domain::vo::ai_hub::embeddings::*;
pub use crate::domain::vo::ai_hub::responses::*;
pub use crate::domain::vo::ai_hub::usage::Usage as VoUsage;
pub use crate::domain::vo::ai_hub::usage_log::*;
pub use crate::domain::vo::ai_hub::usage_log::{AiHubUsageLogVO as UsageLog, UsageStatisticsVO};
pub use crate::domain::vo::ai_hub::user_quota::*;
// 明确导出VO streaming类型，避免与DTO冲突
pub use crate::domain::vo::ai_hub::streaming::{
    ChatCompletionChunk as VoChatCompletionChunk,
    ChatCompletionChunkChoice as VoChatCompletionChunkChoice, ChatCompletionChunkDelta,
    ToolCallChunk,
};
