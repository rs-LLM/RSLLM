use axum::Router;
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::controller::ai_hub::user_level_model_rate_limit_controller;

/// 用户等级模型速率限制路由器
///
/// 提供用户等级模型速率限制配置的API端点
pub fn create_user_level_model_rate_limit_router() -> Router<Arc<ServiceContext>> {
    Router::new()
        .route(
            "/admin/user-level-model-limits",
            axum::routing::post(
                user_level_model_rate_limit_controller::create_user_level_model_limit,
            )
            .get(user_level_model_rate_limit_controller::list_user_level_model_limits),
        )
        .route(
            "/admin/user-level-model-limits/{id}",
            axum::routing::get(user_level_model_rate_limit_controller::get_user_level_model_limit)
                .put(user_level_model_rate_limit_controller::update_user_level_model_limit)
                .delete(user_level_model_rate_limit_controller::delete_user_level_model_limit),
        )
        .route(
            "/admin/user-levels/{level_code}/model-limits",
            axum::routing::get(user_level_model_rate_limit_controller::get_user_level_model_limits),
        )
        .route(
            "/admin/user-level-model-limits/batch",
            axum::routing::post(
                user_level_model_rate_limit_controller::batch_create_user_level_model_limits,
            ),
        )
}
