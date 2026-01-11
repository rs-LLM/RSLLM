//! 管理路由模块
//! 提供模型和供应商管理的RESTful API路由

use axum::{
    Router,
    routing::{delete, get, post, put},
};
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::controller::ai_hub::{
    admin_stats_controller, model_controller, model_provider_mapping_controller,
    provider_config_controller, quota_controller, user_level_controller,
};

use crate::middleware::auth_axum::auth;
use crate::middleware::permission_middleware::require_permission;

/// 创建管理路由
///
/// 提供以下API端点：
/// - 模型管理：创建、查询、更新、删除、批量导入
/// - 供应商管理：创建、查询、更新、删除
/// - 配额管理：查询、创建、更新、充值、分配
pub fn create_admin_router() -> Router<Arc<ServiceContext>> {
    let provider_management_routes = Router::new()
        .route(
            "/admin/providers",
            post(provider_config_controller::create_provider),
        )
        .route(
            "/admin/providers/{id}",
            put(provider_config_controller::update_provider),
        )
        .route(
            "/admin/providers/{id}",
            delete(provider_config_controller::delete_provider),
        )
        .layer(axum::middleware::from_fn(require_permission(
            "sys:provider:edit",
        )));

    let model_management_routes = Router::new()
        .route("/admin/models", post(model_controller::create_model))
        .route("/admin/models/{id}", put(model_controller::update_model))
        .route("/admin/models/{id}", delete(model_controller::delete_model))
        .route(
            "/admin/model-provider-mappings",
            post(model_provider_mapping_controller::create_model_provider_mapping),
        )
        .route(
            "/admin/model-provider-mappings/{id}",
            put(model_provider_mapping_controller::update_model_provider_mapping),
        )
        .route(
            "/admin/model-provider-mappings/{id}",
            delete(model_provider_mapping_controller::delete_model_provider_mapping),
        )
        .layer(axum::middleware::from_fn(require_permission(
            "sys:model:edit",
        )));

    let view_permission_routes = Router::new()
        .route(
            "/admin/providers",
            get(provider_config_controller::list_providers),
        )
        .route(
            "/admin/providers/{id}",
            get(provider_config_controller::get_provider),
        )
        .route(
            "/admin/providers/name/{name}",
            get(provider_config_controller::get_provider_by_name),
        )
        .route("/admin/models", get(model_controller::list_models))
        .route("/admin/models/{id}", get(model_controller::get_model))
        .route(
            "/admin/model-provider-mappings",
            get(model_provider_mapping_controller::list_model_provider_mappings),
        )
        .route(
            "/admin/model-provider-mappings/{id}",
            get(model_provider_mapping_controller::get_model_provider_mapping),
        )
        .route(
            "/admin/models/{model_id}/provider-mappings",
            get(model_provider_mapping_controller::get_mappings_by_model),
        )
        .route(
            "/admin/users/levels",
            get(user_level_controller::get_all_user_levels),
        )
        .route(
            "/admin/users/{id}/level",
            get(user_level_controller::get_user_level),
        )
        .layer(axum::middleware::from_fn(require_permission(
            "ai:model:view",
        )));

    let admin_stats_routes = Router::new()
        .route(
            "/admin/stats/overview",
            get(admin_stats_controller::get_overview_stats),
        )
        .route(
            "/admin/stats/trends",
            get(admin_stats_controller::get_trend_stats),
        )
        .route(
            "/admin/stats/users",
            get(admin_stats_controller::get_user_stats),
        )
        .layer(axum::middleware::from_fn(require_permission(
            "admin:stats:view",
        )));

    let admin_stats_refresh_routes = Router::new()
        .route(
            "/admin/stats/refresh",
            post(admin_stats_controller::refresh_stats),
        )
        .layer(axum::middleware::from_fn(require_permission(
            "admin:stats:refresh",
        )));

    let quota_management_routes = Router::new()
        .route("/admin/quota", post(quota_controller::create_quota))
        .route("/admin/quota", get(quota_controller::list_admin_quotas))
        .route("/admin/quota/{id}", put(quota_controller::update_quota))
        .route("/admin/quota/{id}", get(quota_controller::get_admin_quota))
        .layer(axum::middleware::from_fn(require_permission(
            "manage_quotas",
        )));

    let user_level_management_routes = Router::new()
        .route(
            "/admin/users/{id}/level",
            put(user_level_controller::update_user_level),
        )
        .layer(axum::middleware::from_fn(require_permission(
            "sys:user:edit",
        )));

    Router::new()
        .merge(provider_management_routes)
        .merge(model_management_routes)
        .merge(view_permission_routes)
        .merge(quota_management_routes)
        .merge(user_level_management_routes)
        .merge(admin_stats_routes)
        .merge(admin_stats_refresh_routes)
        .layer(axum::middleware::from_fn(auth))
}
