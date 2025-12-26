// 用途：声明价格规则服务模块
// 原因：处理价格规则相关业务逻辑
mod price_rule_service;

// 用途：声明配额管理服务模块
// 原因：处理用户配额相关业务逻辑
mod quota_service;

// 用途：声明计费服务模块
// 原因：处理费用计算和配额检查
mod billing_service;

// 用途：声明账单服务模块
// 原因：处理账单生成和支付
mod bill_service;

// 用途：声明统计分析服务模块
// 原因：处理用量统计和分析
mod analytics_service;

// 用途：声明token计数服务模块
// 原因：处理多模态内容的token计数
mod token_counter;

// 用途：声明安全加密服务模块
// 原因：提供AES-256-GCM加密、解密和哈希功能
mod encryption_service;

// 用途：声明Provider Trait定义模块
// 原因：定义统一的AI服务提供商接口
pub mod provider_trait;

// 用途：声明Provider工厂模块
// 原因：管理AI服务提供商的创建和配置
pub mod provider_factory;

// 用途：声明模型定义服务模块
// 原因：管理AI模型的定义、配置和生命周期
pub mod model_definition_service;

// 用途：声明供应商配置服务模块
// 原因：管理AI服务供应商的配置和生命周期
mod provider_config_service;

// 用途：声明计费标准服务模块
// 原因：处理AI模型计费标准相关业务逻辑
mod pricing_service;

// 用途：声明交易服务模块
// 原因：处理用户余额交易记录相关业务逻辑
pub mod transaction_service;

// 用途：声明余额服务模块
// 原因：处理用户余额管理相关业务逻辑
pub mod balance_service;

// 用途：导出价格规则服务
// 原因：允许其他模块访问价格规则功能
pub use price_rule_service::*;

// 用途：导出配额管理服务
// 原因：允许其他模块访问配额管理功能
pub use quota_service::*;

// 用途：导出计费服务
// 原因：允许其他模块访问计费功能
pub use billing_service::*;
pub use billing_service::CalculatedFee;

// 用途：导出账单服务
// 原因：允许其他模块访问账单功能
pub use bill_service::*;

// 用途：导出统计分析服务
// 原因：允许其他模块访问统计分析功能
pub use analytics_service::*;

// 用途：导出token计数服务
// 原因：允许其他模块访问token计数功能
pub use token_counter::*;

// 用途：导出安全加密服务
// 原因：允许其他模块访问加密功能
pub use encryption_service::*;

// 用途：导出模型定义服务
// 原因：允许其他模块访问模型管理功能
pub use model_definition_service::*;

// 用途：导出供应商配置服务
// 原因：允许其他模块访问供应商管理功能
pub use provider_config_service::*;

// 用途：导出计费标准服务
// 原因：允许其他模块访问计费标准功能
pub use pricing_service::*;

// 用途：导出交易服务
// 原因：允许其他模块访问交易功能
pub use transaction_service::*;

// 用途：导出余额服务
// 原因：允许其他模块访问余额功能
pub use balance_service::*;

// 用途：导出Provider相关类型
// 原因：供外部模块使用
pub use provider_trait::{
    AIProvider, ChatCompletionRequest as ProviderChatCompletionRequest, ChatCompletionResponse as ProviderChatCompletionResponse, CompletionRequest,
    CompletionResponse, EmbeddingRequest, EmbeddingResponse, ModelInfo,
    ChatCompletionChunk, Message, Choice, Usage as ProviderUsage, Delta, ChunkChoice,
    ResponseFormat, ToolDefinition, ToolChoice, ToolCall, FunctionCall,
    FunctionDefinition, FunctionChoice, Logprobs, EmbeddingInput, EmbeddingData,
    CompletionChoice
};

pub use provider_factory::{ProviderConfig, ProviderFactory, GenericProviderFactory, ProviderWrapper};

// 用途：导出DTO类型
// 原因：供控制器使用
// 明确导出DTO chat类型，避免与VO冲突
pub use crate::domain::dto::ai_hub::chat::{ChatCompletionRequest as DtoChatCompletionRequest, ChatCompletionResponse as DtoChatCompletionResponse, ReasoningConfig};
pub use crate::domain::dto::ai_hub::content::{ChatMessageContent, ChatMessageContentPart, ChatCompletionMessage};
pub use crate::domain::dto::ai_hub::embeddings::*;
pub use crate::domain::dto::ai_hub::logprob::*;
// 明确导出DTO streaming类型，避免与VO冲突
pub use crate::domain::dto::ai_hub::streaming::{ChatCompletionChunk as DtoChatCompletionChunk, ChatCompletionChunkChoice as DtoChatCompletionChunkChoice};
pub use crate::domain::dto::ai_hub::usage_log::*;
pub use crate::domain::dto::ai_hub::user_quota::*;
pub use crate::domain::dto::ai_hub::billing::*;
pub use crate::domain::dto::ai_hub::price_rule::*;
pub use crate::domain::dto::ai_hub::analytics::*;
pub use crate::domain::dto::ai_hub::pricing::*;
pub use crate::domain::dto::ai_hub::transaction::*;

// 用途：导出VO类型
// 原因：供控制器使用
// 明确导出VO chat类型，避免与DTO冲突
pub use crate::domain::vo::ai_hub::chat::{ChatCompletion as VoChatCompletion, ChatCompletionChoice as VoChatCompletionChoice};
pub use crate::domain::vo::ai_hub::embeddings::*;
pub use crate::domain::vo::ai_hub::responses::*;
pub use crate::domain::vo::ai_hub::usage::{Usage as VoUsage};
pub use crate::domain::vo::ai_hub::usage_log::{AiHubUsageLogVO as UsageLog, UsageStatisticsVO};
pub use crate::domain::vo::ai_hub::usage_log::*;
pub use crate::domain::vo::ai_hub::user_quota::*;
pub use crate::domain::vo::ai_hub::billing::*;
pub use crate::domain::vo::ai_hub::price_rule::*;
pub use crate::domain::vo::ai_hub::analytics::*;
// 明确导出VO streaming类型，避免与DTO冲突
pub use crate::domain::vo::ai_hub::streaming::{ChatCompletionChunk as VoChatCompletionChunk, ChatCompletionChunkChoice as VoChatCompletionChunkChoice, ChatCompletionChunkDelta, ToolCallChunk};