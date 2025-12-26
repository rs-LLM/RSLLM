//! 供应商配置控制器模块
//! 提供供应商管理的RESTful API接口

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::domain::vo::response::ApiResponse;
use crate::error::{Error, Result};
use crate::service::ai_hub::{
    CreateProviderRequest, ListProvidersRequest, ProviderConfigService, UpdateProviderRequest,
};

/// 创建供应商
///
/// 创建新的供应商配置
#[utoipa::path(
    post,
    path = "/api/v1/admin/providers",
    request_body = CreateProviderRequest,
    responses(
        (status = 200, description = "创建成功", body = ApiResponse<crate::domain::table::ai_hub::provider_config::ProviderConfig>),
        (status = 400, description = "参数错误", body = ApiResponse<crate::domain::table::ai_hub::provider_config::ProviderConfig>),
        (status = 500, description = "服务器错误", body = ApiResponse<crate::domain::table::ai_hub::provider_config::ProviderConfig>)
    ),
    tag = "providers"
)]
pub async fn create_provider(
    State(ctx): State<Arc<ServiceContext>>,
    Json(req): Json<CreateProviderRequest>,
) -> Result<Json<ApiResponse<crate::domain::table::ai_hub::provider_config::ProviderConfig>>> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;
    
    let provider = service.create_provider(req).await?;
    Ok(Json(ApiResponse::success(provider)))
}

/// 获取供应商详情
///
/// 根据ID获取供应商配置详情
#[utoipa::path(
    get,
    path = "/api/v1/admin/providers/{id}",
    params(
        ("id" = String, Path, description = "供应商ID")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<crate::domain::table::ai_hub::provider_config::ProviderConfig>),
        (status = 404, description = "供应商不存在", body = ApiResponse<crate::domain::table::ai_hub::provider_config::ProviderConfig>),
        (status = 500, description = "服务器错误", body = ApiResponse<crate::domain::table::ai_hub::provider_config::ProviderConfig>)
    ),
    tag = "providers"
)]
pub async fn get_provider(
    State(ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<crate::domain::table::ai_hub::provider_config::ProviderConfig>>> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;
    
    let provider = service.get_provider(&id).await?;
    Ok(Json(ApiResponse::success(provider)))
}

/// 通过名称获取供应商详情
///
/// 根据名称获取供应商配置详情
#[utoipa::path(
    get,
    path = "/api/v1/admin/providers/name/{name}",
    params(
        ("name" = String, Path, description = "供应商名称")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<crate::domain::table::ai_hub::provider_config::ProviderConfig>),
        (status = 404, description = "供应商不存在", body = ApiResponse<crate::domain::table::ai_hub::provider_config::ProviderConfig>),
        (status = 500, description = "服务器错误", body = ApiResponse<crate::domain::table::ai_hub::provider_config::ProviderConfig>)
    ),
    tag = "providers"
)]
pub async fn get_provider_by_name(
    State(ctx): State<Arc<ServiceContext>>,
    Path(name): Path<String>,
) -> Result<Json<ApiResponse<crate::domain::table::ai_hub::provider_config::ProviderConfig>>> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;
    
    let provider = service.get_provider_by_name(&name).await?;
    Ok(Json(ApiResponse::success(provider)))
}

/// 获取供应商列表
///
/// 分页查询供应商列表
#[utoipa::path(
    get,
    path = "/api/v1/admin/providers",
    params(
        ("page" = i64, Query, description = "页码"),
        ("size" = i64, Query, description = "每页数量"),
        ("status" = Option<String>, Query, description = "状态筛选"),
        ("keyword" = Option<String>, Query, description = "关键词搜索")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<crate::service::ai_hub::ListProvidersResponse>),
        (status = 400, description = "参数错误", body = ApiResponse<crate::service::ai_hub::ListProvidersResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<crate::service::ai_hub::ListProvidersResponse>)
    ),
    tag = "providers"
)]
pub async fn list_providers(
    State(ctx): State<Arc<ServiceContext>>,
    Query(req): Query<ListProvidersRequest>,
) -> Result<Json<ApiResponse<crate::service::ai_hub::ListProvidersResponse>>> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;
    
    let response = service.list_providers(req).await?;
    Ok(Json(ApiResponse::success(response)))
}

/// 更新供应商
///
/// 更新供应商配置
#[utoipa::path(
    put,
    path = "/api/v1/admin/providers/{id}",
    params(
        ("id" = String, Path, description = "供应商ID")
    ),
    request_body = UpdateProviderRequest,
    responses(
        (status = 200, description = "更新成功", body = ApiResponse<crate::domain::table::ai_hub::provider_config::ProviderConfig>),
        (status = 400, description = "参数错误", body = ApiResponse<crate::domain::table::ai_hub::provider_config::ProviderConfig>),
        (status = 404, description = "供应商不存在", body = ApiResponse<crate::domain::table::ai_hub::provider_config::ProviderConfig>),
        (status = 500, description = "服务器错误", body = ApiResponse<crate::domain::table::ai_hub::provider_config::ProviderConfig>)
    ),
    tag = "providers"
)]
pub async fn update_provider(
    State(ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateProviderRequest>,
) -> Result<Json<ApiResponse<crate::domain::table::ai_hub::provider_config::ProviderConfig>>> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;
    
    let provider = service.update_provider(&id, req).await?;
    Ok(Json(ApiResponse::success(provider)))
}

/// 删除供应商
///
/// 删除指定的供应商配置
#[utoipa::path(
    delete,
    path = "/api/v1/admin/providers/{id}",
    params(
        ("id" = String, Path, description = "供应商ID")
    ),
    responses(
        (status = 204, description = "删除成功"),
        (status = 404, description = "供应商不存在"),
        (status = 500, description = "服务器错误")
    ),
    tag = "providers"
)]
pub async fn delete_provider(
    State(ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;
    
    service.delete_provider(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}