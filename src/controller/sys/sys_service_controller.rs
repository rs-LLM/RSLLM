use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::context::CONTEXT;

use crate::domain::vo::response::ApiResponse;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct ServiceStatusVO {
    pub name: String,
    pub status: bool,
    pub description: String,
    pub response_time: u64,
}

#[utoipa::path(
    get,
    path = "/service/list",
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<Vec<ServiceStatusVO>>),
        (status = 500, description = "查询失败", body = ApiResponse<Vec<ServiceStatusVO>>)
    ),
    tag = "service"
)]
pub async fn get_service_list(_req: Request) -> impl IntoResponse {
    let start_time = std::time::Instant::now();

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

    (StatusCode::OK, axum::Json(ApiResponse::success(services)))
}

async fn check_database_health() -> bool {
    CONTEXT.rbac_permission_service.finds_all().await.is_ok()
}

async fn check_cache_health() -> bool {
    true
}

async fn check_auth_health() -> bool {
    true
}

async fn check_rbac_health() -> bool {
    (CONTEXT.rbac_permission_service.finds_all().await).is_ok()
}
