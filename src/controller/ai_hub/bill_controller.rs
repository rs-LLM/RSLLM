//! 账单管理控制器模块
//! 提供账单查询、支付、统计等API接口

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::error::Result;
use crate::service::ai_hub::{BillingQueryDTO, BillingStatisticsQueryDTO, PayBillingDTO, UpdateBillingDTO};
use crate::domain::vo::ai_hub::billing::{BillingStatisticsVO, AiHubBillingVO, BillingOverviewVO};
use crate::domain::vo::response::ApiResponse;

use axum::debug_handler;

/// 查询账单列表
///
/// 分页查询用户账单记录
#[utoipa::path(
    get,
    path = "/api/v1/billing",
    params(
        ("user_id" = String, Query, description = "用户ID"),
        ("page" = u32, Query, description = "页码"),
        ("size" = u32, Query, description = "每页数量")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<BillingOverviewVO>),
        (status = 400, description = "参数错误", body = ApiResponse<BillingOverviewVO>),
        (status = 401, description = "未授权", body = ApiResponse<BillingOverviewVO>),
        (status = 500, description = "服务器错误", body = ApiResponse<BillingOverviewVO>)
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn get_billing_list(
    headers: HeaderMap,
    State(state): State<Arc<ServiceContext>>,
    Query(params): Query<BillingQueryDTO>,
) -> Result<Json<ApiResponse<BillingOverviewVO>>> {
    // 从请求头获取用户信息
    let user_id = headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("default_user");
    
    // 如果不是管理员，只能查询自己的账单
    let role = headers
        .get("x-user-role")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("user");
    
    let query_user_id = if role == "admin" {
        params.user_id.clone().unwrap_or_else(|| user_id.to_string())
    } else {
        user_id.to_string()
    };
    
    let bills = state.bill_service
        .list_bills(BillingQueryDTO {
            user_id: Some(query_user_id.clone()),
            billing_cycle: None,
            payment_status: None,
            bill_status: None,
            start_time: None,
            end_time: None,
            page: params.page,
            page_size: params.page_size,
        })
        .await?;
    
    Ok(Json(ApiResponse::success(BillingOverviewVO {
        user_id: query_user_id.to_string(),
        total_bills: bills.len() as i32,
        pending_bills: bills.iter().filter(|b| b.payment_status == "pending").count() as i32,
        paid_bills: bills.iter().filter(|b| b.payment_status == "paid").count() as i32,
        total_amount: bills.iter().map(|b| b.total_amount).sum(),
        paid_amount: bills.iter().filter(|b| b.payment_status == "paid").map(|b| b.total_amount).sum(),
        pending_amount: bills.iter().filter(|b| b.payment_status == "pending").map(|b| b.total_amount).sum(),
        bills: bills.into_iter().map(|b| b).collect(),
    })))
}

/// 查询账单统计
///
/// 获取用户账单统计数据
#[utoipa::path(
    get,
    path = "/api/v1/billing/statistics",
    params(
        ("user_id" = String, Query, description = "用户ID"),
        ("start_time" = String, Query, description = "开始时间"),
        ("end_time" = String, Query, description = "结束时间")
    ),
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<BillingStatisticsVO>),
        (status = 400, description = "参数错误", body = ApiResponse<BillingStatisticsVO>),
        (status = 401, description = "未授权", body = ApiResponse<BillingStatisticsVO>),
        (status = 500, description = "服务器错误", body = ApiResponse<BillingStatisticsVO>)
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn get_billing_statistics(
    headers: HeaderMap,
    State(state): State<Arc<ServiceContext>>,
    Query(params): Query<BillingStatisticsQueryDTO>,
) -> Result<Json<ApiResponse<BillingStatisticsVO>>> {
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
    
    let statistics = state.bill_service
        .statistics(BillingStatisticsQueryDTO {
            user_id: Some(query_user_id.to_string()),
            period: None,
            start_time: params.start_time.clone(),
            end_time: params.end_time.clone(),
        })
        .await?;
    
    Ok(Json(ApiResponse::success(statistics)))
}

/// 支付账单
///
/// 支付指定账单
#[utoipa::path(
    post,
    path = "/api/v1/billing/pay",
    request_body = PayBillingDTO,
    responses(
        (status = 200, description = "支付成功", body = ApiResponse<AiHubBillingVO>),
        (status = 400, description = "参数错误或余额不足", body = ApiResponse<AiHubBillingVO>),
        (status = 404, description = "账单不存在", body = ApiResponse<AiHubBillingVO>),
        (status = 500, description = "服务器错误", body = ApiResponse<AiHubBillingVO>)
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn pay_billing(
    State(state): State<Arc<ServiceContext>>,
    Json(dto): Json<PayBillingDTO>,
) -> Result<Json<ApiResponse<AiHubBillingVO>>> {
    state.bill_service.pay_bill(&dto.billing_id, dto.clone()).await?;
    let billing = state.bill_service.get_bill(&dto.billing_id).await?;
    Ok(Json(ApiResponse::success(billing)))
}

/// 更新账单
///
/// 更新账单信息（管理员功能）
#[utoipa::path(
    put,
    path = "/api/v1/billing",
    request_body = UpdateBillingDTO,
    responses(
        (status = 200, description = "更新成功", body = ApiResponse<AiHubBillingVO>),
        (status = 400, description = "参数错误", body = ApiResponse<AiHubBillingVO>),
        (status = 404, description = "账单不存在", body = ApiResponse<AiHubBillingVO>),
        (status = 500, description = "服务器错误", body = ApiResponse<AiHubBillingVO>)
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn update_billing(
    State(state): State<Arc<ServiceContext>>,
    Json(dto): Json<UpdateBillingDTO>,
) -> Result<Json<ApiResponse<AiHubBillingVO>>> {
    state.bill_service
        .update_bill(&dto.billing_id, dto.clone())
        .await?;
    let billing = state.bill_service.get_bill(&dto.billing_id).await?;
    Ok(Json(ApiResponse::success(billing)))
}

/// 生成测试账单
///
/// 为测试目的生成账单记录
#[utoipa::path(
    post,
    path = "/api/v1/billing/generate",
    request_body = BillingQueryDTO,
    responses(
        (status = 200, description = "生成成功", body = ApiResponse<AiHubBillingVO>),
        (status = 400, description = "参数错误", body = ApiResponse<AiHubBillingVO>),
        (status = 500, description = "服务器错误", body = ApiResponse<AiHubBillingVO>)
    ),
    security(
        ("api_key" = [])
    )
)]
#[debug_handler]
pub async fn generate_test_billing(
    State(state): State<Arc<ServiceContext>>,
    Json(dto): Json<BillingQueryDTO>,
) -> Result<Json<ApiResponse<AiHubBillingVO>>> {
    let user_id = dto.user_id.clone().unwrap_or_else(|| "test_user".to_string());
    let billing_id = state.bill_service.generate_bill(&user_id, &"2024-12").await?;
    let billing = state.bill_service.get_bill(&billing_id).await?;
    Ok(Json(ApiResponse::success(billing)))
}