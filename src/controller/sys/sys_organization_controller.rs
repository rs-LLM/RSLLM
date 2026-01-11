use axum::{Json, http::StatusCode, response::IntoResponse};

use crate::context::CONTEXT;

use crate::domain::vo::response::ApiResponse;

use crate::domain::vo::response::PageWrapper;

use crate::domain::dto::basic::sys_organization::{OrgAddDTO, OrgEditDTO, OrgPageDTO};

use crate::domain::dto::IdDTO;

use crate::domain::vo::basic::sys_organization::{OrganizationTreeNodeVO, SysOrganizationVO};

#[utoipa::path(
    post,
    path = "/organization/page",
    request_body = OrgPageDTO,
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<PageWrapper<SysOrganizationVO>>),
        (status = 400, description = "查询失败", body = ApiResponse<PageWrapper<SysOrganizationVO>>)
    ),
    tag = "organization"
)]
pub async fn page(page: Json<OrgPageDTO>) -> impl IntoResponse {
    let data = CONTEXT.sys_organization_service.page(&page.0).await;
    let wrapper_data: Result<PageWrapper<SysOrganizationVO>, _> = data.map(|p| PageWrapper {
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
    get,
    path = "/organization/tree",
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<Vec<OrganizationTreeNodeVO>>),
        (status = 500, description = "查询失败", body = ApiResponse<Vec<OrganizationTreeNodeVO>>)
    ),
    tag = "organization"
)]
pub async fn tree() -> impl IntoResponse {
    let data = CONTEXT.sys_organization_service.find_tree().await;
    match data {
        Ok(data) => (StatusCode::OK, Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/organization/add",
    request_body = OrgAddDTO,
    responses(
        (status = 200, description = "添加成功", body = ApiResponse<String>),
        (status = 400, description = "添加失败", body = ApiResponse<String>)
    ),
    tag = "organization"
)]
pub async fn add(arg: Json<OrgAddDTO>) -> impl IntoResponse {
    let data = CONTEXT.sys_organization_service.add(&arg.0).await;
    match data {
        Ok(data) => (StatusCode::OK, Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/organization/update",
    request_body = OrgEditDTO,
    responses(
        (status = 200, description = "更新成功", body = ApiResponse<String>),
        (status = 400, description = "更新失败", body = ApiResponse<String>)
    ),
    tag = "organization"
)]
pub async fn update(arg: Json<OrgEditDTO>) -> impl IntoResponse {
    let data = CONTEXT.sys_organization_service.edit(&arg.0).await;
    match data {
        Ok(data) => (StatusCode::OK, Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/organization/remove",
    request_body = IdDTO,
    responses(
        (status = 200, description = "删除成功", body = ApiResponse<String>),
        (status = 400, description = "删除失败", body = ApiResponse<String>)
    ),
    tag = "organization"
)]
pub async fn remove(arg: Json<IdDTO>) -> impl IntoResponse {
    let id = arg.0.id.unwrap_or_default();
    let data = CONTEXT.sys_organization_service.remove(&id).await;
    match data {
        Ok(data) => (StatusCode::OK, Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}

#[utoipa::path(
    get,
    path = "/organization/list",
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<Vec<SysOrganizationVO>>),
        (status = 500, description = "查询失败", body = ApiResponse<Vec<SysOrganizationVO>>)
    ),
    tag = "organization"
)]
pub async fn list() -> impl IntoResponse {
    let data = CONTEXT.sys_organization_service.finds_all().await;
    match data {
        Ok(data) => (StatusCode::OK, Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/organization/detail",
    request_body = IdDTO,
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<SysOrganizationVO>),
        (status = 400, description = "查询失败", body = ApiResponse<SysOrganizationVO>)
    ),
    tag = "organization"
)]
pub async fn detail(arg: Json<IdDTO>) -> impl IntoResponse {
    let data = CONTEXT
        .sys_organization_service
        .detail(&arg.0.id.unwrap_or_default())
        .await;
    match data {
        Ok(data) => (StatusCode::OK, Json(ApiResponse::success(data))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("-1", &e.to_string())),
        ),
    }
}
