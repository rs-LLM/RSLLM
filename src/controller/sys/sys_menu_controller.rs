use crate::context::CONTEXT;

use crate::domain::vo::basic::rbac::RbacPermissionVO;

use crate::domain::vo::response::ApiResponse;

use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::middleware::auth_axum::TOKEN_KEY;

use crate::domain::vo::basic::jwt::JWTToken;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct MenuRouteVO {
    pub name: String,
    pub path: String,
    pub component: String,
    pub children: Vec<MenuRouteVO>,
    #[serde(rename = "meta")]
    pub meta: MenuMetaVO,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct MenuMetaVO {
    pub title: String,
    pub icon: Option<String>,
    pub authority: Option<Vec<String>>,
    #[serde(rename = "hideInMenu")]
    pub hide_in_menu: Option<bool>,
    pub order: Option<i32>,
}

#[utoipa::path(
    get,
    path = "/menu/all",
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<Vec<MenuRouteVO>>),
        (status = 401, description = "认证失败", body = ApiResponse<Vec<MenuRouteVO>>),
        (status = 500, description = "查询失败", body = ApiResponse<Vec<MenuRouteVO>>)
    ),
    tag = "menu"
)]
pub async fn get_all_menus(req: Request) -> impl IntoResponse {
    let token = req.headers().get(TOKEN_KEY);

    match token {
        Some(token) => {
            let token = token.to_str().unwrap_or("");
            let token = token.trim_start_matches("Bearer ");
            let jwt = JWTToken::verify(&CONTEXT.config.jwt_secret, token);

            match jwt {
                Ok(_jwt_token) => {
                    let permissions_result = CONTEXT.rbac_permission_service.finds_all().await;

                    match permissions_result {
                        Ok(permissions) => {
                            let menu_permissions: Vec<RbacPermissionVO> = permissions
                                .into_iter()
                                .filter(|p| {
                                    p.permission_type.as_deref() == Some("menu")
                                        || p.permission_type.is_none()
                                        || p.permission_type.as_deref() == Some("")
                                })
                                .map(RbacPermissionVO::from)
                                .collect();

                            let menu_tree = build_menu_tree(menu_permissions);

                            (StatusCode::OK, axum::Json(ApiResponse::success(menu_tree)))
                        }
                        Err(e) => (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            axum::Json(ApiResponse::error("-1", &e.to_string())),
                        ),
                    }
                }
                Err(e) => (
                    StatusCode::UNAUTHORIZED,
                    axum::Json(ApiResponse::error("-1", &e.to_string())),
                ),
            }
        }
        _ => (
            StatusCode::UNAUTHORIZED,
            axum::Json(ApiResponse::error("-1", "缺少访问令牌，请先登录")),
        ),
    }
}

fn build_menu_tree(permissions: Vec<RbacPermissionVO>) -> Vec<MenuRouteVO> {
    let mut menu_map: std::collections::HashMap<String, MenuRouteVO> =
        std::collections::HashMap::new();
    let mut children_map: std::collections::HashMap<String, Vec<MenuRouteVO>> =
        std::collections::HashMap::new();

    for permission in permissions {
        let menu_id = permission.id.clone().unwrap_or_default();
        let path = permission.path.clone().unwrap_or_default();
        let name = permission.name.clone().unwrap_or_default();

        let component = if path.is_empty() {
            String::new()
        } else {
            let normalized_path = path
                .trim_start_matches("views/")
                .trim_start_matches('/')
                .trim_end_matches(".vue");

            format!("/{}", normalized_path)
        };

        let meta = MenuMetaVO {
            title: name.clone(),
            icon: permission.icon.clone(),
            authority: permission.permission.clone().map(|p| vec![p]),
            hide_in_menu: Some(permission.status.is_some_and(|s| s != 1)),
            order: permission.sort_order,
        };

        let menu_node = MenuRouteVO {
            name: name.clone(),
            path: path.clone(),
            component,
            children: vec![],
            meta,
        };

        menu_map.insert(menu_id.clone(), menu_node);
    }

    for menu_node in menu_map.values() {
        if let Some(parent_path) = get_parent_path(&menu_node.path) {
            for (parent_id, parent_node) in &menu_map {
                if parent_node.path == parent_path {
                    children_map
                        .entry(parent_id.clone())
                        .or_default()
                        .push(menu_node.clone());
                }
            }
        }
    }

    for (parent_id, children) in children_map {
        if let Some(menu_node) = menu_map.get_mut(&parent_id) {
            menu_node.children = children;
        }
    }

    menu_map
        .into_values()
        .filter(|menu| {
            let path_parts: Vec<&str> = menu.path.split('/').filter(|s| !s.is_empty()).collect();
            path_parts.len() == 1
        })
        .collect()
}

fn get_parent_path(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('/').collect();

    if parts.len() <= 1 {
        return None;
    }

    Some(parts[..parts.len() - 1].join("/"))
}
