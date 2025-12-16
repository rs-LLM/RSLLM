// 用途：导入全局上下文
// 说明：用于访问权限相关的服务实例
use crate::context::CONTEXT;

// 用途：导入ID数据传输对象
// 说明：用于接收删除权限请求中的ID参数
use crate::domain::dto::IdDTO;

// 用途：导入权限相关的数据传输对象
// 说明：用于接收权限的添加、分页查询和更新请求参数
use crate::domain::dto::rbac::{PermissionAddDTO, PermissionPageDTO, ResEditDTO};

// 用途：导入权限表结构
// 说明：用于将DTO转换为数据库实体
use crate::domain::table::rbac::RbacPermission;

// 用途：导入响应VO
// 说明：用于统一HTTP响应格式
use crate::domain::vo::RespVO;

// 用途：导入错误信息宏
// 说明：用于生成错误信息
use crate::error_info;

// 用途：导入axum的Json提取器
// 说明：用于从HTTP请求体中提取JSON数据
use axum::Json;

// 用途：导入响应转换特性
// 说明：用于将函数返回值转换为HTTP响应
use axum::response::IntoResponse;

/// 用途：获取所有权限层级结构
/// 说明：返回权限的层级关系，便于前端展示权限树
pub async fn layer_top() -> impl IntoResponse {
    // 用途：调用权限服务获取所有权限
    // 说明：从数据库中查询权限数据
    let data = CONTEXT.rbac_permission_service.finds_all().await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(data)
}

/// 用途：分页查询权限
/// 说明：处理权限的分页查询请求
pub async fn page(page: Json<PermissionPageDTO>) -> impl IntoResponse {
    // 用途：调用权限服务分页查询权限
    // 说明：从数据库中分页查询权限数据
    let data = CONTEXT.rbac_permission_service.page(&page.0).await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(data)
}

/// 用途：添加权限
/// 说明：处理权限的添加请求
pub async fn add(mut arg: Json<PermissionAddDTO>) -> impl IntoResponse {
    // 用途：检查权限名称是否为空
    // 说明：权限名称是必填项
    if arg.name.is_none() {
        return RespVO::<u64>::from_error(error_info!("arg.name_empty"));
    }
    // 用途：检查权限标识是否为空
    // 说明：权限标识是必填项，用于权限验证
    if arg.permission.is_none() {
        return RespVO::<u64>::from_error(error_info!("arg.permission_empty"));
    }
    // 用途：检查路径是否为空
    // 说明：路径可以为空，为空时设置为空字符串
    if arg.path.is_none() {
        arg.path = Some("".to_string());
    }
    // 用途：将DTO转换为数据库实体
    // 说明：数据库操作需要使用实体对象
    let res = RbacPermission::from(arg.0);
    // 用途：调用权限服务添加权限
    // 说明：将权限数据保存到数据库
    let data = CONTEXT.rbac_permission_service.add(&res).await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(data)
}

/// 用途：更新权限
/// 说明：处理权限的更新请求
pub async fn update(arg: Json<ResEditDTO>) -> impl IntoResponse {
    // 用途：调用权限服务更新权限
    // 说明：更新数据库中的权限数据
    let data = CONTEXT.rbac_permission_service.edit(&arg.0).await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(data)
}

/// 用途：删除权限
/// 说明：处理权限的删除请求
pub async fn remove(arg: Json<IdDTO>) -> impl IntoResponse {
    // 用途：调用权限服务删除权限
    // 说明：从数据库中删除指定ID的权限
    let data = CONTEXT
        .rbac_permission_service
        .remove(&arg.0.id.unwrap_or_default())
        .await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(data)
}
