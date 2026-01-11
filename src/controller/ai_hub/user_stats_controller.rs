//! 用户统计控制器模块
//! 提供用户面板统计API接口

use axum::{Json, extract::Query};
use serde::Deserialize;

use crate::domain::vo::ai_hub::admin_stats::TimeDimension;
use crate::domain::vo::ai_hub::user_stats::*;
use crate::domain::vo::response::ApiResponse;
use crate::error::Result;
use crate::middleware::auth_axum::JwtAuth;
use crate::service::ai_hub::UserStatsService;

use axum::debug_handler;

/// 查询用户概览统计
///
/// 获取当前用户的概览统计数据，包括余额、总消费、token使用等
#[utoipa::path(
    get,
    path = "/user/stats/overview",
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<UserStatsVO>),
        (status = 401, description = "未授权", body = ApiResponse<UserStatsVO>),
        (status = 500, description = "服务器错误", body = ApiResponse<UserStatsVO>)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "用户统计"
)]
#[debug_handler]
pub async fn get_user_overview(jwt_auth: JwtAuth) -> Result<Json<ApiResponse<UserStatsVO>>> {
    let user_stats_service = UserStatsService::new();
    let user_id = jwt_auth.id.clone();

    let stats = user_stats_service.get_user_overview(&user_id).await?;

    Ok(Json(ApiResponse::success(stats)))
}

/// 查询用户趋势统计
///
/// 根据时间维度获取当前用户的趋势统计数据，包括token使用、消费等趋势
#[utoipa::path(
    get,
    path = "/user/stats/trends",
    params(
        ("dimension" = String, Query, description = "时间维度：day/week/month", example = "day")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<UserTrendStatsVO>),
        (status = 400, description = "参数错误", body = ApiResponse<UserTrendStatsVO>),
        (status = 401, description = "未授权", body = ApiResponse<UserTrendStatsVO>),
        (status = 500, description = "服务器错误", body = ApiResponse<UserTrendStatsVO>)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "用户统计"
)]
#[debug_handler]
pub async fn get_user_trends(
    jwt_auth: JwtAuth,
    Query(params): Query<TrendStatsQueryDTO>,
) -> Result<Json<ApiResponse<UserTrendStatsVO>>> {
    let dimension = match params.dimension.as_deref() {
        Some("day") => TimeDimension::Day,
        Some("week") => TimeDimension::Week,
        Some("month") => TimeDimension::Month,
        _ => TimeDimension::Day,
    };

    let user_stats_service = UserStatsService::new();
    let user_id = jwt_auth.id.clone();

    let stats = user_stats_service
        .get_user_trend_stats(&user_id, dimension)
        .await?;

    Ok(Json(ApiResponse::success(stats)))
}

/// 刷新用户统计数据
///
/// 清空当前用户的缓存并重新计算统计数据
#[utoipa::path(
    post,
    path = "/user/stats/refresh",
    responses(
        (status = 200, description = "刷新成功", body = ApiResponse<String>),
        (status = 401, description = "未授权", body = ApiResponse<String>),
        (status = 500, description = "服务器错误", body = ApiResponse<String>)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "用户统计"
)]
#[debug_handler]
pub async fn refresh_user_stats(jwt_auth: JwtAuth) -> Result<Json<ApiResponse<String>>> {
    let user_stats_service = UserStatsService::new();
    let user_id = jwt_auth.id.clone();

    user_stats_service.clear_user_cache(&user_id).await;

    Ok(Json(ApiResponse::success("用户统计数据已刷新".to_string())))
}

/// 趋势统计查询参数
#[derive(Debug, Deserialize)]
pub struct TrendStatsQueryDTO {
    pub dimension: Option<String>,
}
