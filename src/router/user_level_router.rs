//! 用户等级管理路由器模块
//!
//! 提供用户等级管理的路由配置

use axum::{
    Router,
    routing::{delete, get, post, put},
};

use std::sync::Arc;

use crate::context::ServiceContext;
use crate::controller::ai_hub::rate_limit_controller;
use crate::controller::ai_hub::user_level_controller;
use crate::middleware::auth_axum::auth;
use crate::middleware::permission_middleware::require_permission;

/// 创建用户等级管理路由器
///
/// 提供以下API端点：
/// - GET /user-levels - 获取所有用户等级配置
/// - POST /user-levels - 创建新的用户等级配置
/// - GET /user-levels/level/{level_code} - 根据等级代码获取等级配置
/// - PUT /user-levels/{id} - 更新用户等级配置
/// - GET /user-levels/{id}/detail - 获取用户等级详情
/// - PUT /user-levels/batch/status - 批量更新用户等级状态
/// - DELETE /user-levels/{id} - 删除用户等级
/// - DELETE /user-levels/batch - 批量删除用户等级
/// - GET /users/{user_id}/level - 获取用户当前等级
/// - PUT /users/{user_id}/level - 更新用户等级
/// - GET /users/level - 获取当前登录用户的等级及速率限制信息
/// - POST /user-levels/init - 初始化默认用户等级配置
/// - GET /users/rate-limit - 获取当前用户速率限制状态
pub fn create_user_level_router() -> Router<Arc<ServiceContext>> {
    let current_user_info_routes = Router::new()
        .route(
            "/users/level",
            get(user_level_controller::get_current_user_level),
        )
        .route(
            "/users/rate-limit",
            get(rate_limit_controller::get_current_user_rate_limit),
        );

    let view_permission_routes = Router::new()
        .route(
            "/user-levels",
            get(user_level_controller::get_all_user_levels),
        )
        .route(
            "/user-levels/level/{level_code}",
            get(user_level_controller::get_user_level_by_code),
        )
        .route(
            "/user-levels/{id}/detail",
            get(user_level_controller::get_user_level_detail),
        )
        .route(
            "/users/{user_id}/level",
            get(user_level_controller::get_user_level),
        )
        .layer(axum::middleware::from_fn(require_permission(
            "ai:model:view",
        )));

    let manage_permission_routes = Router::new()
        .route(
            "/user-levels",
            post(user_level_controller::create_user_level),
        )
        .route(
            "/user-levels/{id}",
            put(user_level_controller::update_user_level_config),
        )
        .route(
            "/user-levels/batch/status",
            put(user_level_controller::batch_update_user_level_status),
        )
        .route(
            "/user-levels/{id}",
            delete(user_level_controller::delete_user_level),
        )
        .route(
            "/user-levels/batch",
            delete(user_level_controller::batch_delete_user_levels),
        )
        .route(
            "/users/{user_id}/level",
            put(user_level_controller::update_user_level),
        )
        .route(
            "/user-levels/init",
            post(user_level_controller::init_user_levels),
        )
        .layer(axum::middleware::from_fn(require_permission(
            "sys:user:edit",
        )));

    Router::new()
        .merge(current_user_info_routes)
        .merge(view_permission_routes)
        .merge(manage_permission_routes)
        .layer(axum::middleware::from_fn(auth))
}
