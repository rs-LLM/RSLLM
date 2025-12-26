use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::domain::dto::ai_hub::{RechargeDTO, DeductDTO, SetBalanceDTO};
use crate::domain::vo::ai_hub::BalanceVO;
use crate::domain::vo::response::ApiResponse;
use crate::error::ApplicationResult;
use crate::service::ai_hub::BalanceService;

#[derive(Debug, Deserialize)]
pub struct GetBalanceRequest {
    pub user_id: String,
}

#[derive(Debug, Serialize)]
pub struct RechargeResponse {
    pub transaction_id: String,
}

#[derive(Debug, Serialize)]
pub struct DeductResponse {
    pub transaction_id: String,
}

pub async fn get_balance(
    State(_ctx): State<Arc<ServiceContext>>,
    Json(req): Json<GetBalanceRequest>,
) -> ApplicationResult<Json<ApiResponse<BalanceVO>>> {
    let balance_service = BalanceService::new();
    let balance = balance_service.get_balance(&req.user_id).await?;
    
    Ok(Json(ApiResponse::success(balance)))
}

pub async fn recharge(
    State(_ctx): State<Arc<ServiceContext>>,
    Json(req): Json<RechargeDTO>,
) -> ApplicationResult<Json<ApiResponse<RechargeResponse>>> {
    let operator_id = Some("admin".to_string());
    let balance_service = BalanceService::new();
    let transaction_id = balance_service.recharge(req, operator_id).await?;
    
    Ok(Json(ApiResponse::success(RechargeResponse { transaction_id })))
}

pub async fn deduct(
    State(_ctx): State<Arc<ServiceContext>>,
    Json(req): Json<DeductDTO>,
) -> ApplicationResult<Json<ApiResponse<DeductResponse>>> {
    let balance_service = BalanceService::new();
    let transaction_id = balance_service.deduct(req).await?;
    
    Ok(Json(ApiResponse::success(DeductResponse { transaction_id })))
}

pub async fn set_balance(
    State(_ctx): State<Arc<ServiceContext>>,
    Json(req): Json<SetBalanceDTO>,
) -> ApplicationResult<Json<ApiResponse<RechargeResponse>>> {
    let operator_id = Some("admin".to_string());
    let balance_service = BalanceService::new();
    let transaction_id = balance_service.set_balance(req, operator_id).await?;
    
    Ok(Json(ApiResponse::success(RechargeResponse { transaction_id })))
}
