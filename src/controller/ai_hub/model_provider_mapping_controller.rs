//! 模型-供应商映射管理控制器模块
//! 提供模型-供应商映射管理的RESTful API接口

use axum::{
    Json,
    extract::{Path, Query, State},
};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::context::ServiceContext;
use crate::domain::table::ai_hub::model_provider_mapping::ModelProviderMapping;
use crate::domain::vo::response::ApiResponse;
use crate::error::{Error, Result};

fn resolve_page(page: Option<i64>) -> Result<usize> {
    let page = page.unwrap_or(1);
    if page < 1 {
        return Err(Error::ValidationError(format!(
            "page must be >= 1, got {}",
            page
        )));
    }

    usize::try_from(page).map_err(|_| Error::ValidationError("page is too large".to_string()))
}

fn resolve_page_size(page_size: Option<i64>) -> Result<usize> {
    let page_size = page_size.unwrap_or(10);
    if page_size < 1 {
        return Err(Error::ValidationError(format!(
            "page_size must be >= 1, got {}",
            page_size
        )));
    }

    usize::try_from(page_size)
        .map_err(|_| Error::ValidationError("page_size is too large".to_string()))
}

/// 创建模型-供应商映射请求
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateModelProviderMappingDTO {
    pub model_id: String,
    pub provider_id: String,
    pub provider_model_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// 更新模型-供应商映射请求
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UpdateModelProviderMappingDTO {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_model_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// 列出映射查询参数
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ListMappingsParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
}

/// 列出映射响应
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ListMappingsResponse {
    pub items: Vec<ModelProviderMapping>,
    pub total: i64,
}

/// 创建模型-供应商映射
///
/// 创建新的模型-供应商映射关系
#[utoipa::path(
    post,
    path = "/admin/model-provider-mappings",
    request_body = CreateModelProviderMappingDTO,
    responses(
        (status = 200, description = "创建成功", body = ApiResponse<ModelProviderMapping>),
        (status = 500, description = "服务器错误", body = ApiResponse<ModelProviderMapping>)
    ),
    tag = "model-provider-mappings"
)]
pub async fn create_model_provider_mapping(
    State(_ctx): State<Arc<ServiceContext>>,
    Json(dto): Json<CreateModelProviderMappingDTO>,
) -> Result<Json<ApiResponse<ModelProviderMapping>>> {
    let rb = crate::pool!();

    let mapping = ModelProviderMapping {
        id: Some(ulid::Ulid::new().to_string()),
        model_id: dto.model_id,
        provider_id: dto.provider_id,
        provider_model_name: dto.provider_model_name,
        api_endpoint: dto.api_endpoint,
        api_key_encrypted: None,
        priority: Some(dto.priority.unwrap_or(10)),
        status: dto.status,
        created_at: None,
        updated_at: None,
    };

    ModelProviderMapping::insert(rb, &mapping)
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(Json(ApiResponse::success(mapping)))
}

/// 列出模型-供应商映射
///
/// 支持分页和按模型ID、供应商ID过滤
#[utoipa::path(
    get,
    path = "/admin/model-provider-mappings",
    params(
        ("page" = Option<i64>, Query, description = "页码"),
        ("page_size" = Option<i64>, Query, description = "每页数量"),
        ("model_id" = Option<String>, Query, description = "模型ID"),
        ("provider_id" = Option<String>, Query, description = "供应商ID")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<ListMappingsResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<ListMappingsResponse>)
    ),
    tag = "model-provider-mappings"
)]
pub async fn list_model_provider_mappings(
    State(_ctx): State<Arc<ServiceContext>>,
    Query(params): Query<ListMappingsParams>,
) -> Result<Json<ApiResponse<ListMappingsResponse>>> {
    let rb = crate::pool!();

    let page = resolve_page(params.page)?;
    let page_size = resolve_page_size(params.page_size)?;

    let mappings = if let Some(model_id) = params.model_id {
        ModelProviderMapping::select_by_model_id(rb, &model_id)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?
    } else if let Some(provider_id) = params.provider_id {
        ModelProviderMapping::select_by_provider_id(rb, &provider_id)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?
    } else {
        ModelProviderMapping::select_all(rb)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?
    };

    let total = mappings.len() as i64;
    let offset = page.saturating_sub(1).saturating_mul(page_size);
    let end = offset.saturating_add(page_size).min(mappings.len());

    let items = if offset >= mappings.len() {
        Vec::new()
    } else {
        mappings[offset..end].to_vec()
    };

    let response = ListMappingsResponse { items, total };

    Ok(Json(ApiResponse::success(response)))
}

/// 获取模型-供应商映射详情
///
/// 根据ID获取映射详情
#[utoipa::path(
    get,
    path = "/admin/model-provider-mappings/{id}",
    params(
        ("id" = String, Path, description = "映射ID")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<ModelProviderMapping>),
        (status = 404, description = "映射不存在", body = ApiResponse<ModelProviderMapping>),
        (status = 500, description = "服务器错误", body = ApiResponse<ModelProviderMapping>)
    ),
    tag = "model-provider-mappings"
)]
pub async fn get_model_provider_mapping(
    State(_ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<ModelProviderMapping>>> {
    let rb = crate::pool!();

    let mapping = ModelProviderMapping::select_by_map(rb, rbs::value! { "id": &id })
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?
        .pop();

    match mapping {
        Some(mapping) => Ok(Json(ApiResponse::success(mapping))),
        None => Err(Error::NotFound(
            "Model provider mapping not found".to_string(),
        )),
    }
}

/// 更新模型-供应商映射
///
/// 根据ID更新映射信息
#[utoipa::path(
    put,
    path = "/admin/model-provider-mappings/{id}",
    params(
        ("id" = String, Path, description = "映射ID")
    ),
    request_body = UpdateModelProviderMappingDTO,
    responses(
        (status = 200, description = "更新成功", body = ApiResponse<ModelProviderMapping>),
        (status = 404, description = "映射不存在", body = ApiResponse<ModelProviderMapping>),
        (status = 500, description = "服务器错误", body = ApiResponse<ModelProviderMapping>)
    ),
    tag = "model-provider-mappings"
)]
pub async fn update_model_provider_mapping(
    State(_ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
    Json(dto): Json<UpdateModelProviderMappingDTO>,
) -> Result<Json<ApiResponse<ModelProviderMapping>>> {
    let rb = crate::pool!();

    let existing = ModelProviderMapping::select_by_map(rb, rbs::value! { "id": &id })
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?
        .pop();

    let mut mapping = match existing {
        Some(m) => m,
        None => {
            return Err(Error::NotFound(
                "Model provider mapping not found".to_string(),
            ));
        }
    };

    if let Some(provider_model_name) = dto.provider_model_name {
        mapping.provider_model_name = provider_model_name;
    }
    if let Some(api_endpoint) = dto.api_endpoint {
        mapping.api_endpoint = Some(api_endpoint);
    }
    if let Some(priority) = dto.priority {
        mapping.priority = Some(priority);
    }
    if let Some(status) = dto.status {
        mapping.status = Some(status);
    }

    ModelProviderMapping::update_by_map(rb, &mapping, rbs::value! { "id": &id })
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(Json(ApiResponse::success(mapping)))
}

/// 删除模型-供应商映射
///
/// 根据ID删除映射
#[utoipa::path(
    delete,
    path = "/admin/model-provider-mappings/{id}",
    params(
        ("id" = String, Path, description = "映射ID")
    ),
    responses(
        (status = 200, description = "删除成功", body = ApiResponse<String>),
        (status = 404, description = "映射不存在", body = ApiResponse<String>),
        (status = 500, description = "服务器错误", body = ApiResponse<String>)
    ),
    tag = "model-provider-mappings"
)]
pub async fn delete_model_provider_mapping(
    State(_ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<String>>> {
    let rb = crate::pool!();

    let existing = ModelProviderMapping::select_by_map(rb, rbs::value! { "id": &id })
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?
        .pop();

    if existing.is_none() {
        return Err(Error::NotFound(
            "Model provider mapping not found".to_string(),
        ));
    }

    ModelProviderMapping::delete_by_map(rb, rbs::value! { "id": &id })
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(Json(ApiResponse::success(
        "Deleted successfully".to_string(),
    )))
}

/// 根据模型ID获取映射列表
///
/// 获取指定模型的所有供应商映射
#[utoipa::path(
    get,
    path = "/admin/models/{model_id}/provider-mappings",
    params(
        ("model_id" = String, Path, description = "模型ID")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<ListMappingsResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<ListMappingsResponse>)
    ),
    tag = "model-provider-mappings"
)]
pub async fn get_mappings_by_model(
    State(_ctx): State<Arc<ServiceContext>>,
    Path(model_id): Path<String>,
) -> Result<Json<ApiResponse<ListMappingsResponse>>> {
    let rb = crate::pool!();

    let mappings = ModelProviderMapping::select_by_model_id(rb, &model_id)
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    let total = mappings.len() as i64;

    let response = ListMappingsResponse {
        items: mappings,
        total,
    };

    Ok(Json(ApiResponse::success(response)))
}
