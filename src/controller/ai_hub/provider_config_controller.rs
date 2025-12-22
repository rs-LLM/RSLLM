//! 供应商配置控制器模块
//! 提供供应商管理的RESTful API接口

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::error::{Error, Result};
use crate::service::ai_hub::{
    CreateProviderRequest, ListProvidersRequest, ProviderConfigService, UpdateProviderRequest,
};

/// 创建供应商
///
/// POST /api/v1/admin/providers
pub async fn create_provider(
    State(ctx): State<Arc<ServiceContext>>,
    Json(req): Json<CreateProviderRequest>,
) -> Result<Json<crate::domain::table::ai_hub::provider_config::ProviderConfig>> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;
    
    let provider = service.create_provider(req).await?;
    Ok(Json(provider))
}

/// 获取供应商详情
///
/// GET /api/v1/admin/providers/:id
pub async fn get_provider(
    State(ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
) -> Result<Json<crate::domain::table::ai_hub::provider_config::ProviderConfig>> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;
    
    let provider = service.get_provider(&id).await?;
    Ok(Json(provider))
}

/// 通过名称获取供应商详情
///
/// GET /api/v1/admin/providers/name/:name
pub async fn get_provider_by_name(
    State(ctx): State<Arc<ServiceContext>>,
    Path(name): Path<String>,
) -> Result<Json<crate::domain::table::ai_hub::provider_config::ProviderConfig>> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;
    
    let provider = service.get_provider_by_name(&name).await?;
    Ok(Json(provider))
}

/// 获取供应商列表
///
/// GET /api/v1/admin/providers
pub async fn list_providers(
    State(ctx): State<Arc<ServiceContext>>,
    Query(req): Query<ListProvidersRequest>,
) -> Result<Json<crate::service::ai_hub::ListProvidersResponse>> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;
    
    let response = service.list_providers(req).await?;
    Ok(Json(response))
}

/// 更新供应商
///
/// PUT /api/v1/admin/providers/:id
pub async fn update_provider(
    State(ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateProviderRequest>,
) -> Result<Json<crate::domain::table::ai_hub::provider_config::ProviderConfig>> {
    let service_guard = ctx.provider_config_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Provider config service not initialized".to_string())
    })?;
    
    let provider = service.update_provider(&id, req).await?;
    Ok(Json(provider))
}

/// 删除供应商
///
/// DELETE /api/v1/admin/providers/:id
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