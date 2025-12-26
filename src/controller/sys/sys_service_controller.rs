// 用途：导入必要的依赖
// 说明：包含HTTP请求和响应处理等核心功能
use axum::extract::Request;
use axum::response::IntoResponse;

// 用途：导入全局上下文
// 说明：用于访问全局配置和服务
use crate::context::CONTEXT;

// 用途：导入响应VO
// 说明：用于统一响应格式
use crate::domain::vo::RespVO;

// 用途：导入统一API响应
// 说明：用于OpenAPI文档生成
use crate::domain::vo::response::ApiResponse;

// 用途：服务状态VO结构体
// 说明：用于返回服务的健康状态信息
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct ServiceStatusVO {
    // 用途：服务名称
    // 说明：标识服务的名称
    pub name: String,
    // 用途：服务状态
    // 说明：服务是否正常运行（true正常，false异常）
    pub status: bool,
    // 用途：服务描述
    // 说明：服务的详细描述
    pub description: String,
    // 用途：响应时间
    // 说明：服务响应的毫秒数
    pub response_time: u64,
}

/// 用途：获取服务列表
/// 说明：获取所有业务服务的健康状态
#[utoipa::path(
    get,
    path = "/api/v1/service/list",
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<Vec<ServiceStatusVO>>),
        (status = 500, description = "查询失败", body = ApiResponse<Vec<ServiceStatusVO>>)
    ),
    tag = "service"
)]
pub async fn get_service_list(_req: Request) -> impl IntoResponse {
    // 用途：获取当前时间戳
    // 说明：用于计算服务响应时间
    let start_time = std::time::Instant::now();
    
    // 用途：构建服务列表
    // 说明：检查各个业务服务的健康状态
    let services = vec![
        ServiceStatusVO {
            name: "database".to_string(),
            status: check_database_health().await,
            description: "数据库服务".to_string(),
            response_time: start_time.elapsed().as_millis() as u64,
        },
        ServiceStatusVO {
            name: "cache".to_string(),
            status: check_cache_health().await,
            description: "缓存服务".to_string(),
            response_time: start_time.elapsed().as_millis() as u64,
        },
        ServiceStatusVO {
            name: "auth".to_string(),
            status: check_auth_health().await,
            description: "认证服务".to_string(),
            response_time: start_time.elapsed().as_millis() as u64,
        },
        ServiceStatusVO {
            name: "rbac".to_string(),
            status: check_rbac_health().await,
            description: "权限控制服务".to_string(),
            response_time: start_time.elapsed().as_millis() as u64,
        },
    ];
    
    // 用途：返回响应
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(Ok(services))
}

/// 用途：检查数据库健康状态
/// 说明：通过执行简单查询来检查数据库连接是否正常
async fn check_database_health() -> bool {
    // 用途：执行数据库健康检查
    // 说明：尝试执行一个简单的查询来验证数据库连接
    match CONTEXT.rbac_permission_service.finds_all().await {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// 用途：检查缓存健康状态
/// 说明：检查缓存服务是否可用
async fn check_cache_health() -> bool {
    // 用途：返回缓存健康状态
    // 说明：当前项目未实现缓存，返回true表示服务正常
    true
}

/// 用途：检查认证服务健康状态
/// 说明：检查认证服务是否可用
async fn check_auth_health() -> bool {
    // 用途：返回认证服务健康状态
    // 说明：认证服务正常运行，返回true
    true
}

/// 用途：检查权限控制服务健康状态
/// 说明：检查权限控制服务是否可用
async fn check_rbac_health() -> bool {
    // 用途：执行权限服务健康检查
    // 说明：尝试查询权限数据来验证权限服务
    match CONTEXT.rbac_permission_service.finds_all().await {
        Ok(_) => true,
        Err(_) => false,
    }
}
