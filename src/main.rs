// 用途：导入全局上下文
// 说明：用于访问应用程序配置和服务实例
use rsllm::context::CONTEXT;

// 用途：导入各种控制器
// 说明：用于处理HTTP请求
use rsllm::controller::{
    img_controller,                    // 图片控制器
    rbac_permission_controller,        // 权限控制器
    rbac_role_controller,              // 角色控制器
    rbac_user_controller,              // 用户控制器
    sys_auth_controller,               // 系统认证控制器
    sys_dict_controller,               // 系统字典控制器
};

// 用途：导入表结构相关模块
// 说明：用于同步数据库表结构
use rsllm::domain::table;

// 用途：导入响应VO
// 说明：用于统一HTTP响应格式
use rsllm::domain::vo::RespVO;

// 用途：导入axum的路由结构
// 说明：用于构建HTTP路由
use axum::Router;

// 用途：导入默认请求体限制
// 说明：用于设置请求体大小限制
use axum::extract::DefaultBodyLimit;

// 用途：导入HTTP方法
// 说明：用于定义路由的HTTP方法
use axum::routing::{get, post};

// 用途：导入rbs的value宏
// 说明：用于将配置转换为日志格式
use rbs::value;

// 用途：导入CORS中间件
// 说明：用于处理跨域请求
use tower_http::cors::{Any, CorsLayer};

// 用途：导入请求体限制中间件
// 说明：用于设置请求体大小限制
use tower_http::limit::RequestBodyLimitLayer;

// 用途：导入静态文件服务
// 说明：用于提供静态文件访问
use tower_http::services::{ServeDir, ServeFile};

/// 用途：主函数入口
/// 说明：应用程序的启动点，负责初始化和启动服务器
#[tokio::main] // 用途：tokio异步运行时宏
              // 说明：使main函数支持异步操作
async fn main() -> std::io::Result<()> {
    // 用途：初始化日志系统
    // 说明：配置和启动日志服务，便于调试和监控
    rsllm::config::log::init_log();
    
    // 用途：检查是否为调试模式
    // 说明：根据运行模式输出不同的日志信息
    if CONTEXT.config.debug() {
        // 用途：输出配置信息
        // 说明：调试模式下方便查看当前配置
        log::info!("[rsllm] {}", value!(&CONTEXT.config));
        // 用途：输出调试模式启动日志
        // 说明：告知用户当前运行在调试模式
        log::info!(
            "[rsllm] --------------------- Start On Debug Mode ------------------------------"
        );
    } else {
        // 用途：输出发布模式启动日志
        // 说明：告知用户当前运行在发布模式
        log::info!(
            "[rsllm] --------------------- Start On Release Mode ----------------------------"
        );
    }
    
    // 用途：初始化数据库连接池
    // 说明：建立与数据库的连接，为后续操作做准备
    CONTEXT.init_database().await;
    
    // 用途：同步数据库表结构
    // 说明：确保数据库表结构与代码定义一致
    table::sync_tables(&CONTEXT.rb).await;
    
    // 用途：同步表数据
    // 说明：初始化或更新表中的基础数据
    table::sync_tables_data(&CONTEXT.rb).await;
    
    // 用途：输出服务器地址
    // 说明：告知用户服务器的访问地址
    log::info!(
        "Serve: http://{}",
        CONTEXT.config.server_url.replace("0.0.0.0", "127.0.0.1")
    );
    
    // 用途：输出分隔线日志
    // 说明：美化日志输出，便于区分不同阶段的日志
    log::info!(
        "[rsllm] ------------------------------------------------------------------------"
    );
    
    // 用途：配置无认证路由
    // 说明：定义不需要身份验证的API接口
    let no_auth_router = Router::new()
        // 用途：定义根路径路由
        // 说明：提供简单的健康检查接口
        .route(
            "/admin/",
            get(|| async { RespVO::from("hello".to_string()) }),
        )
        // 用途：定义登录路由
        // 说明：处理用户登录请求
        .route("/admin/sys_login", post(rbac_user_controller::login))
        // 用途：定义认证检查路由
        // 说明：检查用户认证状态
        .route("/admin/auth/check", post(sys_auth_controller::check))
        // 用途：定义验证码路由
        // 说明：生成和返回验证码图片
        .route("/admin/captcha", get(img_controller::captcha));
    
    // 用途：配置需认证路由
    // 说明：定义需要身份验证的API接口
    let auth_router = Router::new()
        // 用途：定义用户信息路由
        // 说明：获取当前登录用户信息
        .route("/admin/sys_user_info", post(rbac_user_controller::info))
        // 用途：定义用户详情路由
        // 说明：获取指定用户的详细信息
        .route("/admin/sys_user_detail", post(rbac_user_controller::detail))
        // 用途：定义权限更新路由
        // 说明：更新权限信息
        .route(
            "/admin/sys_permission_update",
            post(rbac_permission_controller::update),
        )
        // 用途：定义权限删除路由
        // 说明：删除权限
        .route(
            "/admin/sys_permission_remove",
            post(rbac_permission_controller::remove),
        )
        // 用途：定义权限添加路由
        // 说明：添加新权限
        .route(
            "/admin/sys_permission_add",
            post(rbac_permission_controller::add),
        )
        // 用途：定义权限分页查询路由
        // 说明：分页查询权限列表
        .route(
            "/admin/sys_permission_page",
            post(rbac_permission_controller::page),
        )
        // 用途：定义权限层级查询路由
        // 说明：查询权限的层级结构
        .route(
            "/admin/sys_permission_layer_top",
            post(rbac_permission_controller::layer_top),
        )
        // 用途：定义用户添加路由
        // 说明：添加新用户
        .route("/admin/sys_user_add", post(rbac_user_controller::add))
        // 用途：定义用户分页查询路由
        // 说明：分页查询用户列表
        .route("/admin/sys_user_page", post(rbac_user_controller::page))
        // 用途：定义用户删除路由
        // 说明：删除用户
        .route("/admin/sys_user_remove", post(rbac_user_controller::remove))
        // 用途：定义用户更新路由
        // 说明：更新用户信息
        .route("/admin/sys_user_update", post(rbac_user_controller::update))
        // 用途：定义角色添加路由
        // 说明：添加新角色
        .route("/admin/sys_role_add", post(rbac_role_controller::add))
        // 用途：定义角色更新路由
        // 说明：更新角色信息
        .route("/admin/sys_role_update", post(rbac_role_controller::update))
        // 用途：定义角色删除路由
        // 说明：删除角色
        .route("/admin/sys_role_delete", post(rbac_role_controller::remove))
        // 用途：定义角色分页查询路由
        // 说明：分页查询角色列表
        .route("/admin/sys_role_page", post(rbac_role_controller::page))
        // 用途：定义角色层级查询路由
        // 说明：查询角色的层级结构
        .route(
            "/admin/sys_role_layer_top",
            post(rbac_role_controller::layer_top),
        )
        // 用途：定义字典添加路由
        // 说明：添加新字典
        .route("/admin/sys_dict_add", post(sys_dict_controller::add))
        // 用途：定义字典更新路由
        // 说明：更新字典信息
        .route("/admin/sys_dict_update", post(sys_dict_controller::update))
        // 用途：定义字典删除路由
        // 说明：删除字典
        .route("/admin/sys_dict_remove", post(sys_dict_controller::remove))
        // 用途：定义字典分页查询路由
        // 说明：分页查询字典列表
        .route("/admin/sys_dict_page", post(sys_dict_controller::page))
        // 用途：添加认证中间件
        // 说明：保护需认证的路由，确保只有已登录用户可以访问
        .layer(axum::middleware::from_fn(
            rsllm::middleware::auth_axum::auth,
        ));
    
    // 用途：配置静态资源路由
    // 说明：提供静态文件访问服务，如前端页面
    let resource_router = Router::new().fallback_service(
        // 用途：定义静态目录服务
        // 说明：提供dist目录下的静态文件访问
        ServeDir::new("dist/").not_found_service(
            // 用途：定义404页面
            // 说明：当找不到请求的静态文件时，返回index.html，支持SPA应用
            ServeFile::new("dist/index.html"),
        ),
    );
    
    // 用途：配置CORS
    // 说明：允许跨域请求，便于前端与后端分离部署
    let cors = CorsLayer::new()
        .allow_origin(Any) // 用途：允许任何来源
                          // 说明：开发环境下方便调试，生产环境应限制来源
        .allow_methods(Any) // 用途：允许任何HTTP方法
                           // 说明：支持所有HTTP请求方法
        .allow_headers(Any); // 用途：允许任何HTTP头
                           // 说明：支持所有自定义头信息
    
    // 用途：创建TCP监听器
    // 说明：监听指定的服务器地址和端口
    let listener = tokio::net::TcpListener::bind(&CONTEXT.config.server_url).await?;
    
    // 用途：构建完整的应用路由
    // 说明：合并所有路由和中间件，构建最终的应用
    let app = Router::new()
        .merge(resource_router) // 用途：合并静态资源路由
                                // 说明：添加静态文件服务
        .merge(no_auth_router)  // 用途：合并无认证路由
                                // 说明：添加不需要认证的API接口
        .merge(auth_router)     // 用途：合并需认证路由
                                // 说明：添加需要认证的API接口
        .layer(cors)            // 用途：添加CORS中间件
                                // 说明：处理跨域请求
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024)) // 用途：设置默认请求体大小限制
                                                         // 说明：限制单个请求的大小，防止恶意请求
        .layer(RequestBodyLimitLayer::new(50 * 1024 * 1024)); // 用途：设置请求体大小限制
                                                             // 说明：双重保障，确保请求体不超过50MB
    
    // 用途：启动服务器
    // 说明：开始接受和处理HTTP请求
    axum::serve(listener, app).await
}
