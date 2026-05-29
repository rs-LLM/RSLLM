//! 管理员统计控制器模块
//! 提供管理员面板统计API接口

use axum::{Json, extract::Query};
use serde::Deserialize;

use crate::domain::vo::ai_hub::admin_stats::*;
use crate::domain::vo::response::ApiResponse;
use crate::error::Result;
use crate::service::ai_hub::AdminStatsService;

use axum::debug_handler;

/// 查询概览统计
///
/// 获取管理员面板的概览统计数据，包括用户数、收入、token使用等
#[utoipa::path(
    get,
    path = "/admin/stats/overview",
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<AdminOverviewStatsVO>),
        (status = 401, description = "未授权", body = ApiResponse<AdminOverviewStatsVO>),
        (status = 403, description = "权限不足", body = ApiResponse<AdminOverviewStatsVO>),
        (status = 500, description = "服务器错误", body = ApiResponse<AdminOverviewStatsVO>)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "管理员统计"
)]
#[debug_handler]
pub async fn get_overview_stats() -> Result<Json<ApiResponse<AdminOverviewStatsVO>>> {
    let admin_stats_service = AdminStatsService::new();

    let stats = admin_stats_service.get_overview_stats().await?;

    Ok(Json(ApiResponse::success(stats)))
}

/// 查询趋势统计
///
/// 根据时间维度获取趋势统计数据，包括收入、token使用、请求数等趋势
#[utoipa::path(
    get,
    path = "/admin/stats/trends",
    params(
        ("dimension" = String, Query, description = "时间维度：day/week/month", example = "day"),
        ("start_date" = Option<String>, Query, description = "开始日期，格式 YYYY-MM-DD", example = "2026-02-01"),
        ("end_date" = Option<String>, Query, description = "结束日期，格式 YYYY-MM-DD", example = "2026-02-27")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<AdminTrendStatsVO>),
        (status = 400, description = "参数错误", body = ApiResponse<AdminTrendStatsVO>),
        (status = 401, description = "未授权", body = ApiResponse<AdminTrendStatsVO>),
        (status = 403, description = "权限不足", body = ApiResponse<AdminTrendStatsVO>),
        (status = 500, description = "服务器错误", body = ApiResponse<AdminTrendStatsVO>)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "管理员统计"
)]
#[debug_handler]
pub async fn get_trend_stats(
    Query(params): Query<TrendStatsQueryDTO>,
) -> Result<Json<ApiResponse<AdminTrendStatsVO>>> {
    let dimension = match params.dimension.as_deref() {
        Some("day") => TimeDimension::Day,
        Some("week") => TimeDimension::Week,
        Some("month") => TimeDimension::Month,
        _ => TimeDimension::Day, // 默认为日维度
    };

    // 创建AdminStatsService实例
    let admin_stats_service = AdminStatsService::new();

    let stats = admin_stats_service
        .get_trend_stats(
            dimension,
            params.start_date.clone(),
            params.end_date.clone(),
        )
        .await?;

    Ok(Json(ApiResponse::success(stats)))
}

/// 查询用户统计
///
/// 根据时间维度获取用户统计数据，包括新增用户、活跃用户等
#[utoipa::path(
    get,
    path = "/admin/stats/users",
    params(
        ("dimension" = String, Query, description = "时间维度：day/week/month", example = "day"),
        ("start_date" = Option<String>, Query, description = "开始日期，格式 YYYY-MM-DD", example = "2026-02-01"),
        ("end_date" = Option<String>, Query, description = "结束日期，格式 YYYY-MM-DD", example = "2026-02-27")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<AdminUserStatsVO>),
        (status = 400, description = "参数错误", body = ApiResponse<AdminUserStatsVO>),
        (status = 401, description = "未授权", body = ApiResponse<AdminUserStatsVO>),
        (status = 403, description = "权限不足", body = ApiResponse<AdminUserStatsVO>),
        (status = 500, description = "服务器错误", body = ApiResponse<AdminUserStatsVO>)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "管理员统计"
)]
#[debug_handler]
pub async fn get_user_stats(
    Query(params): Query<UserStatsQueryDTO>,
) -> Result<Json<ApiResponse<AdminUserStatsVO>>> {
    let dimension = match params.dimension.as_deref() {
        Some("day") => TimeDimension::Day,
        Some("week") => TimeDimension::Week,
        Some("month") => TimeDimension::Month,
        _ => TimeDimension::Day, // 默认为日维度
    };

    // 创建AdminStatsService实例
    let admin_stats_service = AdminStatsService::new();

    let stats = admin_stats_service
        .get_user_stats(
            dimension,
            params.start_date.clone(),
            params.end_date.clone(),
        )
        .await?;

    Ok(Json(ApiResponse::success(stats)))
}

/// 刷新统计数据
///
/// 清空缓存并重新计算统计数据
#[utoipa::path(
    post,
    path = "/admin/stats/refresh",
    responses(
        (status = 200, description = "刷新成功", body = ApiResponse<String>),
        (status = 401, description = "未授权", body = ApiResponse<String>),
        (status = 403, description = "权限不足", body = ApiResponse<String>),
        (status = 500, description = "服务器错误", body = ApiResponse<String>)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "管理员统计"
)]
#[debug_handler]
pub async fn refresh_stats() -> Result<Json<ApiResponse<String>>> {
    let admin_stats_service = AdminStatsService::new();

    // 清空缓存
    admin_stats_service.clear_cache().await;

    Ok(Json(ApiResponse::success("统计数据已刷新".to_string())))
}

/// 趋势统计查询参数
#[derive(Debug, Deserialize)]
pub struct TrendStatsQueryDTO {
    pub dimension: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

/// 用户统计查询参数
#[derive(Debug, Deserialize)]
pub struct UserStatsQueryDTO {
    pub dimension: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}
