//! OpenAI 兼容 API 专用 Router
//!
//! 用于对外暴露 /v1/* 与 /v1beta/* 风格接口。

use std::sync::Arc;

use axum::{
    Extension, Router, middleware,
    routing::{get, post},
};

use crate::context::ServiceContext;
use crate::controller::ai_hub::{
    chat_controller, dashboard_billing_controller, embedding_controller, messages_controller,
    model_controller, responses_controller,
};
use crate::middleware::quota_check_middleware::quota_check_middleware;
use crate::router::ai_hub_router;
use crate::service::ai_hub::AiRequestContext;

#[utoipa::path(
    get,
    path = "/v1/models",
    responses(
        (status = 200, description = "查询成功", body = model_controller::OpenAIModelsListResponse),
        (status = 500, description = "服务器错误", body = model_controller::OpenAIModelsListResponse)
    ),
    tag = "models",
    security(
        ("api_key" = [])
    )
)]
pub async fn list_v1_models(
    state: axum::extract::State<Arc<ServiceContext>>,
) -> crate::error::Result<axum::Json<model_controller::OpenAIModelsListResponse>> {
    model_controller::list_openai_models(state).await
}

#[utoipa::path(
    post,
    path = "/v1/completions",
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
pub async fn create_v1_completions_alias(
    headers: axum::http::HeaderMap,
    state: axum::extract::State<Arc<ServiceContext>>,
    request_context: Option<Extension<AiRequestContext>>,
    body: axum::Json<crate::domain::dto::ai_hub::completion::CompletionAliasRequest>,
) -> axum::response::Response {
    ai_hub_router::create_completions_alias(headers, state, request_context, body).await
}

#[utoipa::path(
    post,
    path = "/v1/messages",
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
pub async fn create_v1_messages_alias(
    headers: axum::http::HeaderMap,
    state: axum::extract::State<Arc<ServiceContext>>,
    request_context: Option<Extension<AiRequestContext>>,
    body: axum::Json<crate::domain::dto::ai_hub::messages::ClaudeMessagesRequest>,
) -> axum::response::Response {
    messages_controller::create_messages(headers, state, request_context, body).await
}

#[utoipa::path(
    post,
    path = "/v1/chat/completions",
    request_body = crate::domain::dto::ai_hub::chat::ChatCompletionRequest,
    responses(
        (status = 200, description = "聊天补全成功", body = crate::domain::vo::ai_hub::chat::ChatCompletion),
        (status = 400, description = "参数错误", body = crate::domain::vo::ai_hub::responses::OpenAIErrorResponse),
        (status = 401, description = "未授权", body = crate::domain::vo::ai_hub::responses::OpenAIErrorResponse),
        (status = 429, description = "请求过多", body = crate::domain::vo::ai_hub::responses::OpenAIErrorResponse),
        (status = 500, description = "服务器错误", body = crate::domain::vo::ai_hub::responses::OpenAIErrorResponse)
    ),
    tag = "chat",
    security(
        ("api_key" = [])
    )
)]
pub async fn create_v1_chat_completions(
    headers: axum::http::HeaderMap,
    state: axum::extract::State<Arc<ServiceContext>>,
    request_context: Option<Extension<AiRequestContext>>,
    body: axum::Json<crate::domain::dto::ai_hub::chat::ChatCompletionRequest>,
) -> axum::response::Response {
    chat_controller::chat_completions(headers, state, request_context, body).await
}

#[utoipa::path(
    post,
    path = "/v1/responses/compact",
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

pub async fn create_v1_compact_response_alias(
    headers: axum::http::HeaderMap,
    state: axum::extract::State<Arc<ServiceContext>>,
    request_context: Option<Extension<AiRequestContext>>,
    body: axum::Json<crate::domain::dto::ai_hub::responses::ResponseCreateParams>,
) -> impl axum::response::IntoResponse {
    ai_hub_router::create_compact_response_alias(headers, state, request_context, body).await
}

#[utoipa::path(
    post,
    path = "/v1/embeddings",
    request_body = crate::domain::dto::ai_hub::embeddings::EmbeddingsRequest,
    responses(
        (status = 200, description = "嵌入生成成功", body = crate::domain::vo::ai_hub::embeddings::EmbeddingsResponse),
        (status = 400, description = "参数错误", body = crate::domain::vo::ai_hub::responses::OpenAIErrorResponse),
        (status = 401, description = "未授权", body = crate::domain::vo::ai_hub::responses::OpenAIErrorResponse),
        (status = 429, description = "请求过多", body = crate::domain::vo::ai_hub::responses::OpenAIErrorResponse),
        (status = 500, description = "服务器错误", body = crate::domain::vo::ai_hub::responses::OpenAIErrorResponse)
    ),
    tag = "embeddings",
    security(
        ("api_key" = [])
    )
)]
pub async fn create_v1_embeddings(
    headers: axum::http::HeaderMap,
    state: axum::extract::State<Arc<ServiceContext>>,
    request_context: Option<Extension<AiRequestContext>>,
    body: axum::Json<crate::domain::dto::ai_hub::embeddings::EmbeddingsRequest>,
) -> impl axum::response::IntoResponse {
    embedding_controller::embeddings(headers, state, request_context, body).await
}

#[utoipa::path(
    post,
    path = "/v1/responses",
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
pub async fn create_v1_response(
    headers: axum::http::HeaderMap,
    state: axum::extract::State<Arc<ServiceContext>>,
    request_context: Option<Extension<AiRequestContext>>,
    body: axum::Json<crate::domain::dto::ai_hub::responses::ResponseCreateParams>,
) -> impl axum::response::IntoResponse {
    responses_controller::create_response(headers, state, request_context, body).await
}

#[utoipa::path(
    get,
    path = "/v1/dashboard/billing/subscription",
    responses(
        (status = 200, description = "查询成功", body = crate::domain::dto::ai_hub::dashboard_billing::OpenAISubscriptionResponse),
        (status = 401, description = "未授权", body = crate::domain::vo::ai_hub::responses::OpenAIErrorResponse),
        (status = 500, description = "服务器错误", body = crate::domain::vo::ai_hub::responses::OpenAIErrorResponse)
    ),
    tag = "billing",
    security(
        ("api_key" = [])
    )
)]
pub async fn get_v1_dashboard_billing_subscription(
    headers: axum::http::HeaderMap,
    state: axum::extract::State<Arc<ServiceContext>>,
) -> crate::error::Result<
    axum::Json<crate::domain::dto::ai_hub::dashboard_billing::OpenAISubscriptionResponse>,
> {
    dashboard_billing_controller::get_dashboard_billing_subscription(headers, state).await
}

#[utoipa::path(
    get,
    path = "/v1/dashboard/billing/usage",
    params(
        ("start_date" = Option<String>, Query, description = "开始时间（可选）"),
        ("end_date" = Option<String>, Query, description = "结束时间（可选）")
    ),
    responses(
        (status = 200, description = "查询成功", body = crate::domain::dto::ai_hub::dashboard_billing::OpenAIUsageResponse),
        (status = 401, description = "未授权", body = crate::domain::vo::ai_hub::responses::OpenAIErrorResponse),
        (status = 500, description = "服务器错误", body = crate::domain::vo::ai_hub::responses::OpenAIErrorResponse)
    ),
    tag = "billing",
    security(
        ("api_key" = [])
    )
)]
pub async fn get_v1_dashboard_billing_usage(
    headers: axum::http::HeaderMap,
    state: axum::extract::State<Arc<ServiceContext>>,
    query: axum::extract::Query<
        crate::domain::dto::ai_hub::dashboard_billing::DashboardBillingUsageQuery,
    >,
) -> crate::error::Result<
    axum::Json<crate::domain::dto::ai_hub::dashboard_billing::OpenAIUsageResponse>,
> {
    dashboard_billing_controller::get_dashboard_billing_usage(headers, state, query).await
}

#[utoipa::path(
    get,
    path = "/v1beta/models",
    responses(
        (status = 200, description = "查询成功", body = model_controller::OpenAIModelsListResponse),
        (status = 500, description = "服务器错误", body = model_controller::OpenAIModelsListResponse)
    ),
    tag = "models",
    security(
        ("api_key" = [])
    )
)]
pub async fn list_v1beta_models(
    state: axum::extract::State<Arc<ServiceContext>>,
) -> crate::error::Result<axum::Json<model_controller::OpenAIModelsListResponse>> {
    model_controller::list_openai_models(state).await
}

#[utoipa::path(
    get,
    path = "/v1beta/openai/models",
    responses(
        (status = 200, description = "查询成功", body = model_controller::OpenAIModelsListResponse),
        (status = 500, description = "服务器错误", body = model_controller::OpenAIModelsListResponse)
    ),
    tag = "models",
    security(
        ("api_key" = [])
    )
)]
pub async fn list_v1beta_openai_models(
    state: axum::extract::State<Arc<ServiceContext>>,
) -> crate::error::Result<axum::Json<model_controller::OpenAIModelsListResponse>> {
    model_controller::list_openai_models(state).await
}

/// 创建 OpenAI 兼容协议 Router
///
/// 说明：
/// - 对外主路径前缀为 /v1
/// - 兼容补齐 /v1beta/models 与 /v1beta/openai/models
/// - 内部复用现有 AI Hub 稳定链路（models/chat/completions/embeddings/responses 等）
pub fn create_openai_router(state: Arc<ServiceContext>) -> Router<()> {
    let v1_protected_router = Router::new()
        .route("/v1/models", get(list_v1_models))
        .route("/v1/chat/completions", post(create_v1_chat_completions))
        .route("/v1/completions", post(create_v1_completions_alias))
        .route("/v1/embeddings", post(create_v1_embeddings))
        .route("/v1/responses", post(create_v1_response))
        .route(
            "/v1/responses/compact",
            post(create_v1_compact_response_alias),
        )
        .route(
            "/v1/dashboard/billing/subscription",
            get(get_v1_dashboard_billing_subscription),
        )
        .route(
            "/v1/dashboard/billing/usage",
            get(get_v1_dashboard_billing_usage),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            quota_check_middleware,
        ))
        .with_state(state.clone());

    let v1_messages_router = Router::new()
        .route("/v1/messages", post(create_v1_messages_alias))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            quota_check_middleware,
        ))
        .with_state(state.clone());

    let v1beta_router = Router::new()
        .route("/v1beta/models", get(list_v1beta_models))
        .route("/v1beta/openai/models", get(list_v1beta_openai_models))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            quota_check_middleware,
        ))
        .with_state(state.clone());

    Router::new()
        .merge(v1_messages_router)
        .merge(v1_protected_router)
        .merge(v1beta_router)
}
