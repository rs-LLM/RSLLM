//! 管理路由模块
//! 提供模型和供应商管理的RESTful API路由

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::controller::ai_hub::{model_definition_controller, provider_config_controller, quota_controller, bill_controller};

use crate::middleware::auth_axum::auth;
use crate::middleware::permission_middleware::require_permission;

/// 创建管理路由
///
/// 提供以下API端点：
/// - 模型管理：创建、查询、更新、删除、批量导入
/// - 供应商管理：创建、查询、更新、删除
/// - 配额管理：查询、创建、更新、充值、分配
/// - 账单管理：生成、支付、查询、更新、取消
pub fn create_admin_router() -> Router<Arc<ServiceContext>> {
    // 用途：创建配额管理路由组
    // 说明：配额管理需要manage_quotas权限
    let quota_management_routes = Router::new()
        // 用途：定义配额创建路由
        // 说明：创建配额（需要manage_quotas权限）
        .route("/api/v1/admin/quota", post(quota_controller::create_quota))
        // 用途：定义配额列表路由
        // 说明：查询配额列表（需要manage_quotas权限）
        .route("/api/v1/admin/quota", get(quota_controller::list_admin_quotas))
        // 用途：定义配额更新路由
        // 说明：更新配额信息（需要manage_quotas权限）
        .route("/api/v1/admin/quota/{id}", put(quota_controller::update_quota))
        // 用途：定义配额详情路由
        // 说明：获取配额详情（需要manage_quotas权限）
        .route("/api/v1/admin/quota/{id}", get(quota_controller::get_admin_quota))
        // 用途：定义配额充值路由
        // 说明：为配额充值（需要manage_quotas权限）
        .route("/api/v1/admin/quota/{id}/recharge", post(quota_controller::recharge_quota))
        // 用途：定义配额分配路由
        // 说明：分配配额（需要manage_quotas权限）
        .route("/api/v1/admin/quota/allocate", post(quota_controller::allocate_quota))
        .layer(axum::middleware::from_fn(require_permission("manage_quotas")));

    Router::new()
        // 模型管理路由（仅包含需要认证的操作）
        .route("/api/v1/admin/models", post(model_definition_controller::create_model))
        .route("/api/v1/admin/models/key/{key}", get(model_definition_controller::get_model_by_key))
        .route("/api/v1/admin/models/{id}", put(model_definition_controller::update_model))
        .route("/api/v1/admin/models/{id}", delete(model_definition_controller::delete_model))
        .route("/api/v1/admin/models/bulk", post(model_definition_controller::bulk_import_models))
        
        // 供应商管理路由
        .route("/api/v1/admin/providers", post(provider_config_controller::create_provider))
        .route("/api/v1/admin/providers", get(provider_config_controller::list_providers))
        .route("/api/v1/admin/providers/{id}", get(provider_config_controller::get_provider))
        .route("/api/v1/admin/providers/name/{name}", get(provider_config_controller::get_provider_by_name))
        .route("/api/v1/admin/providers/{id}", put(provider_config_controller::update_provider))
        .route("/api/v1/admin/providers/{id}", delete(provider_config_controller::delete_provider))
        
        // 账单管理路由
        .route("/api/v1/admin/bills/generate", post(bill_controller::generate_test_billing))
        .route("/api/v1/admin/bills", get(bill_controller::get_billing_list))
        .route("/api/v1/admin/bills/{id}", put(bill_controller::update_billing))
        .route("/api/v1/admin/bills/{id}/pay", post(bill_controller::pay_billing))
        // 用途：合并配额管理路由
        // 说明：将需要权限控制的配额管理路由合并到主路由
        .merge(quota_management_routes)
        // 用途：添加认证中间件
        // 说明：保护管理员路由，确保只有已登录用户可以访问
        .layer(axum::middleware::from_fn(auth))
}