//! AI Hub 路由器模块
//!
//! 提供OpenAI兼容的API路由配置

use axum::{
    routing::post,
    Router,
};

use std::sync::Arc;

// 导入控制器
use crate::controller::ai_hub::{chat_controller, completion_controller, embedding_controller};
use crate::context::ServiceContext;

/// 创建AI Hub路由器
///
/// 提供OpenAI兼容的API端点：
/// - POST /v1/chat/completions - 聊天补全
/// - POST /v1/completions - 文本补全
/// - POST /v1/embeddings - 嵌入生成
pub fn create_ai_hub_router() -> Router<Arc<ServiceContext>> {
    Router::new()
        .route("/v1/chat/completions", post(chat_controller::chat_completions))
        .route("/v1/completions", post(completion_controller::completions))
        .route("/v1/embeddings", post(embedding_controller::embeddings))
}