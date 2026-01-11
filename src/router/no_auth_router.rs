use crate::context::ServiceContext;
use crate::controller::{
    ai_hub::model_controller, img_controller, init_controller, rbac_user_controller,
    sys_auth_controller,
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
        .route("/auth/register", post(init_controller::register))
        .route("/admin/auth/check", post(sys_auth_controller::check))
        .route(
            "/admin/auth/refresh",
            post(sys_auth_controller::refresh_token),
        )
        .route("/admin/auth/logout", post(sys_auth_controller::logout))
        .route("/admin/captcha", get(img_controller::captcha))
        .route("/get_models", get(model_controller::list_public_models))
}
