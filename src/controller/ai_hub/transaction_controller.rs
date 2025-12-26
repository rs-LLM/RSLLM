use axum::{extract::State, Json};
use serde::Serialize;
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::domain::dto::ai_hub::QueryTransactionDTO;
use crate::domain::vo::ai_hub::TransactionVO;
use crate::domain::vo::response::ApiResponse;
use crate::error::ApplicationResult;
use crate::middleware::auth_axum::JwtAuth;
use crate::service::ai_hub::TransactionService;

#[derive(Debug, Serialize)]
pub struct TransactionListResponse {
    pub transactions: Vec<TransactionVO>,
    pub total: usize,
}

pub async fn list(
    State(_ctx): State<Arc<ServiceContext>>,
    jwt_auth: JwtAuth,
    Json(req): Json<QueryTransactionDTO>,
) -> ApplicationResult<Json<ApiResponse<TransactionListResponse>>> {
    let transaction_service = TransactionService {};
    let current_user_id = jwt_auth.id.clone();
    let has_manage_balance = jwt_auth.permissions.contains(&"manage_balance".to_string());
    
    let transactions = transaction_service.list_transaction(req, current_user_id, has_manage_balance).await?;
    let total = transactions.len();
    
    Ok(Json(ApiResponse::success(TransactionListResponse {
        transactions,
        total,
    })))
}
