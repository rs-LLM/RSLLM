// 用途：导入必要的依赖
// 说明：包含HTTP请求和响应处理等核心功能
use axum::{Json, response::IntoResponse};

// 用途：导入全局上下文
// 说明：用于访问全局配置和服务
use crate::context::CONTEXT;

// 用途：导入响应VO
// 说明：用于统一响应格式
use crate::domain::vo::RespVO;

// 用途：导入统一API响应
// 说明：用于OpenAPI文档生成
use crate::domain::vo::response::ApiResponse;

// 用途：导入PageWrapper
// 说明：用于分页响应
use crate::domain::vo::response::PageWrapper;

// 用途：导入组织相关的数据传输对象
// 说明：用于接收组织的添加、分页查询和更新请求参数
use crate::domain::dto::basic::sys_organization::{OrgAddDTO, OrgPageDTO, OrgEditDTO};

// 用途：导入ID数据传输对象
// 说明：用于接收删除组织请求中的ID参数
use crate::domain::dto::IdDTO;

// 用途：导入组织VO
// 说明：用于返回组织数据
use crate::domain::vo::basic::sys_organization::{SysOrganizationVO, OrganizationTreeNodeVO};

/// 用途：分页查询组织
/// 说明：处理组织的分页查询请求
#[utoipa::path(
    post,
    path = "/api/v1/organization/page",
    request_body = OrgPageDTO,
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<PageWrapper<SysOrganizationVO>>),
        (status = 400, description = "查询失败", body = ApiResponse<PageWrapper<SysOrganizationVO>>)
    ),
    tag = "organization"
)]
pub async fn page(page: Json<OrgPageDTO>) -> impl IntoResponse {
    // 用途：调用组织服务分页查询组织
    // 说明：从数据库中分页查询组织数据
    let data = CONTEXT.sys_organization_service.page(&page.0).await;
    // 用途：将结果转换为PageWrapper
    // 说明：将rbatis Page转换为PageWrapper以支持OpenAPI文档生成
    let wrapper_data: Result<PageWrapper<SysOrganizationVO>, _> = data.map(|p| PageWrapper {
        page: p.page_no,
        page_size: p.page_size,
        total: p.total,
        records: p.records,
    });
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(wrapper_data)
}

/// 用途：获取组织树
/// 说明：获取组织的层级结构，便于前端展示组织树
#[utoipa::path(
    get,
    path = "/api/v1/organization/tree",
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<Vec<OrganizationTreeNodeVO>>),
        (status = 500, description = "查询失败", body = ApiResponse<Vec<OrganizationTreeNodeVO>>)
    ),
    tag = "organization"
)]
pub async fn tree() -> impl IntoResponse {
    // 用途：调用组织服务获取组织树
    // 说明：从数据库中查询组织数据并构建树形结构
    let data = CONTEXT.sys_organization_service.find_tree().await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(data)
}

/// 用途：添加组织
/// 说明：处理组织的添加请求
#[utoipa::path(
    post,
    path = "/api/v1/organization/add",
    request_body = OrgAddDTO,
    responses(
        (status = 200, description = "添加成功", body = ApiResponse<String>),
        (status = 400, description = "添加失败", body = ApiResponse<String>)
    ),
    tag = "organization"
)]
pub async fn add(arg: Json<OrgAddDTO>) -> impl IntoResponse {
    // 用途：调用组织服务添加组织
    // 说明：将组织数据保存到数据库
    let data = CONTEXT.sys_organization_service.add(&arg.0).await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(data)
}

/// 用途：更新组织
/// 说明：处理组织的更新请求
#[utoipa::path(
    post,
    path = "/api/v1/organization/update",
    request_body = OrgEditDTO,
    responses(
        (status = 200, description = "更新成功", body = ApiResponse<String>),
        (status = 400, description = "更新失败", body = ApiResponse<String>)
    ),
    tag = "organization"
)]
pub async fn update(arg: Json<OrgEditDTO>) -> impl IntoResponse {
    // 用途：调用组织服务更新组织
    // 说明：更新数据库中的组织数据
    let data = CONTEXT.sys_organization_service.edit(&arg.0).await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(data)
}

/// 用途：删除组织
/// 说明：处理组织的删除请求
#[utoipa::path(
    post,
    path = "/api/v1/organization/remove",
    request_body = IdDTO,
    responses(
        (status = 200, description = "删除成功", body = ApiResponse<String>),
        (status = 400, description = "删除失败", body = ApiResponse<String>)
    ),
    tag = "organization"
)]
pub async fn remove(arg: Json<IdDTO>) -> impl IntoResponse {
    // 用途：获取组织ID
    // 说明：从请求参数中提取ID
    let id = arg.0.id.unwrap_or_else(|| String::new());
    // 用途：调用组织服务删除组织
    // 说明：从数据库中删除指定ID的组织
    let data = CONTEXT.sys_organization_service.remove(&id).await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(data)
}

/// 用途：获取所有组织
/// 说明：获取所有组织数据，不进行分页
#[utoipa::path(
    get,
    path = "/api/v1/organization/list",
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<Vec<SysOrganizationVO>>),
        (status = 500, description = "查询失败", body = ApiResponse<Vec<SysOrganizationVO>>)
    ),
    tag = "organization"
)]
pub async fn list() -> impl IntoResponse {
    // 用途：调用组织服务获取所有组织
    // 说明：从数据库中查询所有组织数据
    let data = CONTEXT.sys_organization_service.finds_all().await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(data)
}

/// 用途：根据ID获取组织详情
/// 说明：获取指定ID的组织的详细信息
#[utoipa::path(
    post,
    path = "/api/v1/organization/detail",
    request_body = IdDTO,
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<SysOrganizationVO>),
        (status = 400, description = "查询失败", body = ApiResponse<SysOrganizationVO>)
    ),
    tag = "organization"
)]
pub async fn detail(arg: Json<IdDTO>) -> impl IntoResponse {
    // 用途：调用组织服务获取组织详情
    // 说明：从数据库中查询指定ID的组织数据
    let data = CONTEXT
        .sys_organization_service
        .detail(&arg.0.id.unwrap_or_default())
        .await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(data)
}
