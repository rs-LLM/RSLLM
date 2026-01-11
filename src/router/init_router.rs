// 用途：初始化向导相关路由
// 说明：提供系统初始化过程中需要的API接口
use axum::Router;
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::controller::init_controller;

/// 用途：创建初始化向导路由
/// 说明：提供系统初始化所需的所有API端点
/// 注意：所有路由都通过 /rsllm/api 前缀访问，完整路径为：
///   - GET  /rsllm/api/init/check - 检查系统初始化状态
///   - POST /rsllm/api/init/verify-db - 验证数据库连接
///   - POST /rsllm/api/init/generate-jwt - 生成JWT密钥
///   - POST /rsllm/api/init/create-admin - 创建超级管理员
///   - GET  /rsllm/api/init/progress - 获取初始化进度
///   - POST /rsllm/api/init/config - 保存初始化配置
pub fn create_init_router() -> Router<Arc<ServiceContext>> {
    Router::new()
        // 检查系统初始化状态
        .route(
            "/init/check",
            axum::routing::get(init_controller::check_init_status),
        )
        // 验证数据库连接
        .route(
            "/init/verify-db",
            axum::routing::post(init_controller::verify_db_connection),
        )
        // 生成JWT密钥
        .route(
            "/init/generate-jwt",
            axum::routing::post(init_controller::generate_jwt_secret),
        )
        // 创建超级管理员
        .route(
            "/init/create-admin",
            axum::routing::post(init_controller::create_super_admin),
        )
        // 获取初始化进度
        .route(
            "/init/progress",
            axum::routing::get(init_controller::get_init_progress),
        )
        // 保存初始化配置
        .route(
            "/init/config",
            axum::routing::post(init_controller::save_init_config),
        )
}
