//! 统计分析控制器模块
//! 提供用量统计、性能分析等API接口

use axum::{
    extract::Query,
    http::HeaderMap,
    Json,
};

use crate::error::Result;
use crate::service::ai_hub::{UserUsageStatsQueryDTO, SystemStatsQueryDTO, PerformanceTrendsQueryDTO, UserUsageStatsVO, SystemStatsVO, PerformanceTrendsVO, ModelStatsVO, DailyStatsVO, UserStatsVO, HourlyTrendVO, AnalyticsService};
use crate::domain::vo::response::ApiResponse;

use axum::debug_handler;

/// 查询用户用量统计
///
/// 获取用户的详细用量统计数据
#[utoipa::path(
    get,
    path = "/api/v1/analytics/user-usage",
    params(
        ("user_id" = String, Query, description = "用户ID"),
        ("start_time" = String, Query, description = "开始时间"),
        ("end_time" = String, Query, description = "结束时间")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<UserUsageStatsVO>),
        (status = 400, description = "参数错误", body = ApiResponse<UserUsageStatsVO>),
        (status = 401, description = "未授权", body = ApiResponse<UserUsageStatsVO>),
        (status = 500, description = "服务器错误", body = ApiResponse<UserUsageStatsVO>)
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn get_user_usage_stats(
    headers: HeaderMap,
    Query(params): Query<UserUsageStatsQueryDTO>,
) -> Result<Json<ApiResponse<UserUsageStatsVO>>> {
    // 从请求头获取用户信息
    let user_id = headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("default_user");
    
    // 如果不是管理员，只能查询自己的统计
    let role = headers
        .get("x-user-role")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("user");
    
    let query_user_id = if role == "admin" {
        params.user_id.clone().unwrap_or_else(|| user_id.to_string())
    } else {
        user_id.to_string()
    };
    
    // 创建AnalyticsService实例
    let analytics_service = AnalyticsService {};
    
    let stats = analytics_service
        .get_user_usage_stats(&query_user_id, params.start_time, params.end_time, params.model_id)
        .await?;
    
    Ok(Json(ApiResponse::success(UserUsageStatsVO {
        user_id: stats.user_id,
        total_requests: stats.total_requests,
        total_input_tokens: stats.total_input_tokens,
        total_output_tokens: stats.total_output_tokens,
        total_tokens: stats.total_tokens,
        total_cost: stats.total_cost,
        avg_response_time: stats.avg_response_time,
        model_stats: stats.model_stats.into_iter().map(|m| ModelStatsVO {
            model_id: m.model_id,
            requests: m.requests,
            input_tokens: m.input_tokens,
            output_tokens: m.output_tokens,
            cost: m.cost,
        }).collect(),
        daily_stats: stats.daily_stats.into_iter().map(|d| DailyStatsVO {
            date: d.date,
            requests: d.requests,
            input_tokens: d.input_tokens,
            output_tokens: d.output_tokens,
            cost: d.cost,
        }).collect(),
    })))
}

/// 查询系统统计
///
/// 获取系统级别的统计数据
#[utoipa::path(
    get,
    path = "/api/v1/analytics/system",
    params(
        ("start_time" = String, Query, description = "开始时间"),
        ("end_time" = String, Query, description = "结束时间")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<SystemStatsVO>),
        (status = 400, description = "参数错误", body = ApiResponse<SystemStatsVO>),
        (status = 401, description = "未授权", body = ApiResponse<SystemStatsVO>),
        (status = 500, description = "服务器错误", body = ApiResponse<SystemStatsVO>)
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn get_system_stats(
    Query(params): Query<SystemStatsQueryDTO>,
) -> Result<Json<ApiResponse<SystemStatsVO>>> {
    // 创建AnalyticsService实例
    let analytics_service = AnalyticsService {};
    
    let stats = analytics_service
        .get_system_stats(params.start_time, params.end_time)
        .await?;
    
    Ok(Json(ApiResponse::success(SystemStatsVO {
        total_requests: stats.total_requests,
        total_tokens: stats.total_tokens,
        total_revenue: stats.total_revenue,
        active_users: stats.active_users,
        top_users: stats.top_users.into_iter().map(|u| UserStatsVO {
            user_id: u.user_id,
            requests: u.requests,
            tokens: u.tokens,
            cost: u.cost,
        }).collect(),
        top_models: stats.top_models.into_iter().map(|m| ModelStatsVO {
            model_id: m.model_id,
            requests: m.requests,
            input_tokens: m.input_tokens,
            output_tokens: m.output_tokens,
            cost: m.cost,
        }).collect(),
    })))
}

/// 查询性能趋势
///
/// 获取系统性能趋势数据
#[utoipa::path(
    get,
    path = "/api/v1/analytics/performance-trends",
    params(
        ("days" = i64, Query, description = "天数")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<PerformanceTrendsVO>),
        (status = 400, description = "参数错误", body = ApiResponse<PerformanceTrendsVO>),
        (status = 401, description = "未授权", body = ApiResponse<PerformanceTrendsVO>),
        (status = 500, description = "服务器错误", body = ApiResponse<PerformanceTrendsVO>)
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn get_performance_trends(
    Query(params): Query<PerformanceTrendsQueryDTO>,
) -> Result<Json<ApiResponse<PerformanceTrendsVO>>> {
    // 创建AnalyticsService实例
    let analytics_service = AnalyticsService {};
    
    let days = params.days.unwrap_or(7);
    let trends = analytics_service
        .get_performance_trends(days)
        .await?;
    
    Ok(Json(ApiResponse::success(PerformanceTrendsVO {
        success_rate: trends.success_rate,
        avg_response_time: trends.avg_response_time,
        hourly_trends: trends.hourly_trends.into_iter().map(|h| HourlyTrendVO {
            hour: h.hour,
            requests: h.requests,
            avg_response_time: h.avg_response_time,
            error_rate: h.error_rate,
        }).collect(),
    })))
}