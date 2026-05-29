use axum::{
    Json,
    extract::{Path, Query, State},
};
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::domain::dto::scheduled_task::*;
use crate::domain::vo::response::{ApiResponse, PageWrapper};
use crate::domain::vo::scheduled_task::*;

#[utoipa::path(
    post,
    path = "/rsllm/api/scheduled-tasks",
    request_body = CreateTaskRequest,
    responses(
        (status = 200, description = "创建任务成功", body = ApiResponse<CreateTaskResponse>),
        (status = 400, description = "请求参数错误", body = ApiResponse<String>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "scheduled-task"
)]
pub async fn create_task(
    State(state): State<Arc<ServiceContext>>,
    Json(params): Json<CreateTaskRequest>,
) -> Json<ApiResponse<CreateTaskResponse>> {
    match crate::controller::scheduled_task::create_task(&state, params).await {
        Ok(response) => Json(ApiResponse::success(response)),
        Err(e) => Json(ApiResponse::<CreateTaskResponse>::error(
            "500",
            &e.to_string(),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/rsllm/api/scheduled-tasks",
    params(ListTasksQuery),
    responses(
        (status = 200, description = "获取任务列表成功", body = ApiResponse<ListTasksResponse>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "scheduled-task"
)]
pub async fn list_tasks(
    State(state): State<Arc<ServiceContext>>,
    Query(params): Query<ListTasksQuery>,
) -> Json<ApiResponse<ListTasksResponse>> {
    match crate::controller::scheduled_task::list_tasks(&state, params).await {
        Ok(response) => Json(ApiResponse::success(response)),
        Err(e) => Json(ApiResponse::<ListTasksResponse>::error(
            "500",
            &e.to_string(),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/rsllm/api/scheduled-tasks/{task_id}",
    params(
        ("task_id" = String, Path, description = "任务ID")
    ),
    responses(
        (status = 200, description = "获取任务详情成功", body = ApiResponse<TaskResponse>),
        (status = 404, description = "任务不存在", body = ApiResponse<String>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "scheduled-task"
)]
pub async fn get_task(
    State(state): State<Arc<ServiceContext>>,
    Path(task_id): Path<String>,
) -> Json<ApiResponse<TaskResponse>> {
    match crate::controller::scheduled_task::get_task(&state, &task_id).await {
        Ok(response) => Json(ApiResponse::success(response)),
        Err(e) => Json(ApiResponse::<TaskResponse>::error("500", &e.to_string())),
    }
}

#[utoipa::path(
    put,
    path = "/rsllm/api/scheduled-tasks/{task_id}",
    params(
        ("task_id" = String, Path, description = "任务ID")
    ),
    request_body = UpdateTaskRequest,
    responses(
        (status = 200, description = "更新任务成功", body = ApiResponse<UpdateTaskResponse>),
        (status = 404, description = "任务不存在", body = ApiResponse<String>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "scheduled-task"
)]
pub async fn update_task(
    State(state): State<Arc<ServiceContext>>,
    Path(task_id): Path<String>,
    Json(params): Json<UpdateTaskRequest>,
) -> Json<ApiResponse<UpdateTaskResponse>> {
    match crate::controller::scheduled_task::update_task(&state, &task_id, params).await {
        Ok(response) => Json(ApiResponse::success(response)),
        Err(e) => Json(ApiResponse::<UpdateTaskResponse>::error(
            "500",
            &e.to_string(),
        )),
    }
}

#[utoipa::path(
    delete,
    path = "/rsllm/api/scheduled-tasks/{task_id}",
    params(
        ("task_id" = String, Path, description = "任务ID")
    ),
    responses(
        (status = 200, description = "删除任务成功", body = ApiResponse<DeleteTaskResponse>),
        (status = 404, description = "任务不存在", body = ApiResponse<String>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "scheduled-task"
)]
pub async fn delete_task(
    State(state): State<Arc<ServiceContext>>,
    Path(task_id): Path<String>,
) -> Json<ApiResponse<DeleteTaskResponse>> {
    match crate::controller::scheduled_task::delete_task(&state, &task_id).await {
        Ok(response) => Json(ApiResponse::success(response)),
        Err(e) => Json(ApiResponse::<DeleteTaskResponse>::error(
            "500",
            &e.to_string(),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/rsllm/api/scheduled-tasks/{task_id}/enable",
    params(
        ("task_id" = String, Path, description = "任务ID")
    ),
    responses(
        (status = 200, description = "启用任务成功", body = ApiResponse<EnableTaskResponse>),
        (status = 404, description = "任务不存在", body = ApiResponse<String>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "scheduled-task"
)]
pub async fn enable_task(
    State(state): State<Arc<ServiceContext>>,
    Path(task_id): Path<String>,
) -> Json<ApiResponse<EnableTaskResponse>> {
    match crate::controller::scheduled_task::enable_task(&state, &task_id).await {
        Ok(response) => Json(ApiResponse::success(response)),
        Err(e) => Json(ApiResponse::<EnableTaskResponse>::error(
            "500",
            &e.to_string(),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/rsllm/api/scheduled-tasks/{task_id}/disable",
    params(
        ("task_id" = String, Path, description = "任务ID")
    ),
    responses(
        (status = 200, description = "禁用任务成功", body = ApiResponse<DisableTaskResponse>),
        (status = 404, description = "任务不存在", body = ApiResponse<String>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "scheduled-task"
)]
pub async fn disable_task(
    State(state): State<Arc<ServiceContext>>,
    Path(task_id): Path<String>,
) -> Json<ApiResponse<DisableTaskResponse>> {
    match crate::controller::scheduled_task::disable_task(&state, &task_id).await {
        Ok(response) => Json(ApiResponse::success(response)),
        Err(e) => Json(ApiResponse::<DisableTaskResponse>::error(
            "500",
            &e.to_string(),
        )),
    }
}

#[utoipa::path(
    post,
    path = "/rsllm/api/scheduled-tasks/{task_id}/run",
    params(
        ("task_id" = String, Path, description = "任务ID")
    ),
    responses(
        (status = 200, description = "手动执行任务成功", body = ApiResponse<RunTaskResponse>),
        (status = 404, description = "任务不存在", body = ApiResponse<String>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "scheduled-task"
)]
pub async fn run_task_now(
    State(state): State<Arc<ServiceContext>>,
    Path(task_id): Path<String>,
) -> Json<ApiResponse<RunTaskResponse>> {
    match crate::controller::scheduled_task::run_task_now(&state, &task_id).await {
        Ok(response) => Json(ApiResponse::success(response)),
        Err(e) => Json(ApiResponse::<RunTaskResponse>::error("500", &e.to_string())),
    }
}

#[utoipa::path(
    get,
    path = "/rsllm/api/scheduled-tasks/logs",
    params(
        ("page" = i64, Query, description = "页码"),
        ("page_size" = i64, Query, description = "每页大小")
    ),
    responses(
        (status = 200, description = "获取所有任务执行日志成功", body = ApiResponse<PageWrapper<TaskLogResponse>>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "scheduled-task"
)]
pub async fn get_all_logs(
    State(state): State<Arc<ServiceContext>>,
    Query(params): Query<TaskLogsQuery>,
) -> Json<ApiResponse<PageWrapper<TaskLogResponse>>> {
    match crate::controller::scheduled_task::get_all_logs(&state, params).await {
        Ok(response) => Json(ApiResponse::success(response)),
        Err(e) => Json(ApiResponse::<PageWrapper<TaskLogResponse>>::error(
            "500",
            &e.to_string(),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/rsllm/api/scheduled-tasks/statistics",
    params(
        ("task_id" = String, Query, description = "任务ID（可选）")
    ),
    responses(
        (status = 200, description = "获取任务统计信息成功", body = ApiResponse<TaskStatisticsResponse>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "scheduled-task"
)]
pub async fn get_all_statistics(
    State(state): State<Arc<ServiceContext>>,
    Query(params): Query<TaskStatisticsQuery>,
) -> Json<ApiResponse<TaskStatisticsResponse>> {
    match crate::controller::scheduled_task::get_task_statistics(&state, params.task_id).await {
        Ok(response) => Json(ApiResponse::success(response)),
        Err(e) => Json(ApiResponse::<TaskStatisticsResponse>::error(
            "500",
            &e.to_string(),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/rsllm/api/scheduled-tasks/{task_id}/logs",
    params(
        ("task_id" = String, Path, description = "任务ID"),
        ("page" = i64, Query, description = "页码"),
        ("page_size" = i64, Query, description = "每页大小")
    ),
    responses(
        (status = 200, description = "获取任务执行日志成功", body = ApiResponse<PageWrapper<TaskLogResponse>>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "scheduled-task"
)]
pub async fn get_task_logs(
    State(state): State<Arc<ServiceContext>>,
    Path(task_id): Path<String>,
    Query(params): Query<TaskLogsQuery>,
) -> Json<ApiResponse<PageWrapper<TaskLogResponse>>> {
    match crate::controller::scheduled_task::get_task_logs(&state, &task_id, params).await {
        Ok(response) => Json(ApiResponse::success(response)),
        Err(e) => Json(ApiResponse::<PageWrapper<TaskLogResponse>>::error(
            "500",
            &e.to_string(),
        )),
    }
}

#[utoipa::path(
    get,
    path = "/rsllm/api/scheduled-tasks/{task_id}/statistics",
    params(
        ("task_id" = String, Path, description = "任务ID")
    ),
    responses(
        (status = 200, description = "获取任务统计信息成功", body = ApiResponse<TaskStatisticsResponse>),
        (status = 404, description = "任务不存在", body = ApiResponse<String>),
        (status = 500, description = "服务器内部错误", body = ApiResponse<String>)
    ),
    tag = "scheduled-task"
)]
pub async fn get_task_statistics(
    State(state): State<Arc<ServiceContext>>,
    Path(task_id): Path<String>,
) -> Json<ApiResponse<TaskStatisticsResponse>> {
    match crate::controller::scheduled_task::get_task_statistics(&state, Some(task_id)).await {
        Ok(response) => Json(ApiResponse::success(response)),
        Err(e) => Json(ApiResponse::<TaskStatisticsResponse>::error(
            "500",
            &e.to_string(),
        )),
    }
}

pub fn create_scheduled_task_router() -> axum::Router<Arc<ServiceContext>> {
    axum::Router::new()
        .route(
            "/rsllm/api/scheduled-tasks",
            axum::routing::post(create_task),
        )
        .route("/rsllm/api/scheduled-tasks", axum::routing::get(list_tasks))
        .route(
            "/rsllm/api/scheduled-tasks/logs",
            axum::routing::get(get_all_logs),
        )
        .route(
            "/rsllm/api/scheduled-tasks/statistics",
            axum::routing::get(get_all_statistics),
        )
        .route(
            "/rsllm/api/scheduled-tasks/{task_id}",
            axum::routing::get(get_task),
        )
        .route(
            "/rsllm/api/scheduled-tasks/{task_id}",
            axum::routing::put(update_task),
        )
        .route(
            "/rsllm/api/scheduled-tasks/{task_id}",
            axum::routing::delete(delete_task),
        )
        .route(
            "/rsllm/api/scheduled-tasks/{task_id}/enable",
            axum::routing::post(enable_task),
        )
        .route(
            "/rsllm/api/scheduled-tasks/{task_id}/disable",
            axum::routing::post(disable_task),
        )
        .route(
            "/rsllm/api/scheduled-tasks/{task_id}/run",
            axum::routing::post(run_task_now),
        )
        .route(
            "/rsllm/api/scheduled-tasks/{task_id}/logs",
            axum::routing::get(get_task_logs),
        )
        .route(
            "/rsllm/api/scheduled-tasks/{task_id}/statistics",
            axum::routing::get(get_task_statistics),
        )
}
