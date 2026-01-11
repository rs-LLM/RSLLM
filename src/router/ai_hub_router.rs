//! AI Hub 路由器模块
//!
//! 提供OpenAI兼容的API路由配置

use axum::{
    Router, middleware,
    routing::{get, post},
};

use std::sync::Arc;

// 导入控制器
use crate::context::ServiceContext;
use crate::controller::ai_hub::{
    chat_controller, embedding_controller, model_controller, responses_controller,
};
use crate::middleware::quota_check_middleware::quota_check_middleware;

/// 创建AI Hub路由器
///
/// 提供OpenAI兼容的API端点：
/// - GET /api/v1/models - 模型列表
/// - POST /api/v1/chat/completions - 聊天补全（支持流式和非流式）
/// - POST /api/v1/embeddings - 嵌入生成
/// - POST /api/v1/responses - Responses API
///
/// 所有端点都需要通过API密钥认证
pub fn create_ai_hub_router(state: Arc<ServiceContext>) -> Router<Arc<ServiceContext>> {
    Router::new()
        .route("/models", get(model_controller::list_openai_models))
        .route("/chat/completions", post(chat_controller::chat_completions))
        .route("/embeddings", post(embedding_controller::embeddings))
        .route("/responses", post(responses_controller::create_response))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            quota_check_middleware,
        ))
}
