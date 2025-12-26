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

// 用途：导入OpenAPI相关模块
// 说明：用于自动生成API文档
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use utoipa_scalar::{Scalar, Servable};

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
    
    // 用途：初始化AI Hub管理服务
    // 说明：创建模型和供应商管理服务实例
    CONTEXT.init_ai_hub_services().await;
    
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
    
    // 用途：配置OpenAPI文档
    // 说明：设置API基本信息和服务器配置
    #[derive(OpenApi)]
    #[openapi(
        paths(
            // 认证相关接口
            rsllm::controller::rbac::rbac_user_controller::login,
            rsllm::controller::sys::sys_auth_controller::check,
            rsllm::controller::sys::sys_auth_controller::refresh_token,
            rsllm::controller::sys::sys_auth_controller::logout,
            // 字典管理接口
            rsllm::controller::sys::sys_dict_controller::page,
            rsllm::controller::sys::sys_dict_controller::add,
            rsllm::controller::sys::sys_dict_controller::update,
            rsllm::controller::sys::sys_dict_controller::remove,
            // 供应商配置接口
            rsllm::controller::ai_hub::provider_config_controller::create_provider,
            rsllm::controller::ai_hub::provider_config_controller::list_providers,
            rsllm::controller::ai_hub::provider_config_controller::update_provider,
            rsllm::controller::ai_hub::provider_config_controller::delete_provider,
            rsllm::controller::ai_hub::provider_config_controller::get_provider,
            // 聊天补全接口
            rsllm::controller::ai_hub::chat_controller::chat_completions,
            // 文本补全接口
            rsllm::controller::ai_hub::completion_controller::completions,
            // 嵌入生成接口
            rsllm::controller::ai_hub::embedding_controller::embeddings,
            // 账单管理接口
            rsllm::controller::ai_hub::bill_controller::get_billing_list,
            rsllm::controller::ai_hub::bill_controller::get_billing_statistics,
            rsllm::controller::ai_hub::bill_controller::pay_billing,
            rsllm::controller::ai_hub::bill_controller::update_billing,
            rsllm::controller::ai_hub::bill_controller::generate_test_billing,
        ),
        components(
            schemas(
                // 响应格式
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::vo::basic::RefreshTokenVO>,
                rsllm::domain::vo::response::ApiResponse<String>,
                rsllm::domain::vo::response::ApiResponse<bool>,
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::vo::response::PageWrapper<rsllm::domain::vo::basic::sys_dict::SysDictVO>>,
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::table::ai_hub::provider_config::ProviderConfig>,
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::vo::response::PageWrapper<rsllm::domain::table::ai_hub::provider_config::ProviderConfig>>,
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::vo::ai_hub::chat::ChatCompletion>,
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::vo::ai_hub::completion::CompletionResponse>,
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::vo::ai_hub::embeddings::EmbeddingsResponse>,
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::vo::ai_hub::BillingOverviewVO>,
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::vo::ai_hub::billing::BillingStatisticsVO>,
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::vo::ai_hub::billing::AiHubBillingVO>,
                // 基础类型
                rsllm::domain::vo::basic::RefreshTokenVO,
                rsllm::domain::vo::response::PageWrapper<rsllm::domain::vo::basic::sys_dict::SysDictVO>,
                rsllm::domain::vo::response::PageWrapper<rsllm::domain::table::ai_hub::provider_config::ProviderConfig>,
                // 认证相关DTO
                rsllm::domain::dto::basic::auth::SysAuthDTO,
                rsllm::domain::dto::SignInDTO,
                // 字典相关DTO
                rsllm::domain::dto::basic::sys_dict::DictPageDTO,
                rsllm::domain::dto::basic::sys_dict::DictAddDTO,
                rsllm::domain::dto::basic::sys_dict::DictEditDTO,
                // 字典相关VO
                rsllm::domain::vo::basic::sys_dict::SysDictVO,
                // 供应商相关DTO
                rsllm::service::ai_hub::CreateProviderRequest,
                rsllm::service::ai_hub::ListProvidersRequest,
                rsllm::service::ai_hub::UpdateProviderRequest,
                // 供应商配置表
                rsllm::domain::table::ai_hub::provider_config::ProviderConfig,
                // 聊天补全相关
                rsllm::service::ai_hub::DtoChatCompletionRequest,
                rsllm::domain::vo::ai_hub::chat::ChatCompletion,
                rsllm::service::ai_hub::ChatCompletionMessage,
                rsllm::domain::vo::ai_hub::chat::ChatCompletionChoice,
                rsllm::domain::vo::ai_hub::usage::Usage,
                // 文本补全相关
                rsllm::domain::dto::ai_hub::completion::CompletionRequest,
                rsllm::domain::vo::ai_hub::completion::CompletionResponse,
                rsllm::domain::vo::ai_hub::completion::CompletionChoice,
                rsllm::domain::vo::ai_hub::usage::Usage,
                // 嵌入生成相关
                rsllm::domain::dto::ai_hub::embeddings::EmbeddingsRequest,
                rsllm::domain::vo::ai_hub::embeddings::EmbeddingsResponse,
                rsllm::domain::vo::ai_hub::embeddings::Embeddings,
                rsllm::domain::vo::ai_hub::usage::EmbeddingUsage,
                // 账单相关DTO
                rsllm::domain::dto::ai_hub::billing::BillingQueryDTO,
                rsllm::service::ai_hub::BillingStatisticsQueryDTO,
                rsllm::service::ai_hub::PayBillingDTO,
                rsllm::service::ai_hub::UpdateBillingDTO,
                // 账单相关VO
                rsllm::domain::vo::ai_hub::BillingOverviewVO,
                rsllm::domain::vo::ai_hub::billing::BillingStatisticsVO,
                rsllm::domain::vo::ai_hub::billing::AiHubBillingVO,
            )
        ),
        tags(
            (name = "auth", description = "认证相关接口"),
            (name = "user", description = "用户管理接口"),
            (name = "sys_dict", description = "系统字典接口"),
            (name = "providers", description = "供应商管理接口"),
            (name = "chat", description = "聊天补全接口"),
            (name = "completion", description = "文本补全接口"),
            (name = "embeddings", description = "嵌入生成接口"),
            (name = "billing", description = "账单管理接口"),
        ),
        info(
            title = "RSLLM API",
            version = "0.1.0",
            description = "RSLLM 统一API文档 - 提供完整的AI模型管理和调用服务",
            contact(
                name = "RSLLM Team",
                email = "litc10@qq.com"
            )
        )
    )]
    struct ApiDoc;
    
    // 用途：创建OpenAPI文档路由
    // 说明：集成Swagger UI和Scalar UI用于API文档展示
    let openapi = ApiDoc::openapi();
    let swagger_ui = SwaggerUi::new("/swagger-ui")
        .url("/api-docs/openapi.json", openapi.clone());
    let scalar_ui = Scalar::with_url("/scalar", openapi);
    
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
    let bind_address = CONTEXT.config.server_url
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let listener = tokio::net::TcpListener::bind(bind_address).await?;
    
    // 用途：构建完整的应用路由
    // 说明：使用新的router模块，合并所有路由和中间件
    let app = app_router
        .merge(swagger_ui)   // 用途：添加Swagger UI路由
                                // 说明：提供API文档界面
        .merge(scalar_ui)     // 用途：添加Scalar UI路由
                                // 说明：提供另一种API文档界面
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