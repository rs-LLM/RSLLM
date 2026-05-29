use crate::context::ServiceContext;
use crate::controller::sys::profile_controller;
use crate::controller::{
    ai_hub::{model_controller, provider_config_controller, subscription_controller},
    status_controller, sys_service_controller,
};
use axum::Router;
use axum::routing::{get, post};
use std::sync::Arc;

pub fn create_public_router() -> Router<Arc<ServiceContext>> {
    Router::new()
        .route("/get_models", get(model_controller::list_public_models))
        .route(
            "/status/overview",
            get(status_controller::get_status_overview),
        )
        .route(
            "/uptime/status",
            get(status_controller::get_status_overview),
        )
        .route("/status/probe", post(status_controller::probe_status))
        .route(
            "/system-config/branding",
            get(sys_service_controller::get_public_branding_config),
        )
        .route("/notice", get(sys_service_controller::get_public_notice))
        .route(
            "/user-agreement",
            get(sys_service_controller::get_public_user_agreement),
        )
        .route(
            "/privacy-policy",
            get(sys_service_controller::get_public_privacy_policy),
        )
        .route("/about", get(sys_service_controller::get_public_about))
        .route(
            "/home_page_content",
            get(sys_service_controller::get_public_home_page_content),
        )
        .route("/pricing", get(sys_service_controller::get_public_pricing))
        .route(
            "/ratio_config",
            get(sys_service_controller::get_public_ratio_config),
        )
        .route(
            "/subscription/plans",
            get(subscription_controller::list_public_subscription_plans),
        )
        .route(
            "/subscription/payment/providers",
            get(subscription_controller::list_public_subscription_payment_providers),
        )
        .route(
            "/subscription/payment/callback",
            post(subscription_controller::confirm_subscription_payment_callback),
        )
        .route("/file/read", get(profile_controller::read_public_file))
        .route(
            "/providers/oauth/callback",
            get(provider_config_controller::provider_oauth_public_callback),
        )
        .route(
            "/providers/{provider_type}/oauth/callback",
            get(provider_config_controller::provider_oauth_public_callback_by_type),
        )
}
