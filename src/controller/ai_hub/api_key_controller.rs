//! API密钥管理控制器模块
//! 提供API密钥创建、查询、更新、删除等API接口

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::domain::dto::ai_hub::api_key::{
    ApiKeyQueryDTO, ApiKeyResponse, CreateApiKeyDTO, ListApiKeysResponse, UpdateApiKeyDTO,
};
use crate::domain::vo::response::ApiResponse;
use crate::error::{Error, Result};
use crate::middleware::auth::checked_token;

use axum::debug_handler;

/// 创建API密钥
///
/// 为当前用户创建新的API密钥
#[utoipa::path(
    post,
    path = "/rsllm/users/api-keys",
    request_body = CreateApiKeyDTO,
    responses(
        (status = 201, description = "创建成功", body = ApiResponse<ApiKeyResponse>),
        (status = 400, description = "参数错误"),
        (status = 401, description = "未授权"),
        (status = 403, description = "权限不足"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn create_api_key(
    headers: HeaderMap,
    State(state): State<Arc<ServiceContext>>,
    Json(dto): Json<CreateApiKeyDTO>,
) -> Result<Json<ApiResponse<ApiKeyResponse>>> {
    const TOKEN_KEY: &str = "Authorization";

    let current_user_id = if let Some(auth_header) = headers.get(TOKEN_KEY) {
        if let Ok(auth_str) = auth_header.to_str() {
            let token = auth_str.trim_start_matches("Bearer ");
            if let Ok(jwt_token) = checked_token(token) {
                jwt_token.id
            } else {
                return Err(Error::BusinessError("无效的访问令牌".to_string()));
            }
        } else {
            return Err(Error::BusinessError("缺少访问令牌".to_string()));
        }
    } else {
        return Err(Error::BusinessError("缺少访问令牌".to_string()));
    };

    let api_key = state
        .api_key_service
        .create_api_key(dto, current_user_id)
        .await?;
    Ok(Json(ApiResponse::success(api_key)))
}

/// 获取API密钥详情
///
/// 根据ID获取API密钥的详细信息（仅限当前用户自己的API Key）
#[utoipa::path(
    get,
    path = "/rsllm/users/api-keys/{id}",
    params(
        ("id" = String, Path, description = "API密钥ID")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiKeyResponse),
        (status = 400, description = "参数错误"),
        (status = 401, description = "未授权"),
        (status = 403, description = "权限不足"),
        (status = 404, description = "API密钥不存在"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn get_api_key(
    headers: HeaderMap,
    State(state): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<ApiKeyResponse>>> {
    const TOKEN_KEY: &str = "Authorization";

    let current_user_id = if let Some(auth_header) = headers.get(TOKEN_KEY) {
        if let Ok(auth_str) = auth_header.to_str() {
            let token = auth_str.trim_start_matches("Bearer ");
            if let Ok(jwt_token) = checked_token(token) {
                jwt_token.id
            } else {
                return Err(Error::BusinessError("无效的访问令牌".to_string()));
            }
        } else {
            return Err(Error::BusinessError("缺少访问令牌".to_string()));
        }
    } else {
        return Err(Error::BusinessError("缺少访问令牌".to_string()));
    };

    let api_key = state.api_key_service.get_api_key(&id).await?;

    if api_key.user_id != current_user_id {
        return Err(Error::BusinessError("无权访问此API Key".to_string()));
    }

    Ok(Json(ApiResponse::success(api_key)))
}

/// 更新API密钥
///
/// 更新API密钥的配置信息（不包括密钥本身，仅限当前用户自己的API Key）
#[utoipa::path(
    put,
    path = "/rsllm/users/api-keys/{id}",
    params(
        ("id" = String, Path, description = "API密钥ID")
    ),
    request_body = UpdateApiKeyDTO,
    responses(
        (status = 200, description = "更新成功", body = ApiKeyResponse),
        (status = 400, description = "参数错误"),
        (status = 401, description = "未授权"),
        (status = 403, description = "权限不足"),
        (status = 404, description = "API密钥不存在"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn update_api_key(
    headers: HeaderMap,
    State(state): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
    Json(dto): Json<UpdateApiKeyDTO>,
) -> Result<Json<ApiResponse<ApiKeyResponse>>> {
    const TOKEN_KEY: &str = "Authorization";

    let current_user_id = if let Some(auth_header) = headers.get(TOKEN_KEY) {
        if let Ok(auth_str) = auth_header.to_str() {
            let token = auth_str.trim_start_matches("Bearer ");
            if let Ok(jwt_token) = checked_token(token) {
                jwt_token.id
            } else {
                return Err(Error::BusinessError("无效的访问令牌".to_string()));
            }
        } else {
            return Err(Error::BusinessError("缺少访问令牌".to_string()));
        }
    } else {
        return Err(Error::BusinessError("缺少访问令牌".to_string()));
    };

    let existing_key = state.api_key_service.get_api_key(&id).await?;

    if existing_key.user_id != current_user_id {
        return Err(Error::BusinessError("无权修改此API Key".to_string()));
    }

    let api_key = state.api_key_service.update_api_key(&id, dto).await?;
    Ok(Json(ApiResponse::success(api_key)))
}

/// 删除API密钥
///
/// 删除指定的API密钥（仅限当前用户自己的API Key）
#[utoipa::path(
    delete,
    path = "/rsllm/users/api-keys/{id}",
    params(
        ("id" = String, Path, description = "API密钥ID")
    ),
    responses(
        (status = 204, description = "删除成功"),
        (status = 400, description = "参数错误"),
        (status = 401, description = "未授权"),
        (status = 403, description = "权限不足"),
        (status = 404, description = "API密钥不存在"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn delete_api_key(
    headers: HeaderMap,
    State(state): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<String>>> {
    const TOKEN_KEY: &str = "Authorization";

    let current_user_id = if let Some(auth_header) = headers.get(TOKEN_KEY) {
        if let Ok(auth_str) = auth_header.to_str() {
            let token = auth_str.trim_start_matches("Bearer ");
            if let Ok(jwt_token) = checked_token(token) {
                jwt_token.id
            } else {
                return Err(Error::BusinessError("无效的访问令牌".to_string()));
            }
        } else {
            return Err(Error::BusinessError("缺少访问令牌".to_string()));
        }
    } else {
        return Err(Error::BusinessError("缺少访问令牌".to_string()));
    };

    let existing_key = state.api_key_service.get_api_key(&id).await?;

    if existing_key.user_id != current_user_id {
        return Err(Error::BusinessError("无权删除此API Key".to_string()));
    }

    state.api_key_service.delete_api_key(&id).await?;
    Ok(Json(ApiResponse::success("删除成功".to_string())))
}

/// 查询API密钥列表
///
/// 分页查询当前用户的API密钥列表
#[utoipa::path(
    get,
    path = "/rsllm/users/api-keys",
    params(
        ("status" = Option<String>, Query, description = "状态"),
        ("page" = Option<u32>, Query, description = "页码"),
        ("page_size" = Option<u32>, Query, description = "每页数量")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<ListApiKeysResponse>),
        (status = 400, description = "参数错误"),
        (status = 401, description = "未授权"),
        (status = 403, description = "权限不足"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn list_api_keys(
    headers: HeaderMap,
    State(state): State<Arc<ServiceContext>>,
    Query(mut params): Query<ApiKeyQueryDTO>,
) -> Result<Json<ApiResponse<ListApiKeysResponse>>> {
    const TOKEN_KEY: &str = "Authorization";

    let current_user_id = if let Some(auth_header) = headers.get(TOKEN_KEY) {
        if let Ok(auth_str) = auth_header.to_str() {
            let token = auth_str.trim_start_matches("Bearer ");
            if let Ok(jwt_token) = checked_token(token) {
                jwt_token.id
            } else {
                return Err(Error::BusinessError("无效的访问令牌".to_string()));
            }
        } else {
            return Err(Error::BusinessError("缺少访问令牌".to_string()));
        }
    } else {
        return Err(Error::BusinessError("缺少访问令牌".to_string()));
    };

    params.user_id = Some(current_user_id);
    let response = state.api_key_service.list_api_keys(params).await?;
    Ok(Json(ApiResponse::success(response)))
}

/// 验证API密钥
///
/// 验证API密钥的有效性
#[utoipa::path(
    post,
    path = "/admin/api-keys/validate",
    request_body = ApiKeyValidateRequest,
    responses(
        (status = 200, description = "验证成功", body = ApiResponse<ApiKeyValidateResponse>),
        (status = 400, description = "参数错误"),
        (status = 401, description = "未授权"),
        (status = 403, description = "权限不足"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn validate_api_key(
    State(state): State<Arc<ServiceContext>>,
    Json(request): Json<ApiKeyValidateRequest>,
) -> Result<Json<ApiResponse<ApiKeyValidateResponse>>> {
    let result = state
        .api_key_service
        .validate_api_key(&request.api_key)
        .await?;

    let response = ApiKeyValidateResponse {
        valid: result.valid,
        user_id: result.user_id,
        prefix: result.api_key_id,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// API密钥验证请求
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct ApiKeyValidateRequest {
    pub api_key: String,
}

/// API密钥验证响应
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ApiKeyValidateResponse {
    pub valid: bool,
    pub user_id: Option<String>,
    pub prefix: Option<String>,
}
