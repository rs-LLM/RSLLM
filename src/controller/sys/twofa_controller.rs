use crate::context::CONTEXT;
use crate::domain::dto::basic::{
    IdDTO, TwoFaBackupCodesRequest, TwoFaDisableRequest, TwoFaEnableRequest,
    TwoFaVerifyLoginRequest,
};
use crate::domain::vo::SignInVO;
use crate::domain::vo::response::ApiResponse;
use crate::middleware::auth_axum::JwtAuth;
use crate::service::sys::{SignInResult, TwoFaService};
use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;

pub use crate::domain::dto::basic::NeedTwoFaResponse;

#[utoipa::path(
    get,
    path = "/user/2fa/status",
    responses(
        (status = 200, description = "获取2FA状态成功", body = ApiResponse<crate::domain::dto::basic::TwoFaStatusResponse>),
        (status = 400, description = "获取2FA状态失败", body = ApiResponse<serde_json::Value>)
    ),
    security(("bearer_auth" = [])),
    tag = "auth"
)]
pub async fn get_twofa_status(jwt: JwtAuth) -> impl IntoResponse {
    let service = TwoFaService;
    match service.get_status(&jwt.id).await {
        Ok(v) => (StatusCode::OK, Json(ApiResponse::success(v))).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::error(
                "-1",
                &e.to_string(),
            )),
        )
            .into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/user/2fa/setup",
    responses(
        (status = 200, description = "发起2FA设置成功", body = ApiResponse<crate::domain::dto::basic::TwoFaSetupResponse>),
        (status = 400, description = "发起2FA设置失败", body = ApiResponse<serde_json::Value>)
    ),
    security(("bearer_auth" = [])),
    tag = "auth"
)]
pub async fn setup_twofa(jwt: JwtAuth) -> impl IntoResponse {
    let service = TwoFaService;
    let ctx = std::sync::Arc::new(CONTEXT.clone());
    match service.setup(&ctx, &jwt.id).await {
        Ok(v) => (StatusCode::OK, Json(ApiResponse::success(v))).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::error(
                "-1",
                &e.to_string(),
            )),
        )
            .into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/user/2fa/enable",
    request_body = TwoFaEnableRequest,
    responses(
        (status = 200, description = "启用2FA成功", body = ApiResponse<serde_json::Value>),
        (status = 400, description = "启用2FA失败", body = ApiResponse<serde_json::Value>)
    ),
    security(("bearer_auth" = [])),
    tag = "auth"
)]
pub async fn enable_twofa(jwt: JwtAuth, Json(body): Json<TwoFaEnableRequest>) -> impl IntoResponse {
    let service = TwoFaService;
    let ctx = std::sync::Arc::new(CONTEXT.clone());
    match service.enable(&ctx, &jwt.id, &body).await {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({"enabled": true}))),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::error(
                "-1",
                &e.to_string(),
            )),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/user/2fa/disable",
    request_body = TwoFaDisableRequest,
    responses(
        (status = 200, description = "禁用2FA成功", body = ApiResponse<serde_json::Value>),
        (status = 400, description = "禁用2FA失败", body = ApiResponse<serde_json::Value>)
    ),
    security(("bearer_auth" = [])),
    tag = "auth"
)]
pub async fn disable_twofa(
    jwt: JwtAuth,
    Json(body): Json<TwoFaDisableRequest>,
) -> impl IntoResponse {
    let service = TwoFaService;
    match service.disable(&jwt.id, &body).await {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({"enabled": false}))),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::error(
                "-1",
                &e.to_string(),
            )),
        ),
    }
}

#[utoipa::path(
    post,
    path = "/user/2fa/backup_codes",
    request_body = TwoFaBackupCodesRequest,
    responses(
        (status = 200, description = "重置备用码成功", body = ApiResponse<crate::domain::dto::basic::TwoFaBackupCodesResponse>),
        (status = 400, description = "重置备用码失败", body = ApiResponse<serde_json::Value>)
    ),
    security(("bearer_auth" = [])),
    tag = "auth"
)]
pub async fn regenerate_backup_codes(
    jwt: JwtAuth,
    Json(body): Json<TwoFaBackupCodesRequest>,
) -> impl IntoResponse {
    let service = TwoFaService;
    match service.regenerate_backup_codes(&jwt.id, &body).await {
        Ok(v) => (StatusCode::OK, Json(ApiResponse::success(v))).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::error(
                "-1",
                &e.to_string(),
            )),
        )
            .into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/user/login/2fa",
    request_body = TwoFaVerifyLoginRequest,
    responses(
        (status = 200, description = "2FA登录成功", body = ApiResponse<SignInVO>),
        (status = 401, description = "2FA登录失败", body = ApiResponse<serde_json::Value>)
    ),
    tag = "auth"
)]
pub async fn verify_login_twofa(Json(body): Json<TwoFaVerifyLoginRequest>) -> impl IntoResponse {
    let service = TwoFaService;
    match service.verify_login(&body).await {
        Ok(v) => (StatusCode::OK, Json(ApiResponse::success(v))).into_response(),
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<serde_json::Value>::error(
                "-1",
                &e.to_string(),
            )),
        )
            .into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/admin/user/2fa/disable",
    request_body = IdDTO,
    responses(
        (status = 200, description = "管理员重置2FA成功", body = ApiResponse<serde_json::Value>),
        (status = 400, description = "管理员重置2FA失败", body = ApiResponse<serde_json::Value>)
    ),
    security(("bearer_auth" = [])),
    tag = "auth"
)]
pub async fn admin_reset_twofa(Json(arg): Json<IdDTO>) -> impl IntoResponse {
    let user_id = arg.id.unwrap_or_default();
    let service = TwoFaService;
    match service.admin_reset_by_user_id(&user_id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({"reset": true}))),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::error(
                "-1",
                &e.to_string(),
            )),
        ),
    }
}

#[utoipa::path(
    get,
    path = "/admin/user/2fa/stats",
    responses(
        (status = 200, description = "获取2FA统计成功", body = ApiResponse<crate::domain::dto::basic::TwoFaAdminStatsResponse>),
        (status = 400, description = "获取2FA统计失败", body = ApiResponse<serde_json::Value>)
    ),
    security(("bearer_auth" = [])),
    tag = "auth"
)]
pub async fn admin_twofa_stats() -> impl IntoResponse {
    let service = TwoFaService;
    match service.get_admin_stats().await {
        Ok(v) => (StatusCode::OK, Json(ApiResponse::success(v))).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::error(
                "-1",
                &e.to_string(),
            )),
        )
            .into_response(),
    }
}

pub fn sign_in_result_to_response(result: SignInResult) -> axum::response::Response {
    match result {
        SignInResult::Success(vo) => {
            (StatusCode::OK, Json(ApiResponse::success(vo))).into_response()
        }
        SignInResult::NeedTwoFa {
            challenge_id,
            expires_in,
        } => (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error_with_data(
                "TWOFA_REQUIRED",
                "需要二次验证",
                NeedTwoFaResponse {
                    challenge_id,
                    expires_in,
                },
            )),
        )
            .into_response(),
    }
}
