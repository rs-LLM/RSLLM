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
use crate::domain::vo::response::ApiResponse;
use crate::domain::table::ai_hub::model_definition::ModelDefinition;

use axum::debug_handler;

/// 创建模型
///
/// POST /api/v1/admin/models
#[utoipa::path(
    post,
    path = "/api/v1/admin/models",
    request_body = CreateModelRequest,
    responses(
        (status = 200, description = "创建成功", body = ApiResponse<ModelDefinition>),
        (status = 400, description = "参数错误", body = ApiResponse<ModelDefinition>),
        (status = 500, description = "服务器错误", body = ApiResponse<ModelDefinition>)
    ),
    tag = "model"
)]
#[debug_handler]
pub async fn create_model(
    State(ctx): State<Arc<ServiceContext>>,
    Json(req): Json<CreateModelRequest>,
) -> Result<Json<ApiResponse<ModelDefinition>>> {
    let service_guard = ctx.model_definition_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Model definition service not initialized".to_string())
    })?;
    
    let model = service.create_model(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// 获取模型详情
///
/// GET /api/v1/admin/models/:id
#[utoipa::path(
    get,
    path = "/api/v1/admin/models/{id}",
    params(
        ("id" = String, Path, description = "模型ID")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<ModelDefinition>),
        (status = 404, description = "模型不存在", body = ApiResponse<ModelDefinition>),
        (status = 500, description = "服务器错误", body = ApiResponse<ModelDefinition>)
    ),
    tag = "model"
)]
pub async fn get_model(
    State(ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<ModelDefinition>>> {
    let service_guard = ctx.model_definition_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Model definition service not initialized".to_string())
    })?;
    
    let model = service.get_model(&id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// 通过key获取模型详情
///
/// GET /api/v1/admin/models/key/:key
#[utoipa::path(
    get,
    path = "/api/v1/admin/models/key/{key}",
    params(
        ("key" = String, Path, description = "模型键名")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<ModelDefinition>),
        (status = 404, description = "模型不存在", body = ApiResponse<ModelDefinition>),
        (status = 500, description = "服务器错误", body = ApiResponse<ModelDefinition>)
    ),
    tag = "model"
)]
pub async fn get_model_by_key(
    State(ctx): State<Arc<ServiceContext>>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<ModelDefinition>>> {
    let service_guard = ctx.model_definition_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Model definition service not initialized".to_string())
    })?;
    
    let model = service.get_model_by_key(&key).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// 获取模型列表
///
/// GET /api/v1/admin/models
#[utoipa::path(
    get,
    path = "/api/v1/admin/models",
    params(
        ("page" = Option<i64>, Query, description = "页码"),
        ("size" = Option<i64>, Query, description = "每页数量"),
        ("provider_id" = Option<String>, Query, description = "供应商ID"),
        ("model_type" = Option<String>, Query, description = "模型类型"),
        ("enabled" = Option<bool>, Query, description = "是否启用"),
        ("status" = Option<String>, Query, description = "状态"),
        ("keyword" = Option<String>, Query, description = "关键词")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<crate::service::ai_hub::ListModelsResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<crate::service::ai_hub::ListModelsResponse>)
    ),
    tag = "model"
)]
pub async fn list_models(
    State(ctx): State<Arc<ServiceContext>>,
    Query(req): Query<ListModelsRequest>,
) -> Result<Json<ApiResponse<crate::service::ai_hub::ListModelsResponse>>> {
    let service_guard = ctx.model_definition_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Model definition service not initialized".to_string())
    })?;
    
    let response = service.list_models(req).await?;
    Ok(Json(ApiResponse::success(response)))
}

/// 更新模型
///
/// PUT /api/v1/admin/models/:id
#[utoipa::path(
    put,
    path = "/api/v1/admin/models/{id}",
    params(
        ("id" = String, Path, description = "模型ID")
    ),
    request_body = UpdateModelRequest,
    responses(
        (status = 200, description = "更新成功", body = ApiResponse<ModelDefinition>),
        (status = 404, description = "模型不存在", body = ApiResponse<ModelDefinition>),
        (status = 500, description = "服务器错误", body = ApiResponse<ModelDefinition>)
    ),
    tag = "model"
)]
pub async fn update_model(
    State(ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateModelRequest>,
) -> Result<Json<ApiResponse<ModelDefinition>>> {
    let service_guard = ctx.model_definition_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Model definition service not initialized".to_string())
    })?;
    
    let model = service.update_model(&id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// 删除模型
///
/// DELETE /api/v1/admin/models/:id
#[utoipa::path(
    delete,
    path = "/api/v1/admin/models/{id}",
    params(
        ("id" = String, Path, description = "模型ID")
    ),
    responses(
        (status = 204, description = "删除成功"),
        (status = 404, description = "模型不存在"),
        (status = 500, description = "服务器错误")
    ),
    tag = "model"
)]
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
#[utoipa::path(
    post,
    path = "/api/v1/admin/models/bulk",
    request_body = BulkImportRequest,
    responses(
        (status = 200, description = "导入成功", body = ApiResponse<crate::service::ai_hub::BulkImportResponse>),
        (status = 400, description = "参数错误", body = ApiResponse<crate::service::ai_hub::BulkImportResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<crate::service::ai_hub::BulkImportResponse>)
    ),
    tag = "model"
)]
pub async fn bulk_import_models(
    State(ctx): State<Arc<ServiceContext>>,
    Json(req): Json<BulkImportRequest>,
) -> Result<Json<ApiResponse<crate::service::ai_hub::BulkImportResponse>>> {
    let service_guard = ctx.model_definition_service.read().await;
    let service = service_guard.as_ref().ok_or_else(|| {
        Error::BusinessError("Model definition service not initialized".to_string())
    })?;
    
    let response = service.bulk_import(req).await?;
    Ok(Json(ApiResponse::success(response)))
}