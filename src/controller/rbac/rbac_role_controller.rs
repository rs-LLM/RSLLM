use crate::context::CONTEXT;

use crate::domain::dto::IdDTO;

use crate::domain::dto::rbac::{SysRoleResAddDTO, SysRoleResPageDTO, SysRoleResUpdateDTO};

use crate::domain::vo::response::ApiResponse;

use crate::domain::vo::basic::SysRoleVO;

use crate::domain::vo::response::PageWrapper;

use axum::Json;

use axum::http::StatusCode;
use axum::response::IntoResponse;

#[utoipa::path(
    get,
    path = "/role/layer-top",
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<Vec<SysRoleVO>>),
        (status = 400, description = "查询失败", body = ApiResponse<Vec<SysRoleVO>>)
    ),
    tag = "role"
)]
pub async fn layer_top() -> impl IntoResponse {
    let vo = CONTEXT.rbac_role_service.find_all().await;
    match vo {
        Ok(data) => (StatusCode::OK, Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/role/add",
    request_body = SysRoleResAddDTO,
    responses(
        (status = 200, description = "添加成功", body = ApiResponse<u64>),
        (status = 400, description = "添加失败", body = ApiResponse<u64>)
    ),
    tag = "role"
)]
pub async fn add(arg: Json<SysRoleResAddDTO>) -> impl IntoResponse {
    let vo = CONTEXT.rbac_role_permission_service.add(&arg.0).await;
    match vo {
        Ok(data) => (StatusCode::OK, Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/role/page",
    request_body = SysRoleResPageDTO,
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<PageWrapper<crate::domain::vo::basic::SysRoleVO>>),
        (status = 400, description = "查询失败", body = ApiResponse<PageWrapper<crate::domain::vo::basic::SysRoleVO>>)
    ),
    tag = "role"
)]
pub async fn page(arg: Json<SysRoleResPageDTO>) -> impl IntoResponse {
    let vo = CONTEXT.rbac_role_permission_service.page(&arg.0).await;
    let wrapper_data: Result<PageWrapper<crate::domain::vo::basic::SysRoleVO>, _> =
        vo.map(|p| PageWrapper {
            page: p.page_no,
            page_size: p.page_size,
            total: p.total,
            records: p.records,
        });
    match wrapper_data {
        Ok(data) => (StatusCode::OK, Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/role/update",
    request_body = SysRoleResUpdateDTO,
    responses(
        (status = 200, description = "更新成功", body = ApiResponse<u64>),
        (status = 400, description = "更新失败", body = ApiResponse<u64>)
    ),
    tag = "role"
)]
pub async fn update(arg: Json<SysRoleResUpdateDTO>) -> impl IntoResponse {
    let vo = CONTEXT.rbac_role_permission_service.edit(&arg.0).await;
    match vo {
        Ok(data) => (StatusCode::OK, Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/role/remove",
    request_body = IdDTO,
    responses(
        (status = 200, description = "删除成功", body = ApiResponse<u64>),
        (status = 400, description = "删除失败", body = ApiResponse<u64>)
    ),
    tag = "role"
)]
pub async fn remove(arg: Json<IdDTO>) -> impl IntoResponse {
    let role_id = arg.0.id.unwrap_or_default();
    let vo = CONTEXT
        .rbac_role_permission_service
        .remove_role(&role_id)
        .await;
    match vo {
        Ok(data) => (StatusCode::OK, Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}
