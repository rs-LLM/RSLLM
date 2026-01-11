use crate::context::CONTEXT;
use crate::domain::dto::IdDTO;
use crate::domain::dto::rbac::{PermissionAddDTO, PermissionPageDTO, ResEditDTO};
use crate::domain::table::rbac::RbacPermission;
use crate::domain::vo::rbac::{RbacPermissionAuditLogVO, RbacPermissionVO};
use crate::domain::vo::response::{ApiResponse, PageWrapper};
use axum::Json;
use axum::response::IntoResponse;

#[utoipa::path(
    post,
    path = "/admin/sys_permission_layer_top",
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<Vec<RbacPermissionVO>>),
        (status = 400, description = "查询失败", body = ApiResponse<Vec<RbacPermissionVO>>)
    ),
    tag = "permission"
)]
pub async fn layer_top() -> impl IntoResponse {
    use axum::http::StatusCode;
    let data = CONTEXT.rbac_permission_service.finds_all().await;
    match data {
        Ok(data) => (StatusCode::OK, axum::Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            axum::Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/admin/sys_permission_page",
    request_body = PermissionPageDTO,
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<PageWrapper<RbacPermissionVO>>),
        (status = 400, description = "查询失败", body = ApiResponse<PageWrapper<RbacPermissionVO>>)
    ),
    tag = "permission"
)]
pub async fn page(page: Json<PermissionPageDTO>) -> impl IntoResponse {
    use axum::http::StatusCode;
    let data = CONTEXT.rbac_permission_service.page(&page.0).await;
    let wrapper_data: Result<PageWrapper<RbacPermissionVO>, _> = data.map(|p| PageWrapper {
        page: p.page_no,
        page_size: p.page_size,
        total: p.total,
        records: p.records,
    });
    match wrapper_data {
        Ok(data) => (StatusCode::OK, axum::Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            axum::Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/admin/sys_permission_add",
    request_body = PermissionAddDTO,
    responses(
        (status = 200, description = "添加成功", body = ApiResponse<u64>),
        (status = 400, description = "添加失败", body = ApiResponse<u64>)
    ),
    tag = "permission"
)]
pub async fn add(mut arg: Json<PermissionAddDTO>) -> impl IntoResponse {
    use axum::http::StatusCode;
    if arg.name.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(ApiResponse::error("-1", "arg.name_empty")),
        );
    }
    if arg.permission.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(ApiResponse::error("-1", "arg.permission_empty")),
        );
    }
    if arg.path.is_none() {
        arg.path = Some("".to_string());
    }
    let res = RbacPermission::from(arg.0);
    let data = CONTEXT.rbac_permission_service.add(&res).await;
    match data {
        Ok(data) => (StatusCode::OK, axum::Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            axum::Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/admin/sys_permission_update",
    request_body = ResEditDTO,
    responses(
        (status = 200, description = "更新成功", body = ApiResponse<u64>),
        (status = 400, description = "更新失败", body = ApiResponse<u64>)
    ),
    tag = "permission"
)]
pub async fn update(arg: Json<ResEditDTO>) -> impl IntoResponse {
    use axum::http::StatusCode;
    let data = CONTEXT.rbac_permission_service.edit(&arg.0).await;
    match data {
        Ok(data) => (StatusCode::OK, axum::Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            axum::Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/admin/sys_permission_remove",
    request_body = IdDTO,
    responses(
        (status = 200, description = "删除成功", body = ApiResponse<u64>),
        (status = 400, description = "删除失败", body = ApiResponse<u64>)
    ),
    tag = "permission"
)]
pub async fn remove(arg: Json<IdDTO>) -> impl IntoResponse {
    use axum::http::StatusCode;
    let data = CONTEXT
        .rbac_permission_service
        .remove(&arg.0.id.unwrap_or_default())
        .await;
    match data {
        Ok(data) => (StatusCode::OK, axum::Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            axum::Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/admin/sys_permission_query_by_type",
    request_body = String,
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<Vec<RbacPermissionVO>>),
        (status = 400, description = "查询失败", body = ApiResponse<Vec<RbacPermissionVO>>)
    ),
    tag = "permission"
)]
pub async fn query_by_type(permission_type: Json<String>) -> impl IntoResponse {
    use axum::http::StatusCode;
    let data = CONTEXT
        .rbac_permission_service
        .find_by_type(&permission_type.0)
        .await;
    let vo_data: Result<Vec<RbacPermissionVO>, _> = data.map(|permissions| {
        permissions
            .into_iter()
            .map(RbacPermissionVO::from)
            .collect()
    });
    match vo_data {
        Ok(data) => (StatusCode::OK, axum::Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            axum::Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/admin/sys_permission_update_status",
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "更新成功", body = ApiResponse<u64>),
        (status = 400, description = "更新失败", body = ApiResponse<u64>)
    ),
    tag = "permission"
)]
pub async fn update_status(arg: Json<serde_json::Value>) -> impl IntoResponse {
    use axum::http::StatusCode;
    let id = arg.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let status = arg.get("status").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
    let data = CONTEXT
        .rbac_permission_service
        .update_status(id, status)
        .await;
    match data {
        Ok(data) => (StatusCode::OK, axum::Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            axum::Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/admin/sys_permission_audit_logs",
    request_body = IdDTO,
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<Vec<RbacPermissionAuditLogVO>>),
        (status = 400, description = "查询失败", body = ApiResponse<Vec<RbacPermissionAuditLogVO>>)
    ),
    tag = "permission"
)]
pub async fn query_audit_logs(arg: Json<IdDTO>) -> impl IntoResponse {
    use axum::http::StatusCode;
    let permission_id = arg.0.id.unwrap_or_default();
    let data = CONTEXT
        .rbac_permission_audit_log_service
        .query_logs(&permission_id)
        .await;
    let vo_data: Result<Vec<RbacPermissionAuditLogVO>, _> = data.map(|logs| {
        logs.into_iter()
            .map(RbacPermissionAuditLogVO::from)
            .collect()
    });
    match vo_data {
        Ok(data) => (StatusCode::OK, axum::Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            axum::Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}
