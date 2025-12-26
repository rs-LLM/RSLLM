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

// 用途：导入角色VO
// 说明：用于返回角色信息
use crate::domain::vo::basic::SysRoleVO;

// 用途：导入PageWrapper
// 说明：用于分页响应的OpenAPI文档生成
use crate::domain::vo::response::PageWrapper;

// 用途：导入axum的Json提取器
// 说明：用于从HTTP请求体中提取JSON数据
use axum::Json;

// 用途：导入响应转换特性
// 说明：用于将函数返回值转换为HTTP响应
use axum::response::IntoResponse;

/// 用途：获取所有角色层级结构
/// 说明：返回角色的层级关系，便于前端展示角色树
#[utoipa::path(
    get,
    path = "/api/v1/role/layer-top",
    responses(
        (status = 200, description = "查询成功", body = RespVO<Vec<SysRoleVO>>),
        (status = 400, description = "查询失败", body = RespVO<Vec<SysRoleVO>>)
    ),
    tag = "role"
)]
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
#[utoipa::path(
    post,
    path = "/api/v1/role/add",
    request_body = SysRoleResAddDTO,
    responses(
        (status = 200, description = "添加成功", body = RespVO<u64>),
        (status = 400, description = "添加失败", body = RespVO<u64>)
    ),
    tag = "role"
)]
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
#[utoipa::path(
    post,
    path = "/api/v1/role/page",
    request_body = SysRoleResPageDTO,
    responses(
        (status = 200, description = "查询成功", body = RespVO<PageWrapper<crate::domain::vo::basic::SysRoleVO>>),
        (status = 400, description = "查询失败", body = RespVO<PageWrapper<crate::domain::vo::basic::SysRoleVO>>)
    ),
    tag = "role"
)]
pub async fn page(arg: Json<SysRoleResPageDTO>) -> impl IntoResponse {
    // 用途：调用角色权限服务分页查询角色
    // 说明：角色和权限是关联的，需要同时查询角色和权限的关系
    let vo = CONTEXT.rbac_role_permission_service.page(&arg.0).await;
    // 用途：将结果转换为PageWrapper
    // 说明：将rbatis Page转换为PageWrapper以支持OpenAPI文档生成
    let wrapper_data: Result<PageWrapper<crate::domain::vo::basic::SysRoleVO>, _> = vo.map(|p| PageWrapper {
                page: p.page_no,
                page_size: p.page_size,
                total: p.total,
                records: p.records,
            });
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(wrapper_data)
}

/// 用途：更新角色
/// 说明：处理角色的更新请求
#[utoipa::path(
    post,
    path = "/api/v1/role/update",
    request_body = SysRoleResUpdateDTO,
    responses(
        (status = 200, description = "更新成功", body = RespVO<u64>),
        (status = 400, description = "更新失败", body = RespVO<u64>)
    ),
    tag = "role"
)]
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
#[utoipa::path(
    post,
    path = "/api/v1/role/remove",
    request_body = IdDTO,
    responses(
        (status = 200, description = "删除成功", body = RespVO<u64>),
        (status = 400, description = "删除失败", body = RespVO<u64>)
    ),
    tag = "role"
)]
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
