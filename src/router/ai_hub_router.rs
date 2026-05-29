//! AI Hub 路由器模块
//!
//! 提供OpenAI兼容的API路由配置

use axum::{
    Extension, Router, middleware,
    response::IntoResponse,
    routing::{get, post},
};

use std::sync::Arc;

// 导入控制器
use crate::context::ServiceContext;
use crate::controller::ai_hub::{
    chat_controller, embedding_controller, messages_controller, model_controller,
    responses_controller,
};
use crate::service::ai_hub::AiRequestContext;

#[utoipa::path(
    post,
    path = "/api/v1/completions",
    request_body = crate::domain::dto::ai_hub::completion::CompletionAliasRequest,
    responses(
        (status = 200, description = "请求成功", body = crate::domain::vo::ai_hub::chat::ChatCompletion),
        (status = 400, description = "参数错误", body = crate::domain::vo::ai_hub::responses::OpenAIErrorResponse),
        (status = 401, description = "未授权", body = crate::domain::vo::ai_hub::responses::OpenAIErrorResponse),
        (status = 500, description = "服务器错误", body = crate::domain::vo::ai_hub::responses::OpenAIErrorResponse)
    ),
    tag = "chat",
    security(
        ("api_key" = [])
    )
)]
pub async fn create_completions_alias(
    headers: axum::http::HeaderMap,
    state: axum::extract::State<Arc<ServiceContext>>,
    request_context: Option<Extension<AiRequestContext>>,
    body: axum::Json<crate::domain::dto::ai_hub::completion::CompletionAliasRequest>,
) -> axum::response::Response {
    let completion_req = body.0;

    let params = crate::domain::dto::ai_hub::validation::CompletionRequestParams {
        model: &completion_req.model,
        prompt: &completion_req.prompt,
        max_tokens: completion_req.max_tokens,
        temperature: completion_req.temperature,
        top_p: completion_req.top_p,
        frequency_penalty: completion_req.frequency_penalty,
        presence_penalty: completion_req.presence_penalty,
        n: completion_req.n,
    };

    if let Err(e) =
        crate::domain::dto::ai_hub::validation::Validator::validate_completion_request(&params)
    {
        let mut response =
            axum::Json(crate::domain::vo::ai_hub::responses::OpenAIErrorResponse::new(400, e))
                .into_response();
        *response.status_mut() = axum::http::StatusCode::BAD_REQUEST;
        return response;
    }

    let chat_req = completion_req.into_chat_request();
    chat_controller::chat_completions(headers, state, request_context, axum::Json(chat_req)).await
}

#[utoipa::path(
    post,
    path = "/api/v1/messages",
    request_body = crate::domain::dto::ai_hub::messages::ClaudeMessagesRequest,
    responses(
        (status = 200, description = "请求成功", body = crate::domain::vo::ai_hub::messages::ClaudeMessageResponse),
        (status = 400, description = "参数错误", body = crate::domain::vo::ai_hub::messages::ClaudeErrorResponse),
        (status = 401, description = "未授权", body = crate::domain::vo::ai_hub::messages::ClaudeErrorResponse),
        (status = 500, description = "服务器错误", body = crate::domain::vo::ai_hub::messages::ClaudeErrorResponse)
    ),
    tag = "chat",
    security(
        ("api_key" = [])
    )
)]
pub async fn create_messages_alias(
    headers: axum::http::HeaderMap,
    state: axum::extract::State<Arc<ServiceContext>>,
    request_context: Option<Extension<AiRequestContext>>,
    body: axum::Json<crate::domain::dto::ai_hub::messages::ClaudeMessagesRequest>,
) -> axum::response::Response {
    messages_controller::create_messages(headers, state, request_context, body).await
}

#[utoipa::path(
    post,
    path = "/api/v1/responses/compact",
    request_body = crate::domain::dto::ai_hub::responses::ResponseCreateParams,
    responses(
        (status = 200, description = "响应创建成功", body = crate::domain::vo::ai_hub::responses::Response),
        (status = 400, description = "参数错误", body = crate::domain::vo::ai_hub::responses::OpenAIErrorResponse),
        (status = 401, description = "未授权", body = crate::domain::vo::ai_hub::responses::OpenAIErrorResponse),
        (status = 500, description = "服务器错误", body = crate::domain::vo::ai_hub::responses::OpenAIErrorResponse)
    ),
    tag = "responses",
    security(
        ("api_key" = [])
    )
)]
pub async fn create_compact_response_alias(
    headers: axum::http::HeaderMap,
    state: axum::extract::State<Arc<ServiceContext>>,
    request_context: Option<Extension<AiRequestContext>>,
    body: axum::Json<crate::domain::dto::ai_hub::responses::ResponseCreateParams>,
) -> impl axum::response::IntoResponse {
    responses_controller::create_response(headers, state, request_context, body).await
}

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
    let protected_routes = Router::new()
        .route("/models", get(model_controller::list_openai_models))
        .route("/chat/completions", post(chat_controller::chat_completions))
        .route("/completions", post(create_completions_alias))
        .route("/embeddings", post(embedding_controller::embeddings))
        .route("/responses", post(responses_controller::create_response))
        .route("/responses/compact", post(create_compact_response_alias))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            quota_check_middleware,
        ));

    let messages_routes = Router::new()
        .route("/messages", post(create_messages_alias))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            quota_check_middleware,
        ));

    Router::new().merge(messages_routes).merge(protected_routes)
}
