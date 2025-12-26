// 用途：导入全局上下文实例
// 说明：用于获取配置信息
use crate::context::CONTEXT;

// 用途：导入权限VO
// 说明：用于返回权限数据
use crate::domain::vo::basic::rbac::RbacPermissionVO;

// 用途：导入响应VO
// 说明：用于统一HTTP响应格式
use crate::domain::vo::RespVO;

// 用途：导入统一API响应
// 说明：用于OpenAPI文档生成
use crate::domain::vo::response::ApiResponse;

// 用途：导入响应转换特性
// 说明：用于将函数返回值转换为HTTP响应
use axum::response::IntoResponse;

// 用途：导入请求类型
// 说明：用于从请求中提取用户信息
use axum::extract::Request;

// 用途：导入JWT令牌相关
// 说明：用于验证用户身份
use crate::middleware::auth_axum::TOKEN_KEY;

// 用途：导入JWT令牌
// 说明：用于解析令牌获取用户信息
use crate::domain::vo::basic::jwt::JWTToken;

/// 用途：菜单路由VO结构体
// 说明：用于返回符合Vben前端期望的菜单路由格式
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct MenuRouteVO {
    // 用途：路由名称
    // 说明：用于标识路由
    pub name: String,
    // 用途：路由路径
    // 说明：用于前端路由
    pub path: String,
    // 用途：组件路径
    // 说明：前端组件的导入路径
    pub component: String,
    // 用途：子路由列表
    // 说明：递归结构，包含子路由
    pub children: Vec<MenuRouteVO>,
    // 用途：路由元信息
    // 说明：包含路由的额外信息
    #[serde(rename = "meta")]
    pub meta: MenuMetaVO,
}

/// 用途：菜单元信息VO结构体
// 说明：用于存储路由的元信息
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct MenuMetaVO {
    // 用途：标题
    // 说明：用于显示菜单名称
    pub title: String,
    // 用途：图标
    // 说明：用于前端菜单图标显示
    pub icon: Option<String>,
    // 用途：权限标识
    // 说明：需要特定的权限才可以访问
    pub authority: Option<Vec<String>>,
    // 用途：是否在菜单中隐藏
    // 说明：当前路由在菜单中不展现
    #[serde(rename = "hideInMenu")]
    pub hide_in_menu: Option<bool>,
    // 用途：排序
    // 说明：用于路由排序
    pub order: Option<i32>,
}

/// 用途：获取所有菜单
/// 说明：从权限数据动态生成菜单树结构，返回符合Vben前端期望的格式
#[utoipa::path(
    get,
    path = "/api/v1/menu/all",
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<Vec<MenuRouteVO>>),
        (status = 401, description = "认证失败", body = ApiResponse<Vec<MenuRouteVO>>),
        (status = 500, description = "查询失败", body = ApiResponse<Vec<MenuRouteVO>>)
    ),
    tag = "menu"
)]
pub async fn get_all_menus(req: Request) -> impl IntoResponse {
    use axum::http::StatusCode;
    // 用途：从请求头中获取令牌
    // 说明：JWT令牌存储在请求头中，用于身份验证
    let token = req.headers().get(TOKEN_KEY);
    
    // 用途：匹配令牌是否存在
    // 说明：处理令牌存在和不存在的情况
    match token {
        Some(token) => {
            // 用途：将令牌转换为字符串
            // 说明：方便后续处理
            let token = token.to_str().unwrap_or("");
            // 用途：去除Bearer前缀
            // 说明：前端发送的令牌包含Bearer前缀，需要去除后才能验证
            let token = token.trim_start_matches("Bearer ");
            // 用途：验证令牌
            // 说明：确保令牌的有效性，防止伪造
            let jwt = JWTToken::verify(&CONTEXT.config.jwt_secret, token);
            
            // 用途：检查令牌验证结果
            // 说明：如果令牌无效，返回错误信息
            match jwt {
                Ok(_jwt_token) => {
                    // 用途：查询用户的所有权限
                    // 说明：通过用户ID获取用户拥有的所有权限
                    let permissions_result = CONTEXT.rbac_permission_service.finds_all().await;
                    
                    // 用途：检查查询结果
                    // 说明：如果查询失败，返回错误信息
                    match permissions_result {
                        Ok(permissions) => {
                            // 用途：过滤出菜单类型的权限
                            // 说明：只保留permission_type为"menu"或为空的权限（兼容旧数据）
                            let menu_permissions: Vec<RbacPermissionVO> = permissions
                                .into_iter()
                                .filter(|p| {
                                    p.permission_type.as_deref() == Some("menu") 
                                    || p.permission_type.is_none() 
                                    || p.permission_type.as_deref() == Some("")
                                })
                                .map(RbacPermissionVO::from)
                                .collect();
                            
                            // 用途：构建菜单树结构
                            // 说明：将扁平的菜单列表转换为树形结构，符合Vben前端期望的格式
                            let menu_tree = build_menu_tree(menu_permissions);
                            
                            // 用途：返回响应
                            // 说明：统一响应格式，包含状态码、消息和数据
                            (StatusCode::OK, axum::Json(RespVO::from(menu_tree)))
                        }
                        Err(e) => {
                            // 用途：返回权限查询失败错误
                            // 说明：权限查询过程中出现错误
                            (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(RespVO::<Vec<MenuRouteVO>>::from_error(e.to_string())))
                        }
                    }
                }
                Err(e) => {
                    // 用途：返回令牌验证失败错误
                    // 说明：令牌无效或已过期
                    let resp_vo = RespVO::<Vec<MenuRouteVO>>::from_error(e.to_string());
                    (StatusCode::UNAUTHORIZED, axum::Json(resp_vo))
                }
            }
        }
        _ => {
            // 用途：返回令牌为空的错误
            // 说明：令牌是获取菜单的必要条件
            let resp_vo = RespVO::<Vec<MenuRouteVO>>::from_error("缺少访问令牌，请先登录".to_string());
            (StatusCode::UNAUTHORIZED, axum::Json(resp_vo))
        }
    }
}

/// 用途：构建菜单树结构
/// 说明：将扁平的菜单列表转换为树形结构，符合Vben前端期望的格式
fn build_menu_tree(permissions: Vec<RbacPermissionVO>) -> Vec<MenuRouteVO> {
    // 用途：创建菜单节点映射
    // 说明：将所有权限转换为菜单节点，并按ID建立索引
    let mut menu_map: std::collections::HashMap<String, MenuRouteVO> = std::collections::HashMap::new();
    
    // 用途：创建父子关系映射
    // 说明：记录每个菜单的子菜单列表
    let mut children_map: std::collections::HashMap<String, Vec<MenuRouteVO>> = std::collections::HashMap::new();
    
    // 用途：遍历所有权限，构建菜单节点
    // 说明：将每个权限转换为菜单节点
    for permission in permissions {
        let menu_id = permission.id.clone().unwrap_or_default();
        let path = permission.path.clone().unwrap_or_default();
        let name = permission.name.clone().unwrap_or_default();
        
        // 用途：生成组件路径
        // 说明：根据路径生成前端组件路径，Vben约定组件路径格式
        let component = if path.is_empty() {
            String::new()
        } else {
            let normalized_path = path
                .trim_start_matches("views/")
                .trim_start_matches('/')
                .trim_end_matches(".vue");
            
            format!("/{}", normalized_path)
        };
        
        // 用途：构建菜单元信息
        // 说明：包含菜单的显示信息
        let meta = MenuMetaVO {
            title: name.clone(),
            icon: permission.icon.clone(),
            authority: permission.permission.clone().map(|p| vec![p]),
            hide_in_menu: Some(permission.status.map_or(false, |s| s != 1)),
            order: permission.sort_order.clone(),
        };
        
        let menu_node = MenuRouteVO {
            name: name.clone(),
            path: path.clone(),
            component,
            children: vec![],
            meta,
        };
        
        // 用途：将菜单节点添加到映射中
        // 说明：使用ID作为键
        menu_map.insert(menu_id.clone(), menu_node);
    }
    
    // 用途：构建父子关系
    // 说明：根据路径确定菜单的层级关系
    for (_id, menu_node) in &menu_map {
        // 用途：查找父菜单
        // 说明：通过路径确定父菜单
        if let Some(parent_path) = get_parent_path(&menu_node.path) {
            // 用途：查找父菜单ID
            // 说明：通过路径查找对应的菜单ID
            for (parent_id, parent_node) in &menu_map {
                if parent_node.path == parent_path {
                    // 用途：将当前菜单添加到父菜单的子列表中
                    // 说明：建立父子关系
                    children_map
                        .entry(parent_id.clone())
                        .or_insert_with(Vec::new)
                        .push(menu_node.clone());
                }
            }
        }
    }
    
    // 用途：构建最终菜单树
    // 说明：将子菜单添加到对应的父菜单中
    for (parent_id, children) in children_map {
        if let Some(menu_node) = menu_map.get_mut(&parent_id) {
            menu_node.children = children;
        }
    }
    
    // 用途：提取顶级菜单
    // 说明：返回没有父菜单的顶级菜单
    menu_map
        .into_values()
        .filter(|menu| {
            // 用途：检查是否为顶级菜单
            // 说明：顶级菜单的路径通常只包含一个部分
            let path_parts: Vec<&str> = menu.path.split('/').filter(|s| !s.is_empty()).collect();
            path_parts.len() == 1
        })
        .collect()
}

/// 用途：获取父路径
/// 说明：从路径中提取父路径
fn get_parent_path(path: &str) -> Option<String> {
    // 用途：分割路径
    // 说明：按"/"分割路径
    let parts: Vec<&str> = path.split('/').collect();
    
    // 用途：检查路径是否包含多个部分
    // 说明：如果只有一个部分，则没有父路径
    if parts.len() <= 1 {
        return None;
    }
    
    // 用途：构建父路径
    // 说明：取除最后一部分外的所有部分
    Some(parts[..parts.len() - 1].join("/"))
}
