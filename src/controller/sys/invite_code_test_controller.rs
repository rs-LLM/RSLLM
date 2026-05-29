use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use serde::Deserialize;

use crate::context::ServiceContext;
use crate::domain::vo::response::ApiResponse;
use crate::error::ApplicationResult;
use crate::middleware::auth_axum::JwtAuth;
use crate::service::sys::RegisterPolicyService;

#[derive(Clone, Debug, Deserialize)]
pub struct AdminConsumeInviteCodeRequest {
    pub code: String,
    pub times: Option<u32>,
}

pub async fn consume_invite_code_for_test(
    State(context): State<Arc<ServiceContext>>,
    jwt_auth: JwtAuth,
    Json(req): Json<AdminConsumeInviteCodeRequest>,
) -> ApplicationResult<Json<ApiResponse<serde_json::Value>>> {
    let times = req.times.unwrap_or(1).clamp(1, 200);
    let mut success = 0u32;
    let mut failed = 0u32;

    for _ in 0..times {
        match RegisterPolicyService::consume_invite_code(
            context.rb.as_ref(),
            req.code.as_str(),
            Some(jwt_auth.id.clone()),
        )
        .await
        {
            Ok(_) => success += 1,
            Err(_) => failed += 1,
        }
    }

    Ok(Json(ApiResponse::success(serde_json::json!({
        "success": success,
        "failed": failed
    }))))
}
