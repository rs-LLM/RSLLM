use crate::context::ServiceContext;
use crate::controller::{
    img_controller, init_controller, rbac_user_controller,
    sys::{sys_auth_controller, sys_oauth_controller, twofa_controller},
};
use crate::domain::vo::response::ApiResponse;
use axum::Router;
use axum::routing::{get, post};
use std::sync::Arc;

pub fn create_no_auth_router() -> Router<Arc<ServiceContext>> {
    Router::new()
        .route(
            "/admin/",
            get(|| async { ApiResponse::success("hello".to_string()) }),
        )
        .route("/admin/sys_login", post(rbac_user_controller::login))
        .route("/admin/login", post(rbac_user_controller::login))
        .route("/auth/login", post(sys_auth_controller::login))
        .route(
            "/oauth/state",
            get(sys_oauth_controller::generate_oauth_state),
        )
        .route(
            "/oauth/{provider}",
            get(sys_oauth_controller::handle_oauth_callback),
        )
        .route(
            "/user/login/2fa",
            post(twofa_controller::verify_login_twofa),
        )
        .route("/auth/register", post(init_controller::register))
        .route(
            "/auth/register/send-email-code",
            post(init_controller::send_register_email_code),
        )
        .route("/admin/auth/check", post(sys_auth_controller::check))
        .route(
            "/admin/auth/refresh",
            post(sys_auth_controller::refresh_token),
        )
        .route("/admin/auth/logout", post(sys_auth_controller::logout))
        .route("/admin/captcha", get(img_controller::captcha))
}
