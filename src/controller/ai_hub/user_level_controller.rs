//! 用户等级管理控制器模块
//! 提供用户等级查询、更新等API接口

use axum::{Json, extract::State};
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::domain::dto::{
    BatchDeleteUserLevelsDTO, BatchUpdateUserLevelStatusDTO, CreateUserLevelDTO,
    ListUserLevelsResponse, QueryUserLevelsDTO, UpdateUserLevelConfigDTO, UserLevelConfigVO,
    UserLevelDetailVO, UserLevelListResponse, UserLevelVO,
};
use crate::domain::vo::response::ApiResponse;
use crate::error::Result;
use crate::middleware::auth_axum::JwtAuth;
use crate::service::ai_hub::UpdateUserLevelDTO;

use axum::debug_handler;

/// 获取所有用户等级配置
///
/// 获取系统中所有可用的用户等级配置列表
#[utoipa::path(
    get,
    path = "/user-levels",
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<ListUserLevelsResponse>),
        (status = 401, description = "未授权"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn get_all_user_levels(
    State(state): State<Arc<ServiceContext>>,
) -> Result<Json<ApiResponse<ListUserLevelsResponse>>> {
    let levels = state.user_level_service.get_all_levels().await?;
    let total = levels.len() as i64;
    let response = ListUserLevelsResponse {
        items: levels,
        total,
    };
    Ok(Json(ApiResponse::success(response)))
}

/// 根据等级代码获取等级配置
///
/// 获取指定等级代码的详细配置信息
#[utoipa::path(
    get,
    path = "/user-levels/{level_code}",
    params(
        ("level_code" = String, Path, description = "等级代码，如L1, L2, L3, L4, L5")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<UserLevelConfigVO>),
        (status = 401, description = "未授权"),
        (status = 404, description = "等级不存在"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn get_user_level_by_code(
    State(state): State<Arc<ServiceContext>>,
    axum::extract::Path(level_code): axum::extract::Path<String>,
) -> Result<Json<ApiResponse<UserLevelConfigVO>>> {
    let level = state
        .user_level_service
        .get_level_by_code(&level_code)
        .await?;
    Ok(Json(ApiResponse::success(level)))
}

/// 获取用户当前等级
///
/// 获取指定用户的当前等级
#[utoipa::path(
    get,
    path = "/users/{user_id}/level",
    params(
        ("user_id" = String, Path, description = "用户ID")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<String>),
        (status = 401, description = "未授权"),
        (status = 404, description = "用户不存在"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn get_user_level(
    State(state): State<Arc<ServiceContext>>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> Result<Json<ApiResponse<String>>> {
    let level = state.user_level_service.get_user_level(&user_id).await?;
    Ok(Json(ApiResponse::success(level)))
}

/// 更新用户等级
///
/// 更新指定用户的等级，管理员权限
#[utoipa::path(
    put,
    path = "/users/{user_id}/level",
    params(
        ("user_id" = String, Path, description = "用户ID")
    ),
    request_body = UpdateUserLevelDTO,
    responses(
        (status = 200, description = "更新成功", body = ApiResponse<String>),
        (status = 400, description = "参数错误"),
        (status = 401, description = "未授权"),
        (status = 403, description = "权限不足"),
        (status = 404, description = "用户或等级不存在"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn update_user_level(
    State(state): State<Arc<ServiceContext>>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
    Json(dto): Json<UpdateUserLevelDTO>,
) -> Result<Json<ApiResponse<String>>> {
    state
        .user_level_service
        .update_user_level(&user_id, dto)
        .await?;
    state
        .rate_limit_service
        .invalidate_user_level_cache(&user_id)
        .await;
    Ok(Json(ApiResponse::success(
        "User level updated successfully".to_string(),
    )))
}

/// 初始化默认用户等级配置
///
/// 初始化系统默认的5个用户等级配置（L1-L5）
#[utoipa::path(
    post,
    path = "/user-levels/init",
    responses(
        (status = 200, description = "初始化成功", body = ApiResponse<String>),
        (status = 401, description = "未授权"),
        (status = 403, description = "权限不足"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn init_user_levels(
    State(state): State<Arc<ServiceContext>>,
) -> Result<Json<ApiResponse<String>>> {
    state.user_level_service.init_default_levels().await?;
    state.rate_limit_service.clear_user_level_cache().await;
    Ok(Json(ApiResponse::success(
        "User levels initialized successfully".to_string(),
    )))
}

/// 获取当前登录用户的等级信息
///
/// 获取当前登录用户的等级及速率限制信息
#[utoipa::path(
    get,
    path = "/users/level",
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<UserLevelVO>),
        (status = 401, description = "未授权"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn get_current_user_level(
    State(state): State<Arc<ServiceContext>>,
    jwt_auth: JwtAuth,
) -> Result<Json<ApiResponse<UserLevelVO>>> {
    let level = state
        .user_level_service
        .get_user_level_info(&jwt_auth.id)
        .await?;
    Ok(Json(ApiResponse::success(level)))
}

/// 更新用户等级配置
///
/// 更新指定用户等级的配置信息
#[utoipa::path(
    put,
    path = "/user-levels/{id}",
    params(
        ("id" = String, Path, description = "用户等级配置ID")
    ),
    request_body = UpdateUserLevelConfigDTO,
    responses(
        (status = 200, description = "更新成功", body = ApiResponse<UserLevelConfigVO>),
        (status = 400, description = "参数错误"),
        (status = 401, description = "未授权"),
        (status = 404, description = "等级配置不存在"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn update_user_level_config(
    State(state): State<Arc<ServiceContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(dto): Json<UpdateUserLevelConfigDTO>,
) -> Result<Json<ApiResponse<UserLevelConfigVO>>> {
    let dto_with_id = UpdateUserLevelConfigDTO {
        id,
        level_name: dto.level_name,
        description: dto.description,
        rpm_limit: dto.rpm_limit,
        tpm_limit: dto.tpm_limit,
        enabled: dto.enabled,
    };
    let level = state.user_level_service.update_level(dto_with_id).await?;
    state.rate_limit_service.clear_user_level_cache().await;
    Ok(Json(ApiResponse::success(level)))
}

/// 创建用户等级
///
/// 创建新的用户等级配置
#[utoipa::path(
    post,
    path = "/user-levels",
    request_body = CreateUserLevelDTO,
    responses(
        (status = 200, description = "创建成功", body = ApiResponse<UserLevelConfigVO>),
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
pub async fn create_user_level(
    State(state): State<Arc<ServiceContext>>,
    Json(dto): Json<CreateUserLevelDTO>,
) -> Result<Json<ApiResponse<UserLevelConfigVO>>> {
    let level = state.user_level_service.create_level(dto).await?;
    state.rate_limit_service.clear_user_level_cache().await;
    Ok(Json(ApiResponse::success(level)))
}

/// 分页查询用户等级
///
/// 分页查询用户等级配置列表，支持按启用状态过滤
#[utoipa::path(
    get,
    path = "/user-levels",
    params(
        ("page" = Option<i32>, Query, description = "页码，默认1"),
        ("page_size" = Option<i32>, Query, description = "每页数量，默认20"),
        ("enabled" = Option<bool>, Query, description = "是否启用")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<UserLevelListResponse>),
        (status = 401, description = "未授权"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn query_user_levels(
    State(state): State<Arc<ServiceContext>>,
    axum::extract::Query(dto): axum::extract::Query<QueryUserLevelsDTO>,
) -> Result<Json<ApiResponse<UserLevelListResponse>>> {
    let response = state.user_level_service.query_levels(dto).await?;
    Ok(Json(ApiResponse::success(response)))
}

/// 获取用户等级详情
///
/// 获取指定用户等级的详细信息，包含模型限制
#[utoipa::path(
    get,
    path = "/user-levels/{id}/detail",
    params(
        ("id" = String, Path, description = "用户等级配置ID")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<UserLevelDetailVO>),
        (status = 401, description = "未授权"),
        (status = 404, description = "等级配置不存在"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn get_user_level_detail(
    State(state): State<Arc<ServiceContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<ApiResponse<UserLevelDetailVO>>> {
    let detail = state.user_level_service.get_level_detail(&id).await?;
    Ok(Json(ApiResponse::success(detail)))
}

/// 批量更新用户等级状态
///
/// 批量更新多个用户等级的启用/禁用状态
#[utoipa::path(
    put,
    path = "/user-levels/batch/status",
    request_body = BatchUpdateUserLevelStatusDTO,
    responses(
        (status = 200, description = "更新成功", body = ApiResponse<String>),
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
pub async fn batch_update_user_level_status(
    State(state): State<Arc<ServiceContext>>,
    Json(dto): Json<BatchUpdateUserLevelStatusDTO>,
) -> Result<Json<ApiResponse<String>>> {
    state.user_level_service.batch_update_status(dto).await?;
    Ok(Json(ApiResponse::success(
        "Batch status update successful".to_string(),
    )))
}

/// 删除用户等级
///
/// 删除指定的用户等级配置
#[utoipa::path(
    delete,
    path = "/user-levels/{id}",
    params(
        ("id" = String, Path, description = "用户等级配置ID")
    ),
    responses(
        (status = 200, description = "删除成功", body = ApiResponse<String>),
        (status = 401, description = "未授权"),
        (status = 403, description = "权限不足"),
        (status = 404, description = "等级配置不存在"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn delete_user_level(
    State(state): State<Arc<ServiceContext>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<ApiResponse<String>>> {
    state.user_level_service.delete_level(&id).await?;
    Ok(Json(ApiResponse::success(
        "User level deleted successfully".to_string(),
    )))
}

/// 批量删除用户等级
///
/// 批量删除多个用户等级配置
#[utoipa::path(
    delete,
    path = "/user-levels/batch",
    request_body = BatchDeleteUserLevelsDTO,
    responses(
        (status = 200, description = "删除成功", body = ApiResponse<String>),
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
pub async fn batch_delete_user_levels(
    State(state): State<Arc<ServiceContext>>,
    Json(dto): Json<BatchDeleteUserLevelsDTO>,
) -> Result<Json<ApiResponse<String>>> {
    state.user_level_service.batch_delete(dto).await?;
    Ok(Json(ApiResponse::success(
        "Batch delete successful".to_string(),
    )))
}
