// 用途：导入全局上下文
// 说明：用于访问角色相关的服务实例
use crate::context::CONTEXT;

// 用途：导入ID数据传输对象
// 说明：用于接收删除角色请求中的ID参数
use crate::domain::dto::IdDTO;

// 用途：导入角色相关的数据传输对象
// 说明：用于接收角色的添加、分页查询和更新请求参数
use crate::domain::dto::rbac::{SysRoleResAddDTO, SysRoleResPageDTO, SysRoleResUpdateDTO};

// 用途：导入响应VO
// 说明：用于统一HTTP响应格式
use crate::domain::vo::RespVO;

// 用途：导入axum的Json提取器
// 说明：用于从HTTP请求体中提取JSON数据
use axum::Json;

// 用途：导入响应转换特性
// 说明：用于将函数返回值转换为HTTP响应
use axum::response::IntoResponse;

/// 用途：获取所有角色层级结构
/// 说明：返回角色的层级关系，便于前端展示角色树
pub async fn layer_top() -> impl IntoResponse {
    // 用途：调用角色服务获取所有角色
    // 说明：从数据库中查询角色数据
    let vo = CONTEXT.rbac_role_service.find_all().await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(vo)
}

/// 用途：添加角色
/// 说明：处理角色的添加请求
pub async fn add(arg: Json<SysRoleResAddDTO>) -> impl IntoResponse {
    // 用途：调用角色权限服务添加角色
    // 说明：角色和权限是关联的，需要同时处理角色和权限的关系
    let vo = CONTEXT.rbac_role_permission_service.add(&arg.0).await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(vo)
}

/// 用途：分页查询角色
/// 说明：处理角色的分页查询请求
pub async fn page(arg: Json<SysRoleResPageDTO>) -> impl IntoResponse {
    // 用途：调用角色权限服务分页查询角色
    // 说明：角色和权限是关联的，需要同时查询角色和权限的关系
    let vo = CONTEXT.rbac_role_permission_service.page(&arg.0).await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(vo)
}

/// 用途：更新角色
/// 说明：处理角色的更新请求
pub async fn update(arg: Json<SysRoleResUpdateDTO>) -> impl IntoResponse {
    // 用途：调用角色权限服务更新角色
    // 说明：角色和权限是关联的，需要同时更新角色和权限的关系
    let vo = CONTEXT.rbac_role_permission_service.edit(&arg.0).await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(vo)
}

/// 用途：删除角色
/// 说明：处理角色的删除请求
pub async fn remove(arg: Json<IdDTO>) -> impl IntoResponse {
    // 用途：获取角色ID
    // 说明：从请求中提取需要删除的角色ID
    let role_id = arg.0.id.unwrap_or_default();
    // 用途：调用角色权限服务删除角色
    // 说明：角色和权限是关联的，需要同时删除角色和权限的关系
    let vo = CONTEXT
        .rbac_role_permission_service
        .remove_role(&role_id)
        .await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(vo)
}
