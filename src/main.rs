// 用途：导入全局上下文
// 说明：用于访问应用程序配置和服务实例
use rsllm::context::CONTEXT;

// 用途：导入路由模块
// 说明：用于构建HTTP路由
use rsllm::router::create_app_router;

// 用途：导入表结构相关模块
// 说明：用于同步数据库表结构
use rsllm::domain::table;


// 用途：导入默认请求体限制
// 说明：用于设置请求体大小限制
use axum::extract::DefaultBodyLimit;

// 用途：导入rbs的value宏
// 说明：用于将配置转换为日志格式
use rbs::value;

// 用途：导入CORS中间件
// 说明：用于处理跨域请求
use tower_http::cors::{Any, CorsLayer};

// 用途：导入请求体限制中间件
// 说明：用于设置请求体大小限制
use tower_http::limit::RequestBodyLimitLayer;

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
    
    // 用途：初始化供应商注册表
    // 说明：从数据库加载供应商配置并初始化供应商注册表
    CONTEXT.init_providers().await;
    
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
    
    // 用途：创建完整的应用路由
    // 说明：使用新的router模块组织路由
    // 注意：由于ServiceContext没有实现Clone，我们直接使用CONTEXT
    // 这里需要修改router设计，使用引用传递
    let app_router = create_app_router(std::sync::Arc::new(CONTEXT.clone()));
    
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
    // 说明：使用新的router模块，合并所有路由和中间件
    let app = app_router
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