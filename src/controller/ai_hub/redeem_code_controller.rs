use std::sync::Arc;

use axum::{Json, extract::State};
use serde::Serialize;

use crate::context::ServiceContext;
use crate::domain::dto::ai_hub::redeem_code::{
    GenerateRedeemCodesDTO, QueryRedeemCodesDTO, RedeemCodeExchangeDTO, UpdateRedeemCodeStatusDTO,
};
use crate::domain::table::ai_hub::redeem_code::RedeemCode;
use crate::domain::vo::response::ApiResponse;
use crate::error::ApplicationResult;
use crate::middleware::auth_axum::JwtAuth;
use crate::service::ai_hub::{RedeemCodeListItem, RedeemCodeService};

#[derive(Serialize)]
pub struct ListRedeemCodesResponse {
    pub total: usize,
    pub items: Vec<RedeemCodeListItem>,
}

pub async fn generate(
    State(context): State<Arc<ServiceContext>>,
    jwt_auth: JwtAuth,
    Json(dto): Json<GenerateRedeemCodesDTO>,
) -> ApplicationResult<Json<ApiResponse<Vec<RedeemCode>>>> {
    let items =
        RedeemCodeService::generate_codes(&context.rb, dto, Some(jwt_auth.id.clone())).await?;
    Ok(Json(ApiResponse::success(items)))
}

pub async fn list(
    State(context): State<Arc<ServiceContext>>,
    Json(dto): Json<QueryRedeemCodesDTO>,
) -> ApplicationResult<Json<ApiResponse<ListRedeemCodesResponse>>> {
    let data = RedeemCodeService::list_codes(&context.rb, dto).await?;
    Ok(Json(ApiResponse::success(ListRedeemCodesResponse {
        total: data.total,
        items: data.items,
    })))
}

pub async fn update_status(
    State(context): State<Arc<ServiceContext>>,
    jwt_auth: JwtAuth,
    Json(dto): Json<UpdateRedeemCodeStatusDTO>,
) -> ApplicationResult<Json<ApiResponse<RedeemCode>>> {
    let updated =
        RedeemCodeService::update_status(&context.rb, dto, Some(jwt_auth.id.clone())).await?;
    Ok(Json(ApiResponse::success(updated)))
}

pub async fn redeem(
    State(context): State<Arc<ServiceContext>>,
    jwt_auth: JwtAuth,
    Json(dto): Json<RedeemCodeExchangeDTO>,
) -> ApplicationResult<Json<ApiResponse<serde_json::Value>>> {
    let result = RedeemCodeService::redeem(&context.rb, dto, jwt_auth.id.clone()).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "amount": result.amount,
        "balanceAfter": result.balance_after
    }))))
}
