//! 模型定义控制器模块
//! 提供模型管理的RESTful API接口

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::error::{Error, Result};
use crate::service::ai_hub::{
    CreateModelRequest, ListModelsRequest, ModelDefinitionService, UpdateModelRequest,
    BulkImportRequest,
};

use axum::debug_handler;

/// 创建模型
///
/// POST /api/v1/admin/models
#[debug_handler]
pub async fn create_model(
    State(ctx): State<Arc<ServiceContext>>,
    Json(req): Json<CreateModelRequest>,
) -> Result<Json<crate::domain::table::ai_hub::model_definition::ModelDefinition>> {
    let service_guard = ctx.model_definition_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Model definition service not initialized".to_string())
    })?;
    
    let model = service.create_model(req).await?;
    Ok(Json(model))
}

/// 获取模型详情
///
/// GET /api/v1/admin/models/:id
pub async fn get_model(
    State(ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
) -> Result<Json<crate::domain::table::ai_hub::model_definition::ModelDefinition>> {
    let service_guard = ctx.model_definition_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Model definition service not initialized".to_string())
    })?;
    
    let model = service.get_model(&id).await?;
    Ok(Json(model))
}

/// 通过key获取模型详情
///
/// GET /api/v1/admin/models/key/:key
pub async fn get_model_by_key(
    State(ctx): State<Arc<ServiceContext>>,
    Path(key): Path<String>,
) -> Result<Json<crate::domain::table::ai_hub::model_definition::ModelDefinition>> {
    let service_guard = ctx.model_definition_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Model definition service not initialized".to_string())
    })?;
    
    let model = service.get_model_by_key(&key).await?;
    Ok(Json(model))
}

/// 获取模型列表
///
/// GET /api/v1/admin/models
pub async fn list_models(
    State(ctx): State<Arc<ServiceContext>>,
    Query(req): Query<ListModelsRequest>,
) -> Result<Json<crate::service::ai_hub::ListModelsResponse>> {
    let service_guard = ctx.model_definition_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Model definition service not initialized".to_string())
    })?;
    
    let response = service.list_models(req).await?;
    Ok(Json(response))
}

/// 更新模型
///
/// PUT /api/v1/admin/models/:id
pub async fn update_model(
    State(ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateModelRequest>,
) -> Result<Json<crate::domain::table::ai_hub::model_definition::ModelDefinition>> {
    let service_guard = ctx.model_definition_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Model definition service not initialized".to_string())
    })?;
    
    let model = service.update_model(&id, req).await?;
    Ok(Json(model))
}

/// 删除模型
///
/// DELETE /api/v1/admin/models/:id
pub async fn delete_model(
    State(ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    let service_guard = ctx.model_definition_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Model definition service not initialized".to_string())
    })?;
    
    service.delete_model(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// 批量导入模型
///
/// POST /api/v1/admin/models/bulk
pub async fn bulk_import_models(
    State(ctx): State<Arc<ServiceContext>>,
    Json(req): Json<BulkImportRequest>,
) -> Result<Json<crate::service::ai_hub::BulkImportResponse>> {
    let service_guard = ctx.model_definition_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Model definition service not initialized".to_string())
    })?;
    
    let response = service.bulk_import(req).await?;
    Ok(Json(response))
}