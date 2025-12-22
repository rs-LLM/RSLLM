//! 管理路由模块
//! 提供模型和供应商管理的RESTful API路由

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::controller::ai_hub::{model_definition_controller, provider_config_controller, quota_controller, bill_controller};

/// 创建管理路由
///
/// 提供以下API端点：
/// - 模型管理：创建、查询、更新、删除、批量导入
/// - 供应商管理：创建、查询、更新、删除
/// - 配额管理：查询、创建、更新、充值、分配
/// - 账单管理：生成、支付、查询、更新、取消
pub fn create_admin_router() -> Router<Arc<ServiceContext>> {
    Router::new()
        // 模型管理路由
        .route("/api/v1/admin/models", post(model_definition_controller::create_model))
        .route("/api/v1/admin/models", get(model_definition_controller::list_models))
        .route("/api/v1/admin/models/:id", get(model_definition_controller::get_model))
        .route("/api/v1/admin/models/key/:key", get(model_definition_controller::get_model_by_key))
        .route("/api/v1/admin/models/:id", put(model_definition_controller::update_model))
        .route("/api/v1/admin/models/:id", delete(model_definition_controller::delete_model))
        .route("/api/v1/admin/models/bulk", post(model_definition_controller::bulk_import_models))
        
        // 供应商管理路由
        .route("/api/v1/admin/providers", post(provider_config_controller::create_provider))
        .route("/api/v1/admin/providers", get(provider_config_controller::list_providers))
        .route("/api/v1/admin/providers/:id", get(provider_config_controller::get_provider))
        .route("/api/v1/admin/providers/name/:name", get(provider_config_controller::get_provider_by_name))
        .route("/api/v1/admin/providers/:id", put(provider_config_controller::update_provider))
        .route("/api/v1/admin/providers/:id", delete(provider_config_controller::delete_provider))
        
        // 配额管理路由
        .route("/api/v1/admin/quota", post(quota_controller::create_quota))
        .route("/api/v1/admin/quota/:id", put(quota_controller::update_quota))
        .route("/api/v1/admin/quota/:id/recharge", post(quota_controller::recharge_quota))
        .route("/api/v1/admin/quota/allocate", post(quota_controller::allocate_quota))
        
        // 账单管理路由
        .route("/api/v1/admin/bills/generate", post(bill_controller::generate_test_billing))
        .route("/api/v1/admin/bills", get(bill_controller::get_billing_list))
        .route("/api/v1/admin/bills/:id", put(bill_controller::update_billing))
        .route("/api/v1/admin/bills/:id/pay", post(bill_controller::pay_billing))
}