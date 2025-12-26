//! 需认证路由模块
//! 
//! 该模块定义需要身份验证的API接口，包括：
//! - 用户管理接口
//! - 角色管理接口
//! - 权限管理接口
//! - 字典管理接口

use axum::Router;
use axum::routing::{get, post};
use std::sync::Arc;

// 用途：导入控制器
// 说明：用于处理HTTP请求
use crate::controller::{
    rbac_permission_controller,
    rbac_role_controller,
    rbac_user_controller,
    sys_auth_controller,
    sys_dict_controller,
    sys_menu_controller,
    sys_organization_controller,
    sys_service_controller,
};
use crate::controller::ai_hub::{balance_controller, transaction_controller};

// 用途：导入认证中间件
// 说明：用于保护需认证的路由
use crate::middleware::auth_axum::auth;
use crate::middleware::permission_middleware::require_permission;
use crate::context::ServiceContext;

/// 创建需认证路由
/// 
/// 返回需要身份验证的路由配置，包含认证中间件
pub fn create_auth_router() -> Router<Arc<ServiceContext>> {
    let user_management_routes = Router::new()
        // 用途：定义用户添加路由
        // 说明：添加新用户（需要manage_users权限）
        .route("/admin/sys_user_add", post(rbac_user_controller::add))
        // 用途：定义用户分页查询路由
        // 说明：分页查询用户列表（需要manage_users权限）
        .route("/admin/sys_user_page", post(rbac_user_controller::page))
        // 用途：定义用户删除路由
        // 说明：删除用户（需要manage_users权限）
        .route("/admin/sys_user_remove", post(rbac_user_controller::remove))
        // 用途：定义用户更新路由
        // 说明：更新用户信息（需要manage_users权限）
        .route("/admin/sys_user_update", post(rbac_user_controller::update))
        .layer(axum::middleware::from_fn(require_permission("manage_users")));

    let balance_management_routes = Router::new()
        // 用途：定义余额充值路由
        // 说明：用户余额充值（需要manage_balance权限）
        .route("/api/v1/balance/recharge", post(balance_controller::recharge))
        // 用途：定义设置余额路由
        // 说明：管理员设置用户余额（需要manage_balance权限）
        .route("/api/v1/balance/set", post(balance_controller::set_balance))
        .layer(axum::middleware::from_fn(require_permission("manage_balance")));

    Router::new()
        // 用途：定义用户信息路由
        // 说明：获取当前登录用户信息
        .route("/admin/sys_user_info", post(rbac_user_controller::info))
        .route("/admin/info", get(rbac_user_controller::info))
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
        // 用途：定义按类型查询权限路由
        // 说明：根据权限类型查询权限列表
        .route(
            "/admin/sys_permission_query_by_type",
            post(rbac_permission_controller::query_by_type),
        )
        // 用途：定义更新权限状态路由
        // 说明：启用或禁用指定权限
        .route(
            "/admin/sys_permission_update_status",
            post(rbac_permission_controller::update_status),
        )
        // 用途：定义查询权限审计日志路由
        // 说明：获取指定权限的审计日志记录
        .route(
            "/admin/sys_permission_audit_logs",
            post(rbac_permission_controller::query_audit_logs),
        )
        .merge(user_management_routes)
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
        // 用途：定义获取所有菜单路由
        // 说明：获取当前用户的所有菜单，符合Vben前端期望的端点格式
        .route("/api/v1/menu/all", get(sys_menu_controller::get_all_menus))
        // 用途：定义组织分页查询路由
        // 说明：分页查询组织列表
        .route("/admin/sys_organization_page", post(sys_organization_controller::page))
        // 用途：定义获取组织树路由
        // 说明：获取组织的层级结构
        .route("/admin/sys_organization_tree", post(sys_organization_controller::tree))
        // 用途：定义添加组织路由
        // 说明：添加新组织
        .route("/admin/sys_organization_add", post(sys_organization_controller::add))
        // 用途：定义更新组织路由
        // 说明：更新组织信息
        .route("/admin/sys_organization_update", post(sys_organization_controller::update))
        // 用途：定义删除组织路由
        // 说明：删除组织
        .route("/admin/sys_organization_remove", post(sys_organization_controller::remove))
        // 用途：定义获取所有组织路由
        // 说明：获取所有组织数据
        .route("/admin/sys_organization_list", post(sys_organization_controller::list))
        // 用途：定义获取组织详情路由
        // 说明：获取指定组织的详细信息
        .route("/admin/sys_organization_detail", post(sys_organization_controller::detail))
        // 用途：定义获取服务列表路由
        // 说明：获取所有业务服务的健康状态
        .route("/admin/service", post(sys_service_controller::get_service_list))
        // 用途：定义获取权限码路由
        // 说明：获取当前用户的权限码列表，用于Vben前端权限控制
        .route("/admin/auth/codes", get(sys_auth_controller::get_codes))
        // 用途：定义获取权限码路由（V3 API格式）
        // 说明：获取当前用户的权限码列表，用于Vben前端权限控制
        .route("/api/v1/auth/codes", get(sys_auth_controller::get_codes))
        // 用途：定义余额查询路由
        // 说明：查询用户余额
        .route("/api/v1/balance/get", post(balance_controller::get_balance))
        // 用途：定义余额扣减路由
        // 说明：系统自动扣减余额
        .route("/api/v1/balance/deduct", post(balance_controller::deduct))
        // 用途：定义交易记录查询路由
        // 说明：查询交易记录列表
        .route("/api/v1/transaction/list", post(transaction_controller::list))
        .merge(balance_management_routes)
        // 用途：添加认证中间件
        // 说明：保护需认证的路由，确保只有已登录用户可以访问
        .layer(axum::middleware::from_fn(auth))
}