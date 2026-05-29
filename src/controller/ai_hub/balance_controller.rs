use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use crate::context::ServiceContext;
use crate::domain::dto::ai_hub::{DeductDTO, RechargeDTO, SetBalanceDTO};
use crate::domain::vo::ai_hub::BalanceVO;
use crate::domain::vo::response::ApiResponse;
use crate::error::ApplicationResult;
use crate::middleware::auth_axum::JwtAuth;
#[derive(Debug, Deserialize, ToSchema)]
pub struct GetBalanceRequest {
    pub user_id: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RechargeResponse {
    pub transaction_id: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DeductResponse {
    pub transaction_id: String,
}

/// 用途：获取用户余额
/// 说明：查询指定用户的当前余额信息
#[utoipa::path(
    post,
    path = "/ai_hub/balance/get",
    request_body = GetBalanceRequest,
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<BalanceVO>),
        (status = 400, description = "查询失败", body = ApiResponse<BalanceVO>)
    ),
    tag = "balance"
)]
pub async fn get_balance(
    State(ctx): State<Arc<ServiceContext>>,
    Json(req): Json<GetBalanceRequest>,
) -> ApplicationResult<Json<ApiResponse<BalanceVO>>> {
    let balance = ctx.balance_service.get_balance(&req.user_id).await?;

    Ok(Json(ApiResponse::success(balance)))
}

/// 用途：余额充值
/// 说明：为指定用户账户充值指定金额
#[utoipa::path(
    post,
    path = "/ai_hub/balance/recharge",
    request_body = RechargeDTO,
    responses(
        (status = 200, description = "充值成功", body = ApiResponse<RechargeResponse>),
        (status = 400, description = "充值失败", body = ApiResponse<RechargeResponse>)
    ),
    tag = "balance"
)]
pub async fn recharge(
    State(ctx): State<Arc<ServiceContext>>,
    jwt_auth: JwtAuth,
    Json(req): Json<RechargeDTO>,
) -> ApplicationResult<Json<ApiResponse<RechargeResponse>>> {
    let operator_id = Some(jwt_auth.id.clone());
    let transaction_id = ctx.balance_service.recharge(req, operator_id).await?;

    Ok(Json(ApiResponse::success(RechargeResponse {
        transaction_id,
    })))
}

/// 用途：余额扣减
/// 说明：从指定用户账户扣减指定金额
#[utoipa::path(
    post,
    path = "/ai_hub/balance/deduct",
    request_body = DeductDTO,
    responses(
        (status = 200, description = "扣减成功", body = ApiResponse<DeductResponse>),
        (status = 400, description = "扣减失败", body = ApiResponse<DeductResponse>)
    ),
    tag = "balance"
)]
pub async fn deduct(
    State(ctx): State<Arc<ServiceContext>>,
    Json(req): Json<DeductDTO>,
) -> ApplicationResult<Json<ApiResponse<DeductResponse>>> {
    let transaction_id = ctx.balance_service.deduct(req).await?;

    Ok(Json(ApiResponse::success(DeductResponse {
        transaction_id,
    })))
}

/// 用途：设置余额
/// 说明：直接设置指定用户账户的余额
#[utoipa::path(
    post,
    path = "/ai_hub/balance/set",
    request_body = SetBalanceDTO,
    responses(
        (status = 200, description = "设置成功", body = ApiResponse<RechargeResponse>),
        (status = 400, description = "设置失败", body = ApiResponse<RechargeResponse>)
    ),
    tag = "balance"
)]
pub async fn set_balance(
    State(ctx): State<Arc<ServiceContext>>,
    jwt_auth: JwtAuth,
    Json(req): Json<SetBalanceDTO>,
) -> ApplicationResult<Json<ApiResponse<RechargeResponse>>> {
    let operator_id = Some(jwt_auth.id.clone());
    let transaction_id = ctx.balance_service.set_balance(req, operator_id).await?;

    Ok(Json(ApiResponse::success(RechargeResponse {
        transaction_id,
    })))
}
