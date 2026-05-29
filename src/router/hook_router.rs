use axum::{
    Json,
    extract::{Query, State},
};
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::domain::dto::hook::*;
use crate::domain::vo::hook::*;
use crate::domain::vo::response::ApiResponse;

#[utoipa::path(
    post,
    path = "/rsllm/api/hooks/register",
    request_body = RegisterHookRequest,
    responses(
        (status = 200, description = "注册Hook成功", body = ApiResponse<RegisterHookResponse>),
        (status = 400, description = "请求参数错误", body = ApiResponse<String>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "hook"
)]
pub async fn register_hook(
    State(state): State<Arc<ServiceContext>>,
    Json(params): Json<RegisterHookRequest>,
) -> Json<ApiResponse<RegisterHookResponse>> {
    match crate::controller::hook::register_hook(&state, params).await {
        Ok(response) => Json(ApiResponse::success(response)),
        Err(e) => Json(ApiResponse::<RegisterHookResponse>::error(
            "500",
            &e.to_string(),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/rsllm/api/hooks/unregister",
    request_body = UnregisterHookRequest,
    responses(
        (status = 200, description = "注销Hook成功", body = ApiResponse<UnregisterHookResponse>),
        (status = 400, description = "请求参数错误", body = ApiResponse<String>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "hook"
)]
pub async fn unregister_hook(
    State(state): State<Arc<ServiceContext>>,
    Json(params): Json<UnregisterHookRequest>,
) -> Json<ApiResponse<UnregisterHookResponse>> {
    match crate::controller::hook::unregister_hook(&state, params).await {
        Ok(response) => Json(ApiResponse::success(response)),
        Err(e) => Json(ApiResponse::<UnregisterHookResponse>::error(
            "500",
            &e.to_string(),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/rsllm/api/hooks",
    params(ListHooksQuery),
    responses(
        (status = 200, description = "获取Hook列表成功", body = ApiResponse<ListHooksResponse>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "hook"
)]
pub async fn list_hooks(
    State(state): State<Arc<ServiceContext>>,
    Query(params): Query<ListHooksQuery>,
) -> Json<ApiResponse<ListHooksResponse>> {
    match crate::controller::hook::list_hooks(&state, params).await {
        Ok(response) => Json(ApiResponse::success(response)),
        Err(e) => Json(ApiResponse::<ListHooksResponse>::error(
            "500",
            &e.to_string(),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/rsllm/api/hooks/events",
    params(ListEventsQuery),
    responses(
        (status = 200, description = "获取Hook事件历史成功", body = ApiResponse<ListEventsResponse>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "hook"
)]
pub async fn list_events(
    State(state): State<Arc<ServiceContext>>,
    Query(params): Query<ListEventsQuery>,
) -> Json<ApiResponse<ListEventsResponse>> {
    match crate::controller::hook::list_events(&state, params).await {
        Ok(response) => Json(ApiResponse::success(response)),
        Err(e) => Json(ApiResponse::<ListEventsResponse>::error(
            "500",
            &e.to_string(),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/rsllm/api/hooks/events/emit",
    request_body = EmitHookEventRequest,
    responses(
        (status = 200, description = "投递Hook事件成功", body = ApiResponse<EmitHookEventResponse>),
        (status = 400, description = "请求参数错误", body = ApiResponse<String>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "hook"
)]
pub async fn emit_event(
    State(state): State<Arc<ServiceContext>>,
    Json(params): Json<EmitHookEventRequest>,
) -> Json<ApiResponse<EmitHookEventResponse>> {
    match crate::controller::hook::emit_event(&state, params).await {
        Ok(response) => Json(ApiResponse::success(response)),
        Err(e) => Json(ApiResponse::<EmitHookEventResponse>::error(
            "500",
            &e.to_string(),
        )),
    }
}

#[utoipa::path(
    delete,
    path = "/rsllm/api/hooks/events/clear",
    responses(
        (status = 200, description = "清除Hook事件历史成功", body = ApiResponse<ClearEventsResponse>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "hook"
)]
pub async fn clear_events(
    State(state): State<Arc<ServiceContext>>,
) -> Json<ApiResponse<ClearEventsResponse>> {
    match crate::controller::hook::clear_events(&state).await {
        Ok(response) => Json(ApiResponse::success(response)),
        Err(e) => Json(ApiResponse::<ClearEventsResponse>::error(
            "500",
            &e.to_string(),
        )),
    }
}

pub fn create_hook_router() -> axum::Router<Arc<ServiceContext>> {
    axum::Router::new()
        .route("/hooks/register", axum::routing::post(register_hook))
        .route("/hooks/unregister", axum::routing::post(unregister_hook))
        .route("/hooks", axum::routing::get(list_hooks))
        .route("/hooks/events", axum::routing::get(list_events))
        .route("/hooks/events/emit", axum::routing::post(emit_event))
        .route("/hooks/events/clear", axum::routing::delete(clear_events))
}
