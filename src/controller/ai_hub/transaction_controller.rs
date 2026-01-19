use axum::{Json, extract::State};
use serde::Serialize;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::context::ServiceContext;
use crate::domain::dto::ai_hub::QueryTransactionDTO;
use crate::domain::vo::ai_hub::TransactionVO;
use crate::domain::vo::response::ApiResponse;
use crate::error::ApplicationResult;
use crate::middleware::auth_axum::JwtAuth;
use crate::service::ai_hub::TransactionService;

#[derive(Debug, Serialize, ToSchema)]
pub struct TransactionListResponse {
    pub transactions: Vec<TransactionVO>,
    pub total: usize,
}

/// 用途：查询交易记录列表
/// 说明：根据用户ID、交易类型等条件查询交易记录
#[utoipa::path(
    post,
    path = "/ai_hub/transaction/list",
    request_body = QueryTransactionDTO,
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<TransactionListResponse>),
        (status = 400, description = "查询失败", body = ApiResponse<TransactionListResponse>),
        (status = 401, description = "未授权", body = ApiResponse<TransactionListResponse>)
    ),
    tag = "transaction",
    security(
        ("jwt_auth" = [])
    )
)]
pub async fn list(
    State(_ctx): State<Arc<ServiceContext>>,
    jwt_auth: JwtAuth,
    Json(req): Json<QueryTransactionDTO>,
) -> ApplicationResult<Json<ApiResponse<TransactionListResponse>>> {
    let transaction_service = TransactionService {};
    let current_user_id = jwt_auth.id.clone();
    let has_manage_balance = jwt_auth.permissions.contains(&"manage_balance".to_string());

    let (transactions, total) = transaction_service
        .list_transaction(req, current_user_id, has_manage_balance)
        .await?;

    Ok(Json(ApiResponse::success(TransactionListResponse {
        transactions,
        total,
    })))
}
