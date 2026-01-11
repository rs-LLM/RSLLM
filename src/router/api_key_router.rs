//! API密钥和速率限制路由器模块
//!
//! 提供API密钥管理和速率限制管理的路由配置

use axum::{
    Router,
    routing::{delete, get, post, put},
};

use std::sync::Arc;

use crate::context::ServiceContext;
use crate::controller::ai_hub::{api_key_controller, rate_limit_controller};
use crate::middleware::auth_axum::auth;

/// 创建API密钥和速率限制路由器
///
/// 提供以下API端点：
/// - POST /admin/api-keys - 创建API密钥
/// - GET /admin/api-keys - 查询API密钥列表
/// - GET /admin/api-keys/{id} - 获取API密钥详情
/// - PUT /admin/api-keys/{id} - 更新API密钥
/// - DELETE /admin/api-keys/{id} - 删除API密钥
/// - POST /admin/api-keys/validate - 验证API密钥
/// - GET /admin/rate-limit/{user_id} - 获取用户速率限制状态
/// - POST /admin/rate-limit/{user_id}/reset - 重置用户速率限制
/// - GET /admin/rate-limit/{user_id}/check - 检查用户配额
/// - GET /admin/rate-limit/{user_id}/level-config - 获取用户等级配置
pub fn create_api_key_and_rate_limit_router() -> Router<Arc<ServiceContext>> {
    Router::new()
        .route("/admin/api-keys", post(api_key_controller::create_api_key))
        .route("/admin/api-keys", get(api_key_controller::list_api_keys))
        .route("/admin/api-keys/{id}", get(api_key_controller::get_api_key))
        .route(
            "/admin/api-keys/{id}",
            put(api_key_controller::update_api_key),
        )
        .route(
            "/admin/api-keys/{id}",
            delete(api_key_controller::delete_api_key),
        )
        .route(
            "/admin/api-keys/validate",
            post(api_key_controller::validate_api_key),
        )
        .route(
            "/admin/rate-limit/{user_id}",
            get(rate_limit_controller::get_rate_limit_status),
        )
        .route(
            "/admin/rate-limit/{user_id}/reset",
            post(rate_limit_controller::reset_rate_limit),
        )
        .route(
            "/admin/rate-limit/{user_id}/check",
            get(rate_limit_controller::check_user_quota),
        )
        .route(
            "/admin/rate-limit/{user_id}/level-config",
            get(rate_limit_controller::get_user_level_config),
        )
        .layer(axum::middleware::from_fn(auth))
}
