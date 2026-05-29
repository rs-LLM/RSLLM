//! 速率限制管理控制器模块
//! 提供速率限制查询、重置等API接口

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::domain::vo::response::ApiResponse;
use crate::error::{Error, Result};
use crate::middleware::auth::checked_token;
use crate::middleware::auth_axum::TOKEN_KEY;
use crate::service::ai_hub::RateLimitCheckResult;

use axum::debug_handler;

/// 获取用户速率限制状态
///
/// 获取指定用户的当前速率限制使用情况
#[utoipa::path(
    get,
    path = "/admin/rate-limit/{user_id}",
    params(
        ("user_id" = String, Path, description = "用户ID")
    ),
    responses(
        (status = 200, description = "查询成功", body = RateLimitStatusResponse),
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
pub async fn get_rate_limit_status(
    State(state): State<Arc<ServiceContext>>,
    Path(user_id): Path<String>,
) -> Result<Json<ApiResponse<RateLimitStatusResponse>>> {
    let result = state
        .rate_limit_service
        .get_rate_limit_status(&user_id)
        .await?;

    let level_config = state
        .rate_limit_service
        .get_user_level_config(&user_id)
        .await?;

    let response = RateLimitStatusResponse {
        user_id: user_id.clone(),
        rpm_usage: level_config.rpm_limit as f64 - result.rpm_remaining as f64,
        rpm_limit: level_config.rpm_limit as f64,
        rpm_remaining: result.rpm_remaining as f64,
        tpm_usage: level_config.tpm_limit as f64 - result.tpm_remaining as f64,
        tpm_limit: level_config.tpm_limit as f64,
        tpm_remaining: result.tpm_remaining as f64,
        is_limited: !result.allowed,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 重置用户速率限制
///
/// 重置指定用户的速率限制计数器
#[utoipa::path(
    post,
    path = "/admin/rate-limit/{user_id}/reset",
    params(
        ("user_id" = String, Path, description = "用户ID")
    ),
    responses(
        (status = 200, description = "重置成功", body = ApiResponse<String>),
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
pub async fn reset_rate_limit(
    State(state): State<Arc<ServiceContext>>,
    Path(user_id): Path<String>,
) -> Result<Json<ApiResponse<String>>> {
    state.rate_limit_service.reset_rate_limit(&user_id).await?;
    Ok(Json(ApiResponse::success(
        "Rate limit reset successfully".to_string(),
    )))
}

/// 检查用户配额
///
/// 检查用户是否超过配额限制
#[utoipa::path(
    get,
    path = "/admin/rate-limit/{user_id}/check",
    params(
        ("user_id" = String, Path, description = "用户ID")
    ),
    responses(
        (status = 200, description = "检查成功", body = RateLimitCheckResult),
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
pub async fn check_user_quota(
    State(state): State<Arc<ServiceContext>>,
    Path(user_id): Path<String>,
) -> Result<Json<ApiResponse<RateLimitCheckResult>>> {
    let result = state.rate_limit_service.check_quota(&user_id).await?;
    Ok(Json(ApiResponse::success(result)))
}

/// 获取用户等级配置
///
/// 获取指定用户的等级配置信息
#[utoipa::path(
    get,
    path = "/admin/rate-limit/{user_id}/level-config",
    params(
        ("user_id" = String, Path, description = "用户ID")
    ),
    responses(
        (status = 200, description = "查询成功", body = UserLevelConfigVO),
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
pub async fn get_user_level_config(
    State(state): State<Arc<ServiceContext>>,
    Path(user_id): Path<String>,
) -> Result<Json<ApiResponse<UserLevelConfigVO>>> {
    let config = state
        .rate_limit_service
        .get_user_level_config(&user_id)
        .await?;
    Ok(Json(ApiResponse::success(UserLevelConfigVO::from(config))))
}

/// 速率限制状态响应
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct RateLimitStatusResponse {
    pub user_id: String,
    pub rpm_usage: f64,
    pub rpm_limit: f64,
    pub rpm_remaining: f64,
    pub tpm_usage: f64,
    pub tpm_limit: f64,
    pub tpm_remaining: f64,
    pub is_limited: bool,
}

/// 用户等级配置VO
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct UserLevelConfigVO {
    pub level_code: String,
    pub level_name: String,
    pub rpm_limit: i32,
    pub tpm_limit: i32,
}

impl From<crate::domain::table::ai_hub::user_level_config::UserLevelConfig> for UserLevelConfigVO {
    fn from(config: crate::domain::table::ai_hub::user_level_config::UserLevelConfig) -> Self {
        UserLevelConfigVO {
            level_code: config.level,
            level_name: config.level_name,
            rpm_limit: config.rpm_limit,
            tpm_limit: config.tpm_limit,
        }
    }
}

/// 获取当前用户速率限制状态
///
/// 获取当前登录用户的速率限制使用情况
#[utoipa::path(
    get,
    path = "/users/rate-limit",
    responses(
        (status = 200, description = "查询成功", body = RateLimitStatusResponse),
        (status = 401, description = "未授权"),
        (status = 500, description = "服务器错误")
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn get_current_user_rate_limit(
    headers: HeaderMap,
    State(state): State<Arc<ServiceContext>>,
) -> Result<Json<ApiResponse<RateLimitStatusResponse>>> {
    let user_id = if let Some(auth_header) = headers.get(TOKEN_KEY) {
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

    let result = state
        .rate_limit_service
        .get_rate_limit_status(&user_id)
        .await?;

    let level_config = state
        .rate_limit_service
        .get_user_level_config(&user_id)
        .await?;

    let response = RateLimitStatusResponse {
        user_id: user_id.clone(),
        rpm_usage: level_config.rpm_limit as f64 - result.rpm_remaining as f64,
        rpm_limit: level_config.rpm_limit as f64,
        rpm_remaining: result.rpm_remaining as f64,
        tpm_usage: level_config.tpm_limit as f64 - result.tpm_remaining as f64,
        tpm_limit: level_config.tpm_limit as f64,
        tpm_remaining: result.tpm_remaining as f64,
        is_limited: !result.allowed,
    };

    Ok(Json(ApiResponse::success(response)))
}
