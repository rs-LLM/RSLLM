// 用途：声明权限服务模块
// 说明：处理权限相关业务逻辑，如权限的增删改查
mod rbac_permission_service;

// 用途：声明权限审计日志服务模块
// 说明：处理权限审计日志相关业务逻辑，如记录权限的创建、修改、删除操作
mod rbac_permission_audit_log_service;

// 用途：声明角色权限服务模块
// 说明：处理角色和权限的关联关系，如给角色分配权限
mod rbac_role_permission_service;

// 用途：声明角色服务模块
// 说明：处理角色相关业务逻辑，如角色的增删改查
mod rbac_role_service;

// 用途：声明用户角色服务模块
// 说明：处理用户和角色的关联关系，如给用户分配角色
mod rbac_user_role_service;

// 用途：导出权限服务
// 说明：允许其他模块访问权限相关功能
pub use rbac_permission_service::*;

// 用途：导出权限审计日志服务
// 说明：允许其他模块访问权限审计日志相关功能
pub use rbac_permission_audit_log_service::*;

// 用途：导出角色权限服务
// 说明：允许其他模块访问角色和权限关联功能
pub use rbac_role_permission_service::*;

// 用途：导出角色服务
// 说明：允许其他模块访问角色相关功能
pub use rbac_role_service::*;

// 用途：导出用户角色服务
// 说明：允许其他模块访问用户和角色关联功能
pub use rbac_user_role_service::*;
