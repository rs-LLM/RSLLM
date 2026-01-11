//! 用户等级模型速率限制控制器模块
//! 提供用户等级模型速率限制的API接口

use axum::{
    Json,
    extract::{Path, Query, State},
};
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::domain::dto::ai_hub::user_level_model_limit::{
    CreateUserLevelModelLimitDTO, ListUserLevelModelLimitsResponse, UpdateUserLevelModelLimitDTO,
    UserLevelModelLimitQueryDTO, UserLevelModelLimitVO,
};
use crate::domain::vo::response::ApiResponse;
use crate::error::Result;

use axum::debug_handler;

/// 创建等级模型限制
///
/// 为指定用户等级和模型创建速率限制配置
#[utoipa::path(
    post,
    path = "/admin/user-level-model-limits",
    request_body = CreateUserLevelModelLimitDTO,
    responses(
        (status = 201, description = "创建成功", body = UserLevelModelLimitVO),
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
pub async fn create_user_level_model_limit(
    State(_state): State<Arc<ServiceContext>>,
    Json(dto): Json<CreateUserLevelModelLimitDTO>,
) -> Result<Json<ApiResponse<UserLevelModelLimitVO>>> {
    let service = crate::service::ai_hub::UserLevelModelRateLimitService::new();
    let limit = service.create_limit(dto).await?;
    Ok(Json(ApiResponse::success(limit)))
}

/// 获取等级模型限制详情
///
/// 根据ID获取等级模型限制的详细信息
#[utoipa::path(
    get,
    path = "/admin/user-level-model-limits/{id}",
    params(
        ("id" = String, Path, description = "限制ID")
    ),
    responses(
        (status = 200, description = "查询成功", body = UserLevelModelLimitVO),
        (status = 400, description = "参数错误"),
        (status = 401, description = "未授权"),
        (status = 403, description = "权限不足"),
        (status = 404, description = "限制不存在"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn get_user_level_model_limit(
    State(_state): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<UserLevelModelLimitVO>>> {
    let service = crate::service::ai_hub::UserLevelModelRateLimitService::new();
    let limit = service.get_limit(&id).await?;
    Ok(Json(ApiResponse::success(limit)))
}

/// 更新等级模型限制
///
/// 更新等级模型限制的配置信息
#[utoipa::path(
    put,
    path = "/admin/user-level-model-limits/{id}",
    params(
        ("id" = String, Path, description = "限制ID")
    ),
    request_body = UpdateUserLevelModelLimitDTO,
    responses(
        (status = 200, description = "更新成功", body = UserLevelModelLimitVO),
        (status = 400, description = "参数错误"),
        (status = 401, description = "未授权"),
        (status = 403, description = "权限不足"),
        (status = 404, description = "限制不存在"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn update_user_level_model_limit(
    State(_state): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
    Json(dto): Json<UpdateUserLevelModelLimitDTO>,
) -> Result<Json<ApiResponse<UserLevelModelLimitVO>>> {
    let service = crate::service::ai_hub::UserLevelModelRateLimitService::new();
    let limit = service.update_limit(&id, dto).await?;
    Ok(Json(ApiResponse::success(limit)))
}

/// 删除等级模型限制
///
/// 删除指定的等级模型限制
#[utoipa::path(
    delete,
    path = "/admin/user-level-model-limits/{id}",
    params(
        ("id" = String, Path, description = "限制ID")
    ),
    responses(
        (status = 200, description = "删除成功", body = ApiResponse<String>),
        (status = 400, description = "参数错误", body = ApiResponse<String>),
        (status = 401, description = "未授权", body = ApiResponse<String>),
        (status = 403, description = "权限不足", body = ApiResponse<String>),
        (status = 404, description = "限制不存在", body = ApiResponse<String>),
        (status = 500, description = "服务器错误", body = ApiResponse<String>)
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn delete_user_level_model_limit(
    State(_state): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<String>>> {
    let service = crate::service::ai_hub::UserLevelModelRateLimitService::new();
    service.delete_limit(&id).await?;
    Ok(Json(ApiResponse::success("删除成功".to_string())))
}

/// 查询等级模型限制列表
///
/// 分页查询等级模型限制列表
#[utoipa::path(
    get,
    path = "/admin/user-level-model-limits",
    params(
        ("user_level" = Option<String>, Query, description = "用户等级"),
        ("model_key" = Option<String>, Query, description = "模型键"),
        ("enabled" = Option<bool>, Query, description = "是否启用"),
        ("page" = Option<u32>, Query, description = "页码"),
        ("page_size" = Option<u32>, Query, description = "每页数量")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<ListUserLevelModelLimitsResponse>),
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
pub async fn list_user_level_model_limits(
    State(_state): State<Arc<ServiceContext>>,
    Query(params): Query<UserLevelModelLimitQueryDTO>,
) -> Result<Json<ApiResponse<ListUserLevelModelLimitsResponse>>> {
    let service = crate::service::ai_hub::UserLevelModelRateLimitService::new();
    let response = service.list_limits(params).await?;
    Ok(Json(ApiResponse::success(response)))
}

/// 批量创建等级模型限制
///
/// 为指定用户等级批量创建多个模型的速率限制配置
#[utoipa::path(
    post,
    path = "/admin/user-level-model-limits/batch",
    request_body = Vec<CreateUserLevelModelLimitDTO>,
    responses(
        (status = 201, description = "创建成功", body = Vec<UserLevelModelLimitVO>),
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
pub async fn batch_create_user_level_model_limits(
    State(_state): State<Arc<ServiceContext>>,
    Json(dtos): Json<Vec<CreateUserLevelModelLimitDTO>>,
) -> Result<Json<ApiResponse<Vec<UserLevelModelLimitVO>>>> {
    let service = crate::service::ai_hub::UserLevelModelRateLimitService::new();
    let mut results = Vec::new();

    for dto in dtos {
        let limit = service.create_limit(dto).await?;
        results.push(limit);
    }

    Ok(Json(ApiResponse::success(results)))
}

/// 获取用户等级的所有模型限制
///
/// 获取指定用户等级的所有模型速率限制配置
#[utoipa::path(
    get,
    path = "/admin/user-levels/{user_level}/model-limits",
    params(
        ("user_level" = String, Path, description = "用户等级代码")
    ),
    responses(
        (status = 200, description = "查询成功", body = Vec<UserLevelModelLimitVO>),
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
pub async fn get_user_level_model_limits(
    State(_state): State<Arc<ServiceContext>>,
    Path(user_level): Path<String>,
) -> Result<Json<ApiResponse<Vec<UserLevelModelLimitVO>>>> {
    let service = crate::service::ai_hub::UserLevelModelRateLimitService::new();
    let query = UserLevelModelLimitQueryDTO {
        user_level: Some(user_level),
        model_key: None,
        enabled: None,
        page: None,
        page_size: None,
    };
    let response = service.list_limits(query).await?;
    Ok(Json(ApiResponse::success(response.items)))
}
