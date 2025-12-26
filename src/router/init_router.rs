// 用途：初始化向导相关路由
// 说明：提供系统初始化过程中需要的API接口
use axum::Router;
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::controller::init_controller;

/// 用途：创建初始化向导路由
/// 说明：提供系统初始化所需的所有API端点
pub fn create_init_router() -> Router<Arc<ServiceContext>> {
    Router::new()
        // 检查系统初始化状态
        .route("/api/v1/init/check", axum::routing::get(init_controller::check_init_status))
        // 验证数据库连接
        .route("/api/v1/init/verify-db", axum::routing::post(init_controller::verify_db_connection))
        // 生成JWT密钥
        .route("/api/v1/init/generate-jwt", axum::routing::get(init_controller::generate_jwt_secret))
        // 创建超级管理员
        .route("/api/v1/init/create-admin", axum::routing::post(init_controller::create_super_admin))
        // 保存初始化配置
        .route("/api/v1/init/config", axum::routing::post(init_controller::save_init_config))
}