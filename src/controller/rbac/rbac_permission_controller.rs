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

// 用途：导入权限VO
// 说明：用于返回权限数据
use crate::domain::vo::rbac::RbacPermissionVO;

// 用途：导入权限审计日志VO
// 说明：用于返回权限审计日志数据
use crate::domain::vo::rbac::RbacPermissionAuditLogVO;

// 用途：导入PageWrapper
// 说明：用于分页响应的OpenAPI文档生成
use crate::domain::vo::response::PageWrapper;

/// 用途：获取所有权限层级结构
/// 说明：返回权限的层级关系，便于前端展示权限树
#[utoipa::path(
    post,
    path = "/admin/sys_permission_layer_top",
    responses(
        (status = 200, description = "查询成功", body = RespVO<Vec<RbacPermissionVO>>),
        (status = 400, description = "查询失败", body = RespVO<Vec<RbacPermissionVO>>)
    ),
    tag = "permission"
)]
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
#[utoipa::path(
    post,
    path = "/admin/sys_permission_page",
    request_body = PermissionPageDTO,
    responses(
        (status = 200, description = "查询成功", body = RespVO<PageWrapper<RbacPermissionVO>>),
        (status = 400, description = "查询失败", body = RespVO<PageWrapper<RbacPermissionVO>>)
    ),
    tag = "permission"
)]
pub async fn page(page: Json<PermissionPageDTO>) -> impl IntoResponse {
    // 用途：调用权限服务分页查询权限
    // 说明：从数据库中分页查询权限数据
    let data = CONTEXT.rbac_permission_service.page(&page.0).await;
    // 用途：将结果转换为PageWrapper
    // 说明：将rbatis Page转换为PageWrapper以支持OpenAPI文档生成
    let wrapper_data: Result<PageWrapper<RbacPermissionVO>, _> = data.map(|p| PageWrapper {
        page: p.page_no,
        page_size: p.page_size,
        total: p.total,
        records: p.records,
    });
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(wrapper_data)
}

/// 用途：添加权限
/// 说明：处理权限的添加请求
#[utoipa::path(
    post,
    path = "/admin/sys_permission_add",
    request_body = PermissionAddDTO,
    responses(
        (status = 200, description = "添加成功", body = RespVO<u64>),
        (status = 400, description = "添加失败", body = RespVO<u64>)
    ),
    tag = "permission"
)]
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

/// 用途：按类型查询权限
/// 说明：根据权限类型查询权限列表
pub async fn query_by_type(permission_type: Json<String>) -> impl IntoResponse {
    // 用途：调用权限服务按类型查询权限
    // 说明：从数据库中查询指定类型的权限数据
    let data = CONTEXT
        .rbac_permission_service
        .find_by_type(&permission_type.0)
        .await;
    // 用途：将结果转换为VO列表
    // 说明：将数据库实体转换为视图对象
    let vo_data: Result<Vec<RbacPermissionVO>, _> = data.map(|permissions| {
        permissions.into_iter().map(RbacPermissionVO::from).collect()
    });
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(vo_data)
}

/// 用途：更新权限状态
/// 说明：启用或禁用指定权限
pub async fn update_status(arg: Json<serde_json::Value>) -> impl IntoResponse {
    // 用途：提取权限ID
    // 说明：从请求参数中获取权限ID
    let id = arg.get("id").and_then(|v| v.as_str()).unwrap_or("");
    // 用途：提取状态值
    // 说明：从请求参数中获取状态值
    let status = arg.get("status").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
    // 用途：调用权限服务更新权限状态
    // 说明：更新数据库中的权限状态
    let data = CONTEXT
        .rbac_permission_service
        .update_status(id, status)
        .await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(data)
}

/// 用途：查询权限审计日志
/// 说明：获取指定权限的审计日志记录
pub async fn query_audit_logs(arg: Json<IdDTO>) -> impl IntoResponse {
    // 用途：提取权限ID
    // 说明：从请求参数中获取权限ID
    let permission_id = arg.0.id.unwrap_or_default();
    // 用途：调用权限审计日志服务查询审计日志
    // 说明：从数据库中查询指定权限的审计日志数据
    let data = CONTEXT
        .rbac_permission_audit_log_service
        .query_logs(&permission_id)
        .await;
    // 用途：将结果转换为VO列表
    // 说明：将数据库实体转换为视图对象
    let vo_data: Result<Vec<RbacPermissionAuditLogVO>, _> = data.map(|logs| {
        logs.into_iter().map(RbacPermissionAuditLogVO::from).collect()
    });
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(vo_data)
}
