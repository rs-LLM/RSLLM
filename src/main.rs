// 用途：导入全局上下文
// 说明：用于访问应用程序配置和服务实例
use rsllm::context::CONTEXT;

// 用途：导入供应商配置服务trait
// 说明：用于调用供应商配置服务的方法
use rsllm::service::ProviderConfigService;

// 用途：导入路由模块
// 说明：用于构建HTTP路由
use rsllm::router::create_app_router;
use rsllm::router::openai_router::create_openai_router;

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
use tower_http::cors::{AllowOrigin, CorsLayer};

// 用途：导入请求体限制中间件
// 说明：用于设置请求体大小限制
use axum::Router;
use axum::http::{
    Method,
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
};
use axum::routing::get;
use tower_http::limit::RequestBodyLimitLayer;

// 用途：导入OpenAPI相关模块
// 说明：用于自动生成API文档
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};
use utoipa_swagger_ui::SwaggerUi;

// 用途：导入信号处理模块
// 说明：用于捕获 Ctrl+C 和 SIGTERM 信号，实现优雅关闭
use std::sync::Arc;
use tokio::signal;
use tokio::time::{Duration as TokioDuration, interval};

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

    // 用途：初始化供应商缓存
    // 说明：解密并缓存所有供应商的API密钥，提高性能
    log::info!("[rsllm] Initializing provider cache...");
    let provider_config_service = CONTEXT.provider_config_service.read().await;
    if let Some(service) = provider_config_service.as_ref() {
        if let Err(e) = service.initialize_provider_cache().await {
            log::error!("[rsllm] Failed to initialize provider cache: {}", e);
        } else {
            log::info!("[rsllm] Provider cache initialized successfully");
        }
    } else {
        log::warn!("[rsllm] Provider config service not initialized");
    }

    // 用途：输出服务器地址
    // 说明：告知用户服务器的访问地址
    log::info!(
        "Serve: {}",
        CONTEXT.config.server_url.replace("0.0.0.0", "127.0.0.1")
    );

    // 用途：输出分隔线日志
    // 说明：美化日志输出，便于区分不同阶段的日志
    log::info!("[rsllm] ------------------------------------------------------------------------");

    let shared_context = Arc::new(CONTEXT.clone());

    {
        let maintenance_context = shared_context.clone();
        tokio::spawn(async move {
            let mut ticker = interval(TokioDuration::from_secs(300));
            loop {
                ticker.tick().await;
                if let Err(e) =
                    rsllm::controller::ai_hub::subscription_controller::run_subscription_maintenance(
                        &maintenance_context,
                    )
                    .await
                {
                    log::error!("[rsllm] Subscription maintenance failed: {}", e);
                }
            }
        });
    }

    // 用途：创建完整的应用路由
    // 说明：使用新的router模块组织路由
    // 注意：由于ServiceContext没有实现Clone，我们直接使用CONTEXT
    // 这里需要修改router设计，使用引用传递
    let app_router = create_app_router(shared_context.clone());

    // 用途：启动本地Codex OAuth回调监听（兼容CLIProxy预注册回调）
    // 说明：默认监听 http://localhost:1455/auth/callback
    if let Ok(callback_url) = url::Url::parse(CONTEXT.config.codex_oauth_callback_url.as_str()) {
        if matches!(
            callback_url.host_str(),
            Some("localhost") | Some("127.0.0.1")
        ) {
            if let Some(port) = callback_url.port_or_known_default() {
                let callback_bind = format!("127.0.0.1:{}", port);
                match tokio::net::TcpListener::bind(callback_bind.as_str()).await {
                    Ok(callback_listener) => {
                        let callback_state = std::sync::Arc::new(CONTEXT.clone());
                        let callback_router = Router::new()
                            .route(
                                "/auth/callback",
                                get(rsllm::controller::ai_hub::provider_config_controller::provider_oauth_public_callback),
                            )
                            .with_state(callback_state);

                        tokio::spawn(async move {
                            if let Err(e) =
                                axum::serve(callback_listener, callback_router.into_make_service())
                                    .await
                            {
                                log::error!("[rsllm] OAuth callback listener stopped: {}", e);
                            }
                        });
                        log::info!(
                            "[rsllm] OAuth callback listener started on {}",
                            callback_bind
                        );
                    }
                    Err(e) => {
                        log::warn!(
                            "[rsllm] Failed to bind OAuth callback listener on {}: {}",
                            callback_bind,
                            e
                        );
                    }
                }
            }
        }
    }

    // 用途：配置OpenAPI文档
    // 说明：设置API基本信息和服务器配置
    #[derive(OpenApi)]
    #[openapi(
        paths(
            // 认证相关接口
            rsllm::controller::rbac::rbac_user_controller::login,
            rsllm::controller::sys::sys_oauth_controller::generate_oauth_state,
            rsllm::controller::sys::sys_oauth_controller::handle_oauth_callback,
            rsllm::controller::sys::sys_auth_controller::login,
            rsllm::controller::sys::sys_auth_controller::check,
            rsllm::controller::sys::sys_auth_controller::refresh_token,
            rsllm::controller::sys::sys_auth_controller::logout,
            // 字典管理接口
            rsllm::controller::sys::sys_dict_controller::page,
            rsllm::controller::sys::sys_dict_controller::add,
            rsllm::controller::sys::sys_dict_controller::update,
            rsllm::controller::sys::sys_dict_controller::remove,
            // 权限管理接口
            rsllm::controller::rbac::rbac_permission_controller::layer_top,
            rsllm::controller::rbac::rbac_permission_controller::page,
            rsllm::controller::rbac::rbac_permission_controller::add,
            rsllm::controller::rbac::rbac_permission_controller::update,
            rsllm::controller::rbac::rbac_permission_controller::remove,
            rsllm::controller::rbac::rbac_permission_controller::query_by_type,
            rsllm::controller::rbac::rbac_permission_controller::update_status,
            rsllm::controller::rbac::rbac_permission_controller::query_audit_logs,
            // 供应商配置接口
            rsllm::controller::ai_hub::provider_config_controller::create_provider,
            rsllm::controller::ai_hub::provider_config_controller::list_providers,
            rsllm::controller::ai_hub::provider_config_controller::update_provider,
            rsllm::controller::ai_hub::provider_config_controller::delete_provider,
            rsllm::controller::ai_hub::provider_config_controller::get_provider,
            // 聊天补全接口
            rsllm::controller::ai_hub::chat_controller::chat_completions,
            rsllm::router::openai_router::list_v1_models,
            rsllm::router::openai_router::create_v1_completions_alias,
            rsllm::router::openai_router::create_v1_messages_alias,
            rsllm::router::openai_router::create_v1_chat_completions,
            rsllm::router::openai_router::create_v1_embeddings,
            rsllm::router::openai_router::create_v1_response,
            rsllm::router::openai_router::create_v1_compact_response_alias,
            rsllm::router::openai_router::get_v1_dashboard_billing_subscription,
            rsllm::router::openai_router::get_v1_dashboard_billing_usage,
            rsllm::controller::sys::sys_service_controller::get_public_user_agreement,
            rsllm::controller::sys::sys_service_controller::get_public_privacy_policy,
            rsllm::router::ai_hub_router::create_completions_alias,
            rsllm::router::ai_hub_router::create_messages_alias,
            rsllm::router::ai_hub_router::create_compact_response_alias,
            // OpenAI/Gemini beta 模型列表兼容接口
            rsllm::router::openai_router::list_v1beta_models,
            rsllm::router::openai_router::list_v1beta_openai_models,
            // 嵌入生成接口
            rsllm::controller::ai_hub::embedding_controller::embeddings,
            rsllm::controller::ai_hub::responses_controller::create_response,
            // 交易管理接口
            rsllm::controller::ai_hub::transaction_controller::list,
            rsllm::controller::ai_hub::transaction_controller::summary,
            // 余额管理接口
            rsllm::controller::ai_hub::balance_controller::get_balance,
            rsllm::controller::ai_hub::balance_controller::recharge,
            rsllm::controller::ai_hub::balance_controller::deduct,
            rsllm::controller::ai_hub::balance_controller::set_balance,
            rsllm::controller::ai_hub::checkin_controller::get_checkin_status,
            rsllm::controller::ai_hub::checkin_controller::do_checkin,
            rsllm::controller::sys::twofa_controller::get_twofa_status,
            rsllm::controller::sys::twofa_controller::setup_twofa,
            rsllm::controller::sys::twofa_controller::enable_twofa,
            rsllm::controller::sys::twofa_controller::disable_twofa,
            rsllm::controller::sys::twofa_controller::regenerate_backup_codes,
            rsllm::controller::sys::twofa_controller::verify_login_twofa,
            rsllm::controller::sys::twofa_controller::admin_reset_twofa,
            rsllm::controller::sys::twofa_controller::admin_twofa_stats,
        ),
        components(
            schemas(
                // 响应格式
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::vo::basic::RefreshTokenVO>,
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::vo::basic::LoginVO>,
                rsllm::domain::vo::response::ApiResponse<serde_json::Value>,
                rsllm::domain::vo::response::ApiResponse<String>,
                rsllm::domain::vo::response::ApiResponse<bool>,
                rsllm::domain::vo::response::ApiResponse<u64>,
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::vo::response::PageWrapper<rsllm::domain::vo::basic::sys_dict::SysDictVO>>,
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::table::ai_hub::provider_config::ProviderConfig>,
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::vo::response::PageWrapper<rsllm::domain::table::ai_hub::provider_config::ProviderConfig>>,
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::vo::ai_hub::chat::ChatCompletion>,
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::vo::ai_hub::embeddings::EmbeddingsResponse>,
                // 权限相关响应
                rsllm::domain::vo::response::ApiResponse<Vec<rsllm::domain::vo::rbac::RbacPermissionVO>>,
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::vo::response::PageWrapper<rsllm::domain::vo::rbac::RbacPermissionVO>>,
                rsllm::domain::vo::response::ApiResponse<Vec<rsllm::domain::vo::rbac::RbacPermissionAuditLogVO>>,
                // 交易相关响应
                rsllm::domain::vo::response::ApiResponse<rsllm::controller::ai_hub::transaction_controller::TransactionListResponse>,
                rsllm::domain::vo::response::ApiResponse<rsllm::controller::ai_hub::transaction_controller::TransactionSummaryResponse>,
                // 余额相关响应
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::vo::ai_hub::BalanceVO>,
                rsllm::domain::vo::response::ApiResponse<rsllm::controller::ai_hub::balance_controller::RechargeResponse>,
                rsllm::domain::vo::response::ApiResponse<rsllm::controller::ai_hub::balance_controller::DeductResponse>,
                rsllm::domain::vo::response::ApiResponse<rsllm::controller::ai_hub::checkin_controller::CheckinStatusData>,
                rsllm::domain::vo::response::ApiResponse<rsllm::controller::ai_hub::checkin_controller::DoCheckinData>,
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::dto::basic::twofa::TwoFaStatusResponse>,
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::dto::basic::twofa::TwoFaSetupResponse>,
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::dto::basic::twofa::TwoFaBackupCodesResponse>,
                rsllm::domain::vo::response::ApiResponse<rsllm::controller::sys::twofa_controller::NeedTwoFaResponse>,
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::dto::basic::twofa::TwoFaAdminStatsResponse>,
                rsllm::domain::vo::response::ApiResponse<rsllm::domain::vo::SignInVO>,
                // 基础类型
                rsllm::domain::vo::basic::RefreshTokenVO,
                rsllm::domain::vo::response::PageWrapper<rsllm::domain::vo::basic::sys_dict::SysDictVO>,
                rsllm::domain::vo::response::PageWrapper<rsllm::domain::table::ai_hub::provider_config::ProviderConfig>,
                rsllm::domain::vo::response::PageWrapper<rsllm::domain::vo::rbac::RbacPermissionVO>,
                // 认证相关DTO
                rsllm::domain::dto::basic::auth::SysAuthDTO,
                rsllm::domain::dto::SignInDTO,
                // 字典相关DTO
                rsllm::domain::dto::basic::sys_dict::DictPageDTO,
                rsllm::domain::dto::basic::sys_dict::DictAddDTO,
                rsllm::domain::dto::basic::sys_dict::DictEditDTO,
                // 字典相关VO
                rsllm::domain::vo::basic::sys_dict::SysDictVO,
                // 权限相关DTO
                rsllm::domain::dto::rbac::PermissionPageDTO,
                rsllm::domain::dto::rbac::PermissionAddDTO,
                rsllm::domain::dto::rbac::ResEditDTO,
                rsllm::domain::dto::IdDTO,
                // 权限相关VO
                rsllm::domain::vo::rbac::RbacPermissionVO,
                rsllm::domain::vo::rbac::RbacPermissionAuditLogVO,
                // 供应商相关DTO
                rsllm::service::ai_hub::CreateProviderRequest,
                rsllm::service::ai_hub::ListProvidersRequest,
                rsllm::service::ai_hub::UpdateProviderRequest,
                // 供应商配置表
                rsllm::domain::table::ai_hub::provider_config::ProviderConfig,
                // 聊天补全相关
                rsllm::service::ai_hub::DtoChatCompletionRequest,
                rsllm::domain::dto::ai_hub::completion::CompletionAliasRequest,
                rsllm::domain::dto::ai_hub::messages::ClaudeMessagesRequest,
                rsllm::domain::vo::ai_hub::chat::ChatCompletion,
                rsllm::domain::vo::ai_hub::messages::ClaudeMessageResponse,
                rsllm::domain::vo::ai_hub::messages::ClaudeErrorResponse,
                rsllm::domain::vo::ai_hub::messages::ClaudeErrorBody,
                rsllm::domain::vo::ai_hub::messages::ClaudeContentBlock,
                rsllm::domain::vo::ai_hub::messages::ClaudeUsage,
                rsllm::domain::vo::ai_hub::messages::ClaudeMessageStartEvent,
                rsllm::domain::vo::ai_hub::messages::ClaudeContentBlockStartEvent,
                rsllm::domain::vo::ai_hub::messages::ClaudeContentBlockDeltaEvent,
                rsllm::domain::vo::ai_hub::messages::ClaudeContentBlockStopEvent,
                rsllm::domain::vo::ai_hub::messages::ClaudeMessageDeltaEvent,
                rsllm::domain::vo::ai_hub::messages::ClaudeMessageStopEvent,
                rsllm::domain::vo::ai_hub::messages::ClaudeErrorEvent,
                rsllm::service::ai_hub::ChatCompletionMessage,
                rsllm::domain::vo::ai_hub::chat::ChatCompletionChoice,
                rsllm::domain::vo::ai_hub::usage::Usage,
                rsllm::controller::ai_hub::model_controller::OpenAIModelsListResponse,
                rsllm::controller::ai_hub::model_controller::OpenAIModelInfo,
                rsllm::controller::ai_hub::model_controller::OpenAIProviderInfo,
                // 嵌入生成相关
                rsllm::domain::dto::ai_hub::embeddings::EmbeddingsRequest,
                rsllm::domain::vo::ai_hub::embeddings::EmbeddingsResponse,
                rsllm::domain::vo::ai_hub::embeddings::Embeddings,
                rsllm::domain::vo::ai_hub::usage::EmbeddingUsage,
                // 交易相关DTO和VO
                rsllm::domain::dto::ai_hub::QueryTransactionDTO,
                rsllm::domain::vo::ai_hub::TransactionVO,
                rsllm::controller::ai_hub::transaction_controller::TransactionListResponse,
                // 余额相关DTO和VO
                rsllm::domain::dto::ai_hub::RechargeDTO,
                rsllm::domain::dto::ai_hub::DeductDTO,
                rsllm::domain::dto::ai_hub::SetBalanceDTO,
                rsllm::domain::vo::ai_hub::BalanceVO,
                rsllm::controller::ai_hub::checkin_controller::CheckinStatusQuery,
                rsllm::controller::ai_hub::checkin_controller::CheckinRecordVO,
                rsllm::controller::ai_hub::checkin_controller::CheckinStatsVO,
                rsllm::controller::ai_hub::checkin_controller::CheckinStatusData,
                rsllm::controller::ai_hub::checkin_controller::DoCheckinData,
                rsllm::domain::dto::ai_hub::dashboard_billing::OpenAISubscriptionResponse,
                rsllm::domain::dto::ai_hub::dashboard_billing::OpenAIUsageResponse,
                rsllm::controller::sys::sys_service_controller::PublicLegalResponse,
                rsllm::domain::dto::basic::twofa::TwoFaStatusResponse,
                rsllm::domain::dto::basic::twofa::TwoFaSetupResponse,
                rsllm::domain::dto::basic::twofa::TwoFaEnableRequest,
                rsllm::domain::dto::basic::twofa::TwoFaDisableRequest,
                rsllm::domain::dto::basic::twofa::TwoFaBackupCodesResponse,
                rsllm::domain::dto::basic::twofa::TwoFaVerifyLoginRequest,
                rsllm::domain::dto::basic::twofa::TwoFaAdminStatsResponse,
                rsllm::controller::sys::twofa_controller::NeedTwoFaResponse,
            )
        ),
        tags(
            (name = "auth", description = "认证相关接口"),
            (name = "user", description = "用户管理接口"),
            (name = "sys_dict", description = "系统字典接口"),
            (name = "permission", description = "权限管理接口"),
            (name = "providers", description = "供应商管理接口"),
            (name = "chat", description = "聊天补全接口"),
            (name = "embedding", description = "嵌入生成接口"),
            (name = "transaction", description = "交易管理接口"),
            (name = "balance", description = "余额管理接口"),
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
    let swagger_ui = SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi.clone());
    let scalar_ui = Scalar::with_url("/scalar", openapi);

    // 用途：配置CORS
    // 说明：允许跨域请求，便于前端与后端分离部署
    let allow_origin = AllowOrigin::predicate(|origin: &axum::http::HeaderValue, _| {
        matches!(
            origin.to_str().ok(),
            Some("http://127.0.0.1:5777") | Some("http://localhost:5777")
        )
    });

    let cors = CorsLayer::new()
        .allow_origin(allow_origin) // 用途：允许本地前端来源
        // 说明：凭证请求不能与通配符来源同时使用，需显式列出开发源
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        // 说明：覆盖当前前后端实际会用到的 HTTP 方法
        .allow_headers([AUTHORIZATION, CONTENT_TYPE, ACCEPT])
        // 说明：凭证请求下不能继续使用通配符请求头
        .allow_credentials(true); // 用途：允许浏览器在跨域请求中携带凭证

    let cors = Arc::new(cors);

    // 用途：启动 OpenAI 兼容 API 独立端口（可选）
    // 说明：当启用时，对外暴露 /v1/* 风格接口，复用相同鉴权与配额中间件
    if CONTEXT.config.openai_enabled {
        if let Some(openai_port) = CONTEXT.config.openai_port {
            let openai_bind = CONTEXT.config.openai_bind.clone();
            let openai_listener_address = format!("{}:{}", openai_bind, openai_port);

            match tokio::net::TcpListener::bind(openai_listener_address.as_str()).await {
                Ok(openai_listener) => {
                    let openai_router = create_openai_router(shared_context.clone())
                        .layer(cors.as_ref().clone())
                        .layer(DefaultBodyLimit::max(50 * 1024 * 1024))
                        .layer(RequestBodyLimitLayer::new(50 * 1024 * 1024));

                    tokio::spawn(async move {
                        if let Err(e) =
                            axum::serve(openai_listener, openai_router.into_make_service()).await
                        {
                            log::error!("[rsllm] OpenAI listener stopped: {}", e);
                        }
                    });

                    log::info!(
                        "[rsllm] OpenAI compatible API listener started on http://{}{}",
                        openai_listener_address,
                        "/v1"
                    );
                }
                Err(e) => {
                    log::warn!(
                        "[rsllm] Failed to bind OpenAI listener on {}: {}",
                        openai_listener_address,
                        e
                    );
                }
            }
        } else {
            log::warn!(
                "[rsllm] openai_enabled=true but openai_port is not set; skip OpenAI listener"
            );
        }
    }

    // 用途：创建TCP监听器
    // 说明：监听指定的服务器地址和端口
    let bind_address = CONTEXT
        .config
        .server_url
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let listener = tokio::net::TcpListener::bind(bind_address).await?;

    // 用途：构建完整的应用路由
    // 说明：使用新的router模块，合并所有路由和中间件
    let app = app_router
        .merge(swagger_ui) // 用途：添加Swagger UI路由
        // 说明：提供API文档界面
        .merge(scalar_ui) // 用途：添加Scalar UI路由
        // 说明：提供另一种API文档界面
        .layer(cors.as_ref().clone()) // 用途：添加CORS中间件
        // 说明：处理跨域请求
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024)) // 用途：设置默认请求体大小限制
        // 说明：限制单个请求的大小，防止恶意请求
        .layer(RequestBodyLimitLayer::new(50 * 1024 * 1024)); // 用途：设置请求体大小限制
    // 说明：双重保障，确保请求体不超过50MB

    // 用途：启动服务器
    // 说明：开始接受和处理HTTP请求，支持优雅关闭
    // 创建优雅关闭信号
    let shutdown_signal = async {
        #[cfg(unix)]
        {
            // 在 Unix 系统上，同时监听 Ctrl+C 和 SIGTERM 信号
            use tokio::signal::unix::{SignalKind, signal};
            let mut sigterm =
                signal(SignalKind::terminate()).expect("Failed to setup SIGTERM handler");

            tokio::select! {
                _ = signal::ctrl_c() => {
                    log::info!("[rsllm] Received Ctrl+C signal, initiating graceful shutdown...");
                }
                _ = sigterm.recv() => {
                    log::info!("[rsllm] Received SIGTERM signal, initiating graceful shutdown...");
                }
            }
        }

        #[cfg(not(unix))]
        {
            // 在非 Unix 系统上，只监听 Ctrl+C 信号
            signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
            log::info!("[rsllm] Received Ctrl+C signal, initiating graceful shutdown...");
        }
    };

    // 使用优雅关闭启动服务器
    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    // 用途：优雅关闭日志
    // 说明：告知用户服务器已正常关闭，数据库连接已自动释放
    log::info!("[rsllm] Server shutdown completed gracefully. Database connections released.");

    Ok(())
}
