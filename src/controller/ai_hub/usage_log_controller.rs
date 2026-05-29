//! 用量记录控制器模块
//! 提供用量记录查询、统计等API接口

use axum::{
    Json,
    extract::{Path, Query, State},
};
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::domain::vo::ai_hub::usage_log::{AiHubUsageLogVO, CostDetailVO, UsageStatisticsVO};
use crate::domain::vo::response::ApiResponse;
use crate::error::{Error, Result};
use crate::middleware::auth_axum::JwtAuth;
use crate::service::ai_hub::{UsageLogQueryDTO, UsageStatisticsQueryDTO};

use axum::debug_handler;

fn resolve_page(page: Option<i64>) -> Result<i64> {
    let page = page.unwrap_or(1);
    if page < 1 {
        return Err(Error::ValidationError(format!(
            "page must be >= 1, got {}",
            page
        )));
    }

    Ok(page)
}

fn resolve_page_size(page_size: Option<i64>) -> Result<i64> {
    let page_size = page_size.unwrap_or(20);
    if page_size < 1 {
        return Err(Error::ValidationError(format!(
            "page_size must be >= 1, got {}",
            page_size
        )));
    }

    Ok(page_size)
}

/// 查询用量记录列表
///
/// 分页查询用户的用量记录，支持按模型、状态、时间范围筛选
#[utoipa::path(
    get,
    path = "/usage-logs",
    params(
        ("model_id" = Option<String>, Query, description = "模型ID"),
        ("request_type" = Option<String>, Query, description = "请求类型"),
        ("status" = Option<String>, Query, description = "状态"),
        ("start_time" = Option<String>, Query, description = "开始时间"),
        ("end_time" = Option<String>, Query, description = "结束时间"),
        ("page" = Option<i64>, Query, description = "页码，默认1"),
        ("page_size" = Option<i64>, Query, description = "每页数量，默认20")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<serde_json::Value>),
        (status = 400, description = "参数错误", body = ApiResponse<serde_json::Value>),
        (status = 401, description = "未授权", body = ApiResponse<serde_json::Value>),
        (status = 500, description = "服务器错误", body = ApiResponse<serde_json::Value>)
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn list_usage_logs(
    jwt: JwtAuth,
    State(state): State<Arc<ServiceContext>>,
    Query(params): Query<UsageLogQueryDTO>,
) -> Result<Json<ApiResponse<serde_json::Value>>> {
    let user_id = jwt.id.to_string();
    let page = resolve_page(params.page)?;
    let page_size = resolve_page_size(params.page_size)?;

    let query_dto = UsageLogQueryDTO {
        user_id: Some(user_id.to_string()),
        model_id: params.model_id,
        provider_id: params.provider_id,
        request_type: params.request_type,
        status: params.status,
        start_time: params.start_time,
        end_time: params.end_time,
        page: Some(page),
        page_size: Some(page_size),
    };

    let logs = state.usage_log_service.list_usage_logs(query_dto).await?;

    let total = logs.len() as u64;

    let response_data = serde_json::json!({
        "data": logs,
        "total": total
    });

    Ok(Json(ApiResponse::success(response_data)))
}

/// 查询用量记录详情
///
/// 根据用量记录ID查询详细信息
#[utoipa::path(
    get,
    path = "/usage-logs/{id}",
    params(
        ("id" = String, Path, description = "用量记录ID")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<AiHubUsageLogVO>),
        (status = 400, description = "参数错误", body = ApiResponse<AiHubUsageLogVO>),
        (status = 401, description = "未授权", body = ApiResponse<AiHubUsageLogVO>),
        (status = 404, description = "记录不存在", body = ApiResponse<AiHubUsageLogVO>),
        (status = 500, description = "服务器错误", body = ApiResponse<AiHubUsageLogVO>)
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn get_usage_log_detail(
    jwt: JwtAuth,
    State(state): State<Arc<ServiceContext>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<AiHubUsageLogVO>>> {
    let user_id = jwt.id.to_string();

    let log = state
        .usage_log_service
        .get_usage_log_detail(&id, user_id.to_string())
        .await?;

    Ok(Json(ApiResponse::success(log)))
}

/// 查询用量统计汇总
///
/// 统计指定时间范围内的用量数据，包括总请求数、总token数、总费用等
#[utoipa::path(
    get,
    path = "/usage-logs/summary",
    params(
        ("period" = Option<String>, Query, description = "统计周期（day/week/month）"),
        ("start_time" = Option<String>, Query, description = "开始时间"),
        ("end_time" = Option<String>, Query, description = "结束时间"),
        ("model_ids" = Option<Vec<String>>, Query, description = "模型ID列表"),
        ("provider_ids" = Option<Vec<String>>, Query, description = "提供商ID列表")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<UsageStatisticsVO>),
        (status = 400, description = "参数错误", body = ApiResponse<UsageStatisticsVO>),
        (status = 401, description = "未授权", body = ApiResponse<UsageStatisticsVO>),
        (status = 500, description = "服务器错误", body = ApiResponse<UsageStatisticsVO>)
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn get_usage_summary(
    jwt: JwtAuth,
    State(state): State<Arc<ServiceContext>>,
    Query(params): Query<UsageStatisticsQueryDTO>,
) -> Result<Json<ApiResponse<UsageStatisticsVO>>> {
    let user_id = jwt.id.to_string();

    let query_dto = UsageStatisticsQueryDTO {
        user_id: Some(user_id.to_string()),
        period: params.period,
        start_time: params.start_time,
        end_time: params.end_time,
        model_ids: params.model_ids,
        provider_ids: params.provider_ids,
    };

    let summary = state.usage_log_service.get_usage_summary(query_dto).await?;

    Ok(Json(ApiResponse::success(summary)))
}

/// 查询费用明细
///
/// 按模型分组统计费用明细
#[utoipa::path(
    get,
    path = "/usage-logs/cost-details",
    params(
        ("start_time" = Option<String>, Query, description = "开始时间"),
        ("end_time" = Option<String>, Query, description = "结束时间"),
        ("model_ids" = Option<Vec<String>>, Query, description = "模型ID列表")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<Vec<CostDetailVO>>),
        (status = 400, description = "参数错误", body = ApiResponse<Vec<CostDetailVO>>),
        (status = 401, description = "未授权", body = ApiResponse<Vec<CostDetailVO>>),
        (status = 500, description = "服务器错误", body = ApiResponse<Vec<CostDetailVO>>)
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn get_cost_details(
    jwt: JwtAuth,
    State(state): State<Arc<ServiceContext>>,
    Query(params): Query<UsageStatisticsQueryDTO>,
) -> Result<Json<ApiResponse<Vec<CostDetailVO>>>> {
    let user_id = jwt.id.to_string();

    let query_dto = UsageStatisticsQueryDTO {
        user_id: Some(user_id.to_string()),
        period: None,
        start_time: params.start_time,
        end_time: params.end_time,
        model_ids: params.model_ids,
        provider_ids: None,
    };

    let cost_details = state.usage_log_service.get_cost_details(query_dto).await?;

    Ok(Json(ApiResponse::success(cost_details)))
}
