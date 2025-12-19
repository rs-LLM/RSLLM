//! 需认证路由模块
//! 
//! 该模块定义需要身份验证的API接口，包括：
//! - 用户管理接口
//! - 角色管理接口
//! - 权限管理接口
//! - 字典管理接口

use axum::Router;
use axum::routing::post;

// 用途：导入控制器
// 说明：用于处理HTTP请求
use crate::controller::{
    rbac_permission_controller,
    rbac_role_controller,
    rbac_user_controller,
    sys_dict_controller,
};

// 用途：导入认证中间件
// 说明：用于保护需认证的路由
use crate::middleware::auth_axum::auth;

/// 创建需认证路由
/// 
/// 返回需要身份验证的路由配置，包含认证中间件
pub fn create_auth_router() -> Router {
    Router::new()
        // 用途：定义用户信息路由
        // 说明：获取当前登录用户信息
        .route("/admin/sys_user_info", post(rbac_user_controller::info))
        // 用途：定义用户详情路由
        // 说明：获取指定用户的详细信息
        .route("/admin/sys_user_detail", post(rbac_user_controller::detail))
        // 用途：定义权限更新路由
        // 说明：更新权限信息
        .route(
            "/admin/sys_permission_update",
            post(rbac_permission_controller::update),
        )
        // 用途：定义权限删除路由
        // 说明：删除权限
        .route(
            "/admin/sys_permission_remove",
            post(rbac_permission_controller::remove),
        )
        // 用途：定义权限添加路由
        // 说明：添加新权限
        .route(
            "/admin/sys_permission_add",
            post(rbac_permission_controller::add),
        )
        // 用途：定义权限分页查询路由
        // 说明：分页查询权限列表
        .route(
            "/admin/sys_permission_page",
            post(rbac_permission_controller::page),
        )
        // 用途：定义权限层级查询路由
        // 说明：查询权限的层级结构
        .route(
            "/admin/sys_permission_layer_top",
            post(rbac_permission_controller::layer_top),
        )
        // 用途：定义用户添加路由
        // 说明：添加新用户
        .route("/admin/sys_user_add", post(rbac_user_controller::add))
        // 用途：定义用户分页查询路由
        // 说明：分页查询用户列表
        .route("/admin/sys_user_page", post(rbac_user_controller::page))
        // 用途：定义用户删除路由
        // 说明：删除用户
        .route("/admin/sys_user_remove", post(rbac_user_controller::remove))
        // 用途：定义用户更新路由
        // 说明：更新用户信息
        .route("/admin/sys_user_update", post(rbac_user_controller::update))
        // 用途：定义角色添加路由
        // 说明：添加新角色
        .route("/admin/sys_role_add", post(rbac_role_controller::add))
        // 用途：定义角色更新路由
        // 说明：更新角色信息
        .route("/admin/sys_role_update", post(rbac_role_controller::update))
        // 用途：定义角色删除路由
        // 说明：删除角色
        .route("/admin/sys_role_delete", post(rbac_role_controller::remove))
        // 用途：定义角色分页查询路由
        // 说明：分页查询角色列表
        .route("/admin/sys_role_page", post(rbac_role_controller::page))
        // 用途：定义角色层级查询路由
        // 说明：查询角色的层级结构
        .route(
            "/admin/sys_role_layer_top",
            post(rbac_role_controller::layer_top),
        )
        // 用途：定义字典添加路由
        // 说明：添加新字典
        .route("/admin/sys_dict_add", post(sys_dict_controller::add))
        // 用途：定义字典更新路由
        // 说明：更新字典信息
        .route("/admin/sys_dict_update", post(sys_dict_controller::update))
        // 用途：定义字典删除路由
        // 说明：删除字典
        .route("/admin/sys_dict_remove", post(sys_dict_controller::remove))
        // 用途：定义字典分页查询路由
        // 说明：分页查询字典列表
        .route("/admin/sys_dict_page", post(sys_dict_controller::page))
        // 用途：添加认证中间件
        // 说明：保护需认证的路由，确保只有已登录用户可以访问
        .layer(axum::middleware::from_fn(auth))
}