//! 模型管理控制器模块
//! 提供模型管理的RESTful API接口

use axum::{
    Json,
    extract::{Path, Query, State},
};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::context::ServiceContext;
use crate::domain::table::ai_hub::model_base::ModelBase;
use crate::domain::table::ai_hub::model_provider_mapping::ModelProviderMapping;
use crate::domain::vo::response::ApiResponse;
use crate::error::{Error, Result};

/// 列出所有模型
///
/// 支持分页和搜索过滤
#[utoipa::path(
    get,
    path = "/admin/models",
    params(
        ("page" = Option<i64>, Query, description = "页码"),
        ("page_size" = Option<i64>, Query, description = "每页数量"),
        ("search" = Option<String>, Query, description = "搜索关键词")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<ListModelsResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<ListModelsResponse>)
    ),
    tag = "models"
)]
pub async fn list_models(
    State(_ctx): State<Arc<ServiceContext>>,
    Query(params): Query<ListModelsParams>,
) -> Result<Json<ApiResponse<ListModelsResponse>>> {
    let rb = crate::pool!();

    let page = params.page.unwrap_or(1).max(1) as u64;
    let page_size = params.page_size.unwrap_or(10).max(1) as u64;
    let search = params.search.unwrap_or_default();

    let models = if search.is_empty() {
        ModelBase::select_page(rb, page, page_size)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?
    } else {
        ModelBase::search_page(rb, &search, page, page_size)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?
    };

    let total = if search.is_empty() {
        ModelBase::count_all(rb)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?
    } else {
        ModelBase::count_search(rb, &search)
            .await
            .map_err(|e| Error::DatabaseError(e.to_string()))?
    };

    let model_list: Vec<Model> = models
        .into_iter()
        .map(|model_base| Model {
            id: model_base.id.unwrap_or_default(),
            model_code: model_base.model_code,
            name: model_base.name,
            model_type: model_base.model_type,
            input_price: model_base.input_price,
            output_price: model_base.output_price,
            currency: model_base.currency.unwrap_or_default(),
            max_tokens_per_request: model_base.max_tokens_per_request,
            max_requests_per_minute: model_base.max_requests_per_minute,
            description: model_base.description,
            capabilities: model_base.capabilities,
            status: model_base.status.unwrap_or_default(),
            created_at: model_base
                .created_at
                .map(|dt| dt.unix_timestamp_millis().to_string())
                .unwrap_or_default(),
            updated_at: model_base
                .updated_at
                .map(|dt| dt.unix_timestamp_millis().to_string())
                .unwrap_or_default(),
            provider: None,
        })
        .collect();

    let response = ListModelsResponse {
        items: model_list,
        total,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 获取模型详情
///
/// 根据ID获取模型详情
#[utoipa::path(
    get,
    path = "/admin/models/{id}",
    params(
        ("id" = String, Path, description = "模型ID")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<ModelBase>),
        (status = 404, description = "模型不存在", body = ApiResponse<ModelBase>),
        (status = 500, description = "服务器错误", body = ApiResponse<ModelBase>)
    ),
    tag = "models"
)]
pub async fn get_model(
    State(_ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<ModelBase>>> {
    let rb = crate::pool!();

    let model = ModelBase::select_by_map(rb, rbs::value! { "id": &id })
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?
        .pop()
        .ok_or_else(|| Error::BusinessError("Model not found".to_string()))?;

    Ok(Json(ApiResponse::success(model)))
}

/// 创建模型
///
/// 创建新的模型配置
#[utoipa::path(
    post,
    path = "/admin/models",
    request_body = CreateModelRequest,
    responses(
        (status = 200, description = "创建成功", body = ApiResponse<ModelBase>),
        (status = 400, description = "参数错误", body = ApiResponse<ModelBase>),
        (status = 500, description = "服务器错误", body = ApiResponse<ModelBase>)
    ),
    tag = "models"
)]
pub async fn create_model(
    State(_ctx): State<Arc<ServiceContext>>,
    Json(req): Json<CreateModelRequest>,
) -> Result<Json<ApiResponse<ModelBase>>> {
    let rb = crate::pool!();

    let model = ModelBase {
        id: Some(ulid::Ulid::new().to_string()),
        model_code: req.model_code,
        name: req.name,
        model_type: req.model_type,
        input_price: req.input_price,
        output_price: req.output_price,
        currency: req.currency,
        max_tokens_per_request: req.max_tokens_per_request,
        max_requests_per_minute: req.max_requests_per_minute,
        description: req.description,
        capabilities: req.capabilities,
        status: Some("active".to_string()),
        image_token_calculation_type: req.image_token_calculation_type,
        patch_multiplier: req.patch_multiplier,
        tile_base_tokens: req.tile_base_tokens,
        tile_tokens_per_tile: req.tile_tokens_per_tile,
        audio_tokens_per_second: req.audio_tokens_per_second,
        created_at: Some(rbatis::rbdc::DateTime::now()),
        updated_at: Some(rbatis::rbdc::DateTime::now()),
    };

    ModelBase::insert(rb, &model)
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(Json(ApiResponse::success(model)))
}

/// 更新模型
///
/// 更新模型配置
#[utoipa::path(
    put,
    path = "/admin/models/{id}",
    params(
        ("id" = String, Path, description = "模型ID")
    ),
    request_body = UpdateModelRequest,
    responses(
        (status = 200, description = "更新成功", body = ApiResponse<ModelBase>),
        (status = 404, description = "模型不存在", body = ApiResponse<ModelBase>),
        (status = 500, description = "服务器错误", body = ApiResponse<ModelBase>)
    ),
    tag = "models"
)]
pub async fn update_model(
    State(_ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateModelRequest>,
) -> Result<Json<ApiResponse<ModelBase>>> {
    let rb = crate::pool!();

    let existing = ModelBase::select_by_map(rb, rbs::value! { "id": &id })
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?
        .pop()
        .ok_or_else(|| Error::BusinessError("Model not found".to_string()))?;

    let mut model = existing;
    if let Some(name) = req.name {
        model.name = name;
    }
    if let Some(model_type) = req.model_type {
        model.model_type = model_type;
    }
    if let Some(input_price) = req.input_price {
        model.input_price = input_price;
    }
    if let Some(output_price) = req.output_price {
        model.output_price = output_price;
    }
    if let Some(currency) = req.currency {
        model.currency = Some(currency);
    }
    if let Some(max_tokens_per_request) = req.max_tokens_per_request {
        model.max_tokens_per_request = Some(max_tokens_per_request);
    }
    if let Some(max_requests_per_minute) = req.max_requests_per_minute {
        model.max_requests_per_minute = Some(max_requests_per_minute);
    }
    if let Some(description) = req.description {
        model.description = Some(description);
    }
    if let Some(capabilities) = req.capabilities {
        model.capabilities = Some(capabilities);
    }
    if let Some(status) = req.status {
        model.status = Some(status);
    }
    if let Some(image_token_calculation_type) = req.image_token_calculation_type {
        model.image_token_calculation_type = Some(image_token_calculation_type);
    }
    if let Some(patch_multiplier) = req.patch_multiplier {
        model.patch_multiplier = Some(patch_multiplier);
    }
    if let Some(tile_base_tokens) = req.tile_base_tokens {
        model.tile_base_tokens = Some(tile_base_tokens);
    }
    if let Some(tile_tokens_per_tile) = req.tile_tokens_per_tile {
        model.tile_tokens_per_tile = Some(tile_tokens_per_tile);
    }
    if let Some(audio_tokens_per_second) = req.audio_tokens_per_second {
        model.audio_tokens_per_second = Some(audio_tokens_per_second);
    }

    ModelBase::update_by_map(rb, &model, rbs::value! { "id": &id })
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(Json(ApiResponse::success(model)))
}

/// 删除模型
///
/// 删除模型配置
#[utoipa::path(
    delete,
    path = "/admin/models/{id}",
    params(
        ("id" = String, Path, description = "模型ID")
    ),
    responses(
        (status = 200, description = "删除成功", body = ApiResponse<String>),
        (status = 404, description = "模型不存在", body = ApiResponse<String>),
        (status = 500, description = "服务器错误", body = ApiResponse<String>)
    ),
    tag = "models"
)]
pub async fn delete_model(
    State(_ctx): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<String>>> {
    let rb = crate::pool!();

    // 先删除对应的供应商模型关系
    ModelProviderMapping::delete_by_map(rb, rbs::value! { "model_id": &id })
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    // 再删除模型本身
    ModelBase::delete_by_map(rb, rbs::value! { "id": &id })
        .await
        .map_err(|e| Error::DatabaseError(e.to_string()))?;

    Ok(Json(ApiResponse::success("删除成功".to_string())))
}

/// 列出模型参数
#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct ListModelsParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
}

/// 列出模型响应
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ListModelsResponse {
    pub items: Vec<Model>,
    pub total: i64,
}

/// 模型信息（适配前端）
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Model {
    pub id: String,
    pub model_code: String,
    pub name: String,
    pub model_type: String,
    pub input_price: f64,
    pub output_price: f64,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_per_request: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_requests_per_minute: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<serde_json::Value>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderInfo>,
}

/// 供应商信息（适配前端）
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProviderInfo {
    pub id: String,
    pub provider_code: String,
    pub provider_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// 创建模型请求
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateModelRequest {
    pub model_code: String,
    pub name: String,
    pub model_type: String,
    pub input_price: f64,
    pub output_price: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_per_request: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_requests_per_minute: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_token_calculation_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch_multiplier: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tile_base_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tile_tokens_per_tile: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_tokens_per_second: Option<f64>,
}

/// 更新模型请求
#[derive(Debug, Clone, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct UpdateModelRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_per_request: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_requests_per_minute: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_token_calculation_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch_multiplier: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tile_base_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tile_tokens_per_tile: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_tokens_per_second: Option<f64>,
}

/// OpenAI 兼容的模型列表响应
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OpenAIModelsListResponse {
    pub object: String,
    pub data: Vec<OpenAIModelInfo>,
    pub providers: Vec<OpenAIProviderInfo>,
}

/// OpenAI 兼容的模型信息
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OpenAIModelInfo {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub owned_by: String,
    pub key: String,
}

/// OpenAI 兼容的供应商信息
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct OpenAIProviderInfo {
    pub id: String,
    pub provider_code: String,
    pub name: String,
    pub provider_type: String,
}

/// 公开模型列表响应
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PublicModelsListResponse {
    pub object: String,
    pub data: Vec<PublicModelInfo>,
    pub total: i64,
}

/// 公开模型信息
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PublicModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub provider_code: String,
    pub model_code: String,
    pub model_type: String,
    pub input_price: f64,
    pub output_price: f64,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_per_request: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_requests_per_minute: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<serde_json::Value>,
}

/// OpenAI 兼容的模型列表
///
/// 返回所有可用模型的 OpenAI 兼容格式列表
#[utoipa::path(
    get,
    path = "/api/v1/models",
    responses(
        (status = 200, description = "查询成功", body = OpenAIModelsListResponse),
        (status = 500, description = "服务器错误", body = OpenAIModelsListResponse)
    ),
    tag = "models"
)]
pub async fn list_openai_models(
    State(ctx): State<Arc<ServiceContext>>,
) -> Result<Json<OpenAIModelsListResponse>> {
    let model_router = ctx.model_router.clone();

    let models = model_router.list_all_models().await?;

    let mut provider_map = std::collections::HashMap::new();
    let mut openai_models = Vec::new();

    for model_info in models {
        let provider_id = model_info.provider.id.clone().unwrap_or_default();
        let provider_code = model_info.provider.provider_code.clone();
        let model_code = model_info.model_base.model_code.clone();

        provider_map
            .entry(provider_id.clone())
            .or_insert_with(|| OpenAIProviderInfo {
                id: provider_id.clone(),
                provider_code: provider_code.clone(),
                name: model_info.provider.name.clone(),
                provider_type: model_info.provider.provider_type.clone(),
            });

        openai_models.push(OpenAIModelInfo {
            id: format!("{}/{}", provider_code, model_code),
            object: "model".to_string(),
            created: model_info
                .model_base
                .created_at
                .map(|dt| dt.unix_timestamp_millis() / 1000)
                .unwrap_or(0),
            owned_by: provider_code.clone(),
            key: model_code,
        });
    }

    let providers: Vec<OpenAIProviderInfo> = provider_map.into_values().collect();

    Ok(Json(OpenAIModelsListResponse {
        object: "list".to_string(),
        data: openai_models,
        providers,
    }))
}

/// 公开模型列表
///
/// 返回所有可用模型的公开列表，支持分页和搜索
#[utoipa::path(
    get,
    path = "/rsllm/api/get_models",
    params(
        ("page" = Option<i64>, Query, description = "页码"),
        ("page_size" = Option<i64>, Query, description = "每页数量"),
        ("search" = Option<String>, Query, description = "搜索关键词")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<PublicModelsListResponse>),
        (status = 500, description = "服务器错误", body = ApiResponse<PublicModelsListResponse>)
    ),
    tag = "models"
)]
pub async fn list_public_models(
    State(ctx): State<Arc<ServiceContext>>,
    Query(params): Query<ListModelsParams>,
) -> Result<Json<ApiResponse<PublicModelsListResponse>>> {
    let model_router = ctx.model_router.clone();

    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(10);
    let search = params.search.unwrap_or_default();

    let models = if search.is_empty() {
        model_router.list_all_models_page(page, page_size).await?
    } else {
        model_router
            .search_models_page(&search, page, page_size)
            .await?
    };

    let public_models: Vec<PublicModelInfo> = models
        .into_iter()
        .map(|model_info| {
            let currency = model_info
                .model_base
                .currency
                .clone()
                .unwrap_or_else(|| "USD".to_string());
            PublicModelInfo {
                id: format!(
                    "{}/{}",
                    model_info.provider.provider_code, model_info.model_base.model_code
                ),
                name: model_info.model_base.name.clone(),
                provider: model_info.provider.name.clone(),
                provider_code: model_info.provider.provider_code.clone(),
                model_code: model_info.model_base.model_code.clone(),
                model_type: model_info.model_base.model_type.clone(),
                input_price: model_info.model_base.input_price,
                output_price: model_info.model_base.output_price,
                currency,
                max_tokens_per_request: model_info.model_base.max_tokens_per_request,
                max_requests_per_minute: model_info.model_base.max_requests_per_minute,
                description: model_info.model_base.description,
                capabilities: model_info.model_base.capabilities,
            }
        })
        .collect();

    let total = public_models.len() as i64;

    Ok(Json(ApiResponse::success(PublicModelsListResponse {
        object: "list".to_string(),
        data: public_models,
        total,
    })))
}
