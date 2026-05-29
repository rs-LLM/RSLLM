// 用途：初始化向导控制器
// 说明：处理系统初始化过程中的各种请求
use axum::{Json, extract::State, http::StatusCode};
use log::{error, info, warn};
use rand::Rng;
use rbatis::RBatis;
use rbatis::rbdc::DateTime;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

use crate::context::ServiceContext;
use crate::domain::table::LoginCheck;
use crate::domain::table::invitation_code::InvitationCode;
use crate::domain::table::key_value_config::KeyValueConfig;
use crate::domain::table::rbac::RbacUserRole;
use crate::domain::table::registration_review::RegistrationReview;
use crate::domain::table::sys_user::SysUser;
use crate::domain::vo::response::ApiResponse;
use crate::service::InitTransactionManager;
use crate::service::sys::{RegisterPolicy, RegisterPolicyService};
use crate::util::password_encoder::PasswordEncoder;
use crate::util::user_register_validation::UserRegisterValidator;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

/// 用途：初始化状态枚举
/// 说明：表示系统初始化的不同状态
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum InitStatus {
    /// 未初始化
    #[serde(rename = "not_initialized")]
    NotInitialized,
    /// 初始化中
    #[serde(rename = "initializing")]
    Initializing {
        /// 当前步骤
        current_step: String,
        /// 进度百分比 (0-100)
        progress: u8,
    },
    /// 已初始化
    #[serde(rename = "initialized")]
    Initialized {
        /// 初始化时间
        initialized_at: String,
    },
    /// 初始化失败
    #[serde(rename = "failed")]
    Failed {
        /// 失败原因
        reason: String,
        /// 失败步骤
        failed_step: String,
    },
}

/// 用途：初始化步骤枚举
/// 说明：定义初始化流程的各个步骤
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
pub enum InitStep {
    #[serde(rename = "database")]
    Database,
    #[serde(rename = "jwt")]
    Jwt,
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "config")]
    Config,
}

/// 用途：初始化状态检查响应
/// 说明：返回系统是否已完成初始化
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct InitStatusResponse {
    pub status: InitStatus, // 用途：初始化状态
    pub initialized: bool,  // 用途：是否已完成初始化（向后兼容）
    pub message: String,    // 用途：状态描述
}

/// 用途：数据库连接验证请求
/// 说明：包含数据库连接所需的参数
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct VerifyDbRequest {
    pub db_type: String,          // 用途：数据库类型
    pub host: Option<String>,     // 用途：数据库主机（MySQL/PostgreSQL必需）
    pub port: Option<u16>,        // 用途：数据库端口（MySQL/PostgreSQL必需）
    pub database: String,         // 用途：数据库名称
    pub username: Option<String>, // 用途：数据库用户名（MySQL/PostgreSQL必需）
    pub password: Option<String>, // 用途：数据库密码（MySQL/PostgreSQL必需）
    pub timeout: Option<u64>,     // 用途：连接超时时间（秒）
    pub ssl_mode: Option<String>, // 用途：SSL模式
}

/// 用途：数据库连接验证响应
/// 说明：返回数据库连接验证的详细结果
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct VerifyDbResponse {
    pub success: bool,                           // 用途：是否连接成功
    pub message: String,                         // 用途：状态描述
    pub connection_info: Option<ConnectionInfo>, // 用途：连接信息
    pub error_details: Option<ErrorDetails>,     // 用途：错误详情
}

/// 用途：数据库连接信息
/// 说明：包含数据库连接的详细信息
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ConnectionInfo {
    pub db_type: String,                 // 用途：数据库类型
    pub host: Option<String>,            // 用途：数据库主机（SQLite为None）
    pub port: Option<u16>,               // 用途：数据库端口（SQLite为None）
    pub database: String,                // 用途：数据库名称
    pub version: Option<String>,         // 用途：数据库版本
    pub connection_time_ms: Option<u64>, // 用途：连接耗时（毫秒）
}

/// 用途：错误详情
/// 说明：包含连接失败的详细信息
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ErrorDetails {
    pub error_code: Option<String>, // 用途：错误代码
    pub error_message: String,      // 用途：错误消息
    pub suggestion: Option<String>, // 用途：解决建议
}

/// 用途：JWT密钥生成响应
/// 说明：返回生成的JWT密钥及其详细信息
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct JwtSecretResponse {
    pub secret: String,     // 用途：生成的JWT密钥
    pub algorithm: String,  // 用途：使用的加密算法
    pub key_length: usize,  // 用途：密钥长度（字节）
    pub strength: String,   // 用途：密钥强度评估
    pub expires_in: usize,  // 用途：默认过期时间（秒）
    pub entropy_score: f64, // 用途：熵值评分（0-1）
}

/// 用途：JWT密钥生成请求
/// 说明：自定义JWT密钥生成参数
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct GenerateJwtRequest {
    pub key_length: Option<usize>, // 用途：密钥长度（字节），默认64
    pub algorithm: Option<String>, // 用途：加密算法，默认HS256
    pub include_special_chars: Option<bool>, // 用途：是否包含特殊字符，默认true
}

/// 用途：超级管理员创建请求
/// 说明：包含创建超级管理员所需的参数
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateAdminRequest {
    pub username: String,      // 用途：管理员用户名
    pub password: String,      // 用途：管理员密码
    pub name: String,          // 用途：管理员姓名
    pub email: Option<String>, // 用途：管理员邮箱（可选）
}

/// 用途：超级管理员创建响应
/// 说明：返回创建的超级管理员信息
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CreateAdminResponse {
    pub user_id: String,       // 用途：用户ID
    pub username: String,      // 用途：用户名
    pub name: String,          // 用途：姓名
    pub email: Option<String>, // 用途：邮箱
    pub created_at: String,    // 用途：创建时间
    pub role_id: String,       // 用途：角色ID
}

/// 用途：初始化步骤状态
/// 说明：表示单个初始化步骤的状态
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum StepStatus {
    #[serde(rename = "pending")]
    Pending, // 等待中
    #[serde(rename = "in_progress")]
    InProgress, // 进行中
    #[serde(rename = "completed")]
    Completed, // 已完成
    #[serde(rename = "failed")]
    Failed, // 失败
}

/// 用途：初始化步骤详情
/// 说明：包含单个初始化步骤的详细信息
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct InitStepDetail {
    pub step: InitStep,          // 用途：步骤标识
    pub name: String,            // 用途：步骤名称
    pub status: StepStatus,      // 用途：步骤状态
    pub message: Option<String>, // 用途：步骤消息
    pub error: Option<String>,   // 用途：错误信息（如果失败）
}

/// 用途：初始化进度响应
/// 说明：返回系统初始化的详细进度信息
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct InitProgressResponse {
    pub current_step: Option<InitStep>, // 用途：当前步骤
    pub total_steps: u8,                // 用途：总步骤数
    pub completed_steps: u8,            // 用途：已完成步骤数
    pub progress_percentage: u8,        // 用途：进度百分比 (0-100)
    pub steps: Vec<InitStepDetail>,     // 用途：所有步骤详情
    pub overall_status: InitStatus,     // 用途：整体状态
}

/// 用途：初始化配置保存请求
/// 说明：包含系统初始化的所有配置
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SaveInitConfigRequest {
    pub server_url: String,       // 用途：服务器URL
    pub db_url: String,           // 用途：数据库URL
    pub db_pool_len: usize,       // 用途：数据库连接池大小
    pub db_pool_timeout: usize,   // 用途：数据库连接超时
    pub jwt_secret: String,       // 用途：JWT密钥
    pub jwt_exp: usize,           // 用途：JWT过期时间
    pub jwt_refresh_token: usize, // 用途：JWT刷新令牌过期时间
}

/// 用途：检查系统是否已初始化
/// 说明：从数据库读取系统配置，判断是否已完成初始化
async fn check_system_initialized(
    conn: &impl rbatis::executor::Executor,
) -> Result<bool, StatusCode> {
    match KeyValueConfig::is_initialized(conn).await {
        Ok(initialized) => {
            // 返回系统配置的initialized字段
            Ok(initialized)
        }
        Err(e) => {
            log::error!("获取系统配置失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 用途：检查系统初始化状态
/// 说明：判断系统是否已完成初始化配置
#[utoipa::path(
    get,
    path = "/init/check",
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<InitStatusResponse>),
        (status = 500, description = "查询失败", body = ApiResponse<InitStatusResponse>)
    ),
    tag = "init"
)]
pub async fn check_init_status(
    State(context): State<Arc<ServiceContext>>,
) -> Result<Json<ApiResponse<InitStatusResponse>>, StatusCode> {
    let conn = context.rb.acquire().await.map_err(|e| {
        log::error!("获取数据库连接失败: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let initialized = KeyValueConfig::is_initialized(&conn).await.map_err(|e| {
        log::error!("获取初始化状态失败: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let (status, message) = if initialized {
        let initialized_at = KeyValueConfig::get_value(&conn, "init_time", "")
            .await
            .unwrap_or_else(|_| "".to_string());

        (
            InitStatus::Initialized { initialized_at },
            "系统已初始化".to_string(),
        )
    } else {
        (InitStatus::NotInitialized, "系统未初始化".to_string())
    };

    let response = InitStatusResponse {
        status,
        initialized,
        message,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 用途：验证数据库连接
/// 说明：测试数据库连接是否可用
#[utoipa::path(
    post,
    path = "/init/verify-db",
    request_body = VerifyDbRequest,
    responses(
        (status = 200, description = "验证成功", body = ApiResponse<VerifyDbResponse>),
        (status = 403, description = "系统已初始化", body = ApiResponse<VerifyDbResponse>),
        (status = 500, description = "验证失败", body = ApiResponse<VerifyDbResponse>)
    ),
    tag = "init"
)]
pub async fn verify_db_connection(
    State(context): State<Arc<ServiceContext>>,
    Json(req): Json<VerifyDbRequest>,
) -> Result<Json<ApiResponse<VerifyDbResponse>>, StatusCode> {
    use std::time::Instant;

    let conn = context.rb.acquire().await.map_err(|e| {
        log::error!("获取数据库连接失败: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if check_system_initialized(&conn).await? {
        warn!("[INIT] 数据库连接验证请求被拒绝，系统已完成初始化");
        let response = VerifyDbResponse {
            success: false,
            message: "系统已完成初始化，该功能仅能使用一次".to_string(),
            connection_info: None,
            error_details: Some(ErrorDetails {
                error_code: Some("INIT_ALREADY_DONE".to_string()),
                error_message: "系统已完成初始化".to_string(),
                suggestion: Some("请直接使用系统，无需再次初始化".to_string()),
            }),
        };
        return Ok(Json(ApiResponse::error(
            "INIT_ALREADY_DONE",
            &response.message,
        )));
    }

    info!(
        "[INIT] 收到数据库连接验证请求: db_type={}, host={:?}, port={:?}, database={}",
        req.db_type, req.host, req.port, req.database
    );

    let db_url = match req.db_type.as_str() {
        "mysql" => {
            let host = req.host.as_ref().ok_or_else(|| {
                error!("[INIT] MySQL数据库缺少host参数");
                StatusCode::BAD_REQUEST
            })?;
            let port = req.port.ok_or_else(|| {
                error!("[INIT] MySQL数据库缺少port参数");
                StatusCode::BAD_REQUEST
            })?;
            let username = req.username.as_ref().ok_or_else(|| {
                error!("[INIT] MySQL数据库缺少username参数");
                StatusCode::BAD_REQUEST
            })?;
            let password = req.password.as_ref().ok_or_else(|| {
                error!("[INIT] MySQL数据库缺少password参数");
                StatusCode::BAD_REQUEST
            })?;
            format!(
                "mysql://{}:{}@{}:{}/{}",
                username,
                password,
                host,
                port,
                req.database.clone()
            )
        }
        "postgres" => {
            let host = req.host.as_ref().ok_or_else(|| {
                error!("[INIT] PostgreSQL数据库缺少host参数");
                StatusCode::BAD_REQUEST
            })?;
            let port = req.port.ok_or_else(|| {
                error!("[INIT] PostgreSQL数据库缺少port参数");
                StatusCode::BAD_REQUEST
            })?;
            let username = req.username.as_ref().ok_or_else(|| {
                error!("[INIT] PostgreSQL数据库缺少username参数");
                StatusCode::BAD_REQUEST
            })?;
            let password = req.password.as_ref().ok_or_else(|| {
                error!("[INIT] PostgreSQL数据库缺少password参数");
                StatusCode::BAD_REQUEST
            })?;
            format!(
                "postgres://{}:{}@{}:{}/{}",
                username,
                password,
                host,
                port,
                req.database.clone()
            )
        }
        "sqlite" => req.database.clone(),
        _ => {
            let response = VerifyDbResponse {
                success: false,
                message: "不支持的数据库类型".to_string(),
                connection_info: None,
                error_details: Some(ErrorDetails {
                    error_code: Some("UNSUPPORTED_DB_TYPE".to_string()),
                    error_message: format!("不支持的数据库类型: {}", req.db_type),
                    suggestion: Some("支持的数据库类型: mysql, postgres, sqlite".to_string()),
                }),
            };
            return Ok(Json(ApiResponse::error(
                "UNSUPPORTED_DB_TYPE",
                &response.message,
            )));
        }
    };

    let start_time = Instant::now();

    let rb = RBatis::new();
    match rb
        .link(include!(concat!(env!("OUT_DIR"), "/driver.rs")), &db_url)
        .await
    {
        Ok(_) => {
            let connection_time = start_time.elapsed().as_millis() as u64;
            info!(
                "[INIT] 数据库连接验证成功: db_type={}, host={:?}, 耗时={}ms",
                req.db_type, req.host, connection_time
            );

            KeyValueConfig::set_value(
                &conn,
                "init_step_database",
                "completed",
                Some("数据库配置步骤状态"),
            )
            .await
            .map_err(|e| {
                log::error!("保存数据库配置步骤状态失败: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            let response = VerifyDbResponse {
                success: true,
                message: "数据库连接成功".to_string(),
                connection_info: Some(ConnectionInfo {
                    db_type: req.db_type.clone(),
                    host: req.host.clone(),
                    port: req.port,
                    database: req.database.clone(),
                    version: None,
                    connection_time_ms: Some(connection_time),
                }),
                error_details: None,
            };
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            let connection_time = start_time.elapsed().as_millis() as u64;
            let error_msg = e.to_string();
            warn!(
                "[INIT] 数据库连接验证失败: db_type={}, host={:?}, 耗时={}ms, error={}",
                req.db_type, req.host, connection_time, error_msg
            );

            let suggestion = if error_msg.contains("connection refused") {
                Some("请检查数据库服务是否启动，主机地址和端口是否正确".to_string())
            } else if error_msg.contains("authentication") || error_msg.contains("password") {
                Some("请检查用户名和密码是否正确".to_string())
            } else if error_msg.contains("database") && error_msg.contains("not exist") {
                Some("请检查数据库名称是否正确，或先创建数据库".to_string())
            } else if error_msg.contains("timeout") {
                Some("连接超时，请检查网络连接或增加超时时间".to_string())
            } else {
                Some("请检查数据库配置是否正确".to_string())
            };

            let response = VerifyDbResponse {
                success: false,
                message: format!("数据库连接失败: {}", error_msg),
                connection_info: None,
                error_details: Some(ErrorDetails {
                    error_code: Some("DB_CONNECTION_FAILED".to_string()),
                    error_message: error_msg,
                    suggestion,
                }),
            };
            Ok(Json(ApiResponse::error(
                "DB_CONNECTION_FAILED",
                &response.message,
            )))
        }
    }
}

/// 用途：生成JWT密钥
/// 说明：生成随机的JWT密钥，支持自定义长度、算法和字符集
#[utoipa::path(
    post,
    path = "/init/generate-jwt",
    request_body = GenerateJwtRequest,
    responses(
        (status = 200, description = "生成成功", body = ApiResponse<JwtSecretResponse>),
        (status = 403, description = "系统已初始化", body = ApiResponse<JwtSecretResponse>),
        (status = 500, description = "生成失败", body = ApiResponse<JwtSecretResponse>)
    ),
    tag = "init"
)]
#[axum::debug_handler]
pub async fn generate_jwt_secret(
    State(context): State<Arc<ServiceContext>>,
    Json(req): Json<GenerateJwtRequest>,
) -> Result<Json<ApiResponse<JwtSecretResponse>>, StatusCode> {
    let conn = context.rb.acquire().await.map_err(|e| {
        log::error!("获取数据库连接失败: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if check_system_initialized(&conn).await? {
        warn!("[INIT] JWT密钥生成请求被拒绝，系统已完成初始化");
        return Ok(Json(ApiResponse::<JwtSecretResponse>::error(
            "INIT_ALREADY_DONE",
            "系统已完成初始化，该功能仅能使用一次",
        )));
    }

    let key_length = req.key_length.unwrap_or(64);
    let algorithm = req.algorithm.unwrap_or_else(|| "HS256".to_string());
    let include_special_chars = req.include_special_chars.unwrap_or(true);

    if !(32..=256).contains(&key_length) {
        warn!("[INIT] JWT密钥长度无效: {}", key_length);
        return Ok(Json(ApiResponse::<JwtSecretResponse>::error(
            "INVALID_KEY_LENGTH",
            "密钥长度必须在32-256字节之间",
        )));
    }

    if !["HS256", "HS384", "HS512", "RS256", "RS384", "RS512"].contains(&algorithm.as_str()) {
        warn!("[INIT] 不支持的JWT算法: {}", algorithm);
        return Ok(Json(ApiResponse::<JwtSecretResponse>::error(
            "UNSUPPORTED_ALGORITHM",
            "不支持的JWT算法",
        )));
    }

    info!(
        "[INIT] 收到JWT密钥生成请求: length={}, algorithm={}",
        key_length, algorithm
    );

    let charset: &[u8] = if include_special_chars {
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+-=[]{}|;:,.<>?"
    } else {
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
    };

    let secret: String = {
        let mut rng = rand::rng();
        (0..key_length)
            .map(|_| {
                let idx = rng.random_range(0..charset.len());
                charset[idx] as char
            })
            .collect()
    };

    let charset_size = charset.len() as f64;
    let entropy_score = (key_length as f64) * charset_size.log2() / (key_length as f64 * 8.0);
    let entropy_score = entropy_score.min(1.0);

    let strength = if key_length >= 64 && include_special_chars {
        "强"
    } else if key_length >= 48 {
        "中"
    } else {
        "弱"
    };

    let expires_in = 86400;

    info!(
        "[INIT] JWT密钥生成成功，密钥长度: {}字节, 算法: {}, 强度: {}, 熵值: {:.2}",
        key_length, algorithm, strength, entropy_score
    );

    KeyValueConfig::set_value(
        &conn,
        "init_step_jwt",
        "completed",
        Some("JWT密钥生成步骤状态"),
    )
    .await
    .map_err(|e| {
        log::error!("保存JWT密钥生成步骤状态失败: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let response = JwtSecretResponse {
        secret,
        algorithm,
        key_length,
        strength: strength.to_string(),
        expires_in,
        entropy_score,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 用途：创建超级管理员
/// 说明：动态创建系统超级管理员账户，支持用户名唯一性验证和密码强度检查
#[utoipa::path(
    post,
    path = "/init/create-admin",
    request_body = CreateAdminRequest,
    responses(
        (status = 200, description = "创建成功", body = ApiResponse<CreateAdminResponse>),
        (status = 400, description = "参数错误", body = ApiResponse<CreateAdminResponse>),
        (status = 403, description = "系统已初始化", body = ApiResponse<CreateAdminResponse>),
        (status = 500, description = "创建失败", body = ApiResponse<CreateAdminResponse>)
    ),
    tag = "init"
)]
pub async fn create_super_admin(
    State(context): State<Arc<ServiceContext>>,
    Json(req): Json<CreateAdminRequest>,
) -> Result<Json<ApiResponse<CreateAdminResponse>>, StatusCode> {
    {
        let conn = context.rb.acquire().await.map_err(|e| {
            log::error!("获取数据库连接失败: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        if check_system_initialized(&conn).await? {
            warn!("[INIT] 超级管理员创建请求被拒绝，系统已完成初始化");
            return Ok(Json(ApiResponse::error(
                "INIT_ALREADY_DONE",
                "系统已完成初始化，该功能仅能使用一次",
            )));
        }

        info!(
            "[INIT] 收到超级管理员创建请求: username={}, name={}",
            req.username, req.name
        );

        // sqlite 模式下连接池会被限制为 1（见 context.rs），如果在同一个请求里嵌套 acquire，
        // 会导致死等从而表现为前端“响应超时”。这里把前置校验/清理逻辑放在独立作用域，
        // 让 conn 在后续步骤开始前被 drop。

        match SysUser::select_by_map(&conn, rbs::value! { "account": &req.username }).await {
            Ok(users) if !users.is_empty() => {
                warn!("[INIT] 用户名已存在，准备覆盖: {}", req.username);
                match SysUser::delete_by_map(&conn, rbs::value! { "account": &req.username }).await
                {
                    Ok(_) => {
                        info!("[INIT] 已删除旧管理员账户: {}", req.username);
                    }
                    Err(e) => {
                        warn!("[INIT] 删除旧管理员账户失败: {}", e);
                        return Ok(Json(ApiResponse::error(
                            "DELETE_USER_FAILED",
                            "删除旧管理员账户失败",
                        )));
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                warn!("[INIT] 检查用户名失败: {}", e);
                return Ok(Json(ApiResponse::error("DB_ERROR", "检查用户名失败")));
            }
        }
    }

    // 注意：上面作用域结束后，conn 已释放，后续步骤可以安全 acquire 新连接。

    if !req
        .username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_')
        || req.username.len() < 3
        || req.username.len() > 20
    {
        warn!("[INIT] 用户名格式无效: {}", req.username);
        return Ok(Json(ApiResponse::error(
            "INVALID_USERNAME",
            "用户名格式无效：3-20个字符，仅允许字母、数字、下划线",
        )));
    }

    let password = &req.password;
    if password.len() < 8 {
        warn!("[INIT] 密码长度不足: {} 字符", password.len());
        return Ok(Json(ApiResponse::error(
            "PASSWORD_TOO_SHORT",
            "密码长度至少为8个字符",
        )));
    }

    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());

    if !has_uppercase || !has_lowercase || !has_digit {
        warn!("[INIT] 密码强度不足: 需要包含大小写字母和数字");
        return Ok(Json(ApiResponse::error(
            "PASSWORD_WEAK",
            "密码强度不足：需要包含大小写字母和数字",
        )));
    }

    if let Some(ref email) = req.email
        && (!email.contains('@') || !email.contains('.'))
    {
        warn!("[INIT] 邮箱格式无效: {}", email);
        return Ok(Json(ApiResponse::error("INVALID_EMAIL", "邮箱格式无效")));
    }

    let mut transaction_manager = InitTransactionManager::new(context.rb.clone());

    let transaction_id = transaction_manager.begin_transaction().await.map_err(|e| {
        error!("[INIT] 开始事务失败: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    info!("[INIT] 开始创建超级管理员事务: {}", transaction_id);

    let user_id = ulid::Ulid::new().to_string();
    let created_at = DateTime::now().to_string();

    let user = SysUser {
        id: Some(user_id.clone()),
        account: Some(req.username.clone()),
        password: Some(PasswordEncoder::encode(password)),
        name: Some(req.name.clone()),
        email: None,
        avatar: Some("/user.png".to_string()),
        login_check: Some(LoginCheck::PasswordCheck),
        state: Some(1),
        create_date: Some(DateTime::now()),
        balance: Some(0.0),
        user_level: Some("L5".to_string()),
    };

    transaction_manager.add_rollback_action(crate::service::RollbackAction::DeleteAdmin {
        account: req.username.clone(),
    });

    if let Err(e) = transaction_manager
        .execute_step("create_admin_user", || async {
            let conn = context
                .rb
                .acquire()
                .await
                .map_err(|e| format!("获取数据库连接失败: {}", e))?;

            SysUser::insert(&conn, &user)
                .await
                .map_err(|e| format!("插入用户失败: {}", e))?;

            Ok::<(), String>(())
        })
        .await
    {
        error!("[INIT] 创建超级管理员用户步骤失败: {}", e);
        let _ = transaction_manager.rollback_transaction().await;
        return Ok(Json(ApiResponse::error(
            "CREATE_USER_FAILED",
            "创建用户失败",
        )));
    }

    // 查询超级管理员角色
    let role_id = {
        let conn = match context.rb.acquire().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("[INIT] 获取数据库连接失败: {}", e);
                let _ = transaction_manager.rollback_transaction().await;
                return Ok(Json(ApiResponse::error(
                    "DB_CONNECTION_FAILED",
                    "获取数据库连接失败",
                )));
            }
        };

        match crate::domain::table::rbac::RbacRole::select_by_map(
            &conn,
            rbs::value!({"name": "admin"}),
        )
        .await
        {
            Ok(roles) if !roles.is_empty() => {
                roles[0].id.clone().unwrap_or_else(|| "1".to_string())
            }
            Ok(_) => {
                warn!("[INIT] 未找到admin角色");
                let _ = transaction_manager.rollback_transaction().await;
                return Ok(Json(ApiResponse::error(
                    "ROLE_NOT_FOUND",
                    "未找到admin角色",
                )));
            }
            Err(e) => {
                warn!("[INIT] 查询admin角色失败: {}", e);
                let _ = transaction_manager.rollback_transaction().await;
                return Ok(Json(ApiResponse::error("DB_ERROR", "查询admin角色失败")));
            }
        }
    };

    if let Err(e) = transaction_manager
        .execute_step("create_user_role", || async {
            let conn = context
                .rb
                .acquire()
                .await
                .map_err(|e| format!("获取数据库连接失败: {}", e))?;

            let user_role = RbacUserRole {
                id: Some(ulid::Ulid::new().to_string()),
                user_id: Some(user_id.clone()),
                role_id: Some(role_id.clone()),
                create_date: Some(DateTime::now()),
            };

            RbacUserRole::insert(&conn, &user_role)
                .await
                .map_err(|e| format!("插入用户角色关联失败: {}", e))?;

            Ok::<(), String>(())
        })
        .await
    {
        error!("[INIT] 创建用户角色关联步骤失败: {}", e);
        let _ = transaction_manager.rollback_transaction().await;
        return Ok(Json(ApiResponse::error(
            "CREATE_USER_ROLE_FAILED",
            "创建用户角色关联失败",
        )));
    }

    transaction_manager.add_rollback_action(crate::service::RollbackAction::DeleteUserRole {
        user_id: user_id.clone(),
    });

    if req.username == "testrollback" {
        error!(
            "[INIT] 模拟事务失败以测试回滚功能: username={}",
            req.username
        );
        let _ = transaction_manager.rollback_transaction().await;
        return Ok(Json(ApiResponse::error(
            "ROLLBACK_TEST",
            "模拟事务失败以测试回滚功能",
        )));
    }

    if let Err(e) = transaction_manager.commit_transaction().await {
        error!("[INIT] 提交事务失败: {}", e);
        return Ok(Json(ApiResponse::error("COMMIT_FAILED", "提交事务失败")));
    }

    info!(
        "[INIT] 超级管理员创建成功: user_id={}, username={}, role_id={}",
        user_id, req.username, role_id
    );

    let conn = context.rb.acquire().await.map_err(|e| {
        log::error!("获取数据库连接失败: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    KeyValueConfig::set_value(
        &conn,
        "init_step_admin",
        "completed",
        Some("超级管理员创建步骤状态"),
    )
    .await
    .map_err(|e| {
        log::error!("保存超级管理员创建步骤状态失败: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let response = CreateAdminResponse {
        user_id,
        username: req.username,
        name: req.name,
        email: req.email,
        created_at,
        role_id,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 用途：获取初始化进度
/// 说明：返回系统初始化的详细进度信息，包括当前步骤、完成状态和进度百分比
#[utoipa::path(
    get,
    path = "/init/progress",
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<InitProgressResponse>),
        (status = 500, description = "查询失败", body = ApiResponse<InitProgressResponse>)
    ),
    tag = "init"
)]
pub async fn get_init_progress(
    State(context): State<Arc<ServiceContext>>,
) -> Result<Json<ApiResponse<InitProgressResponse>>, StatusCode> {
    let conn = context.rb.acquire().await.map_err(|e| {
        log::error!("获取数据库连接失败: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let initialized = check_system_initialized(&conn).await?;

    let all_steps = [
        InitStep::Database,
        InitStep::Jwt,
        InitStep::Admin,
        InitStep::Config,
    ];

    let total_steps = all_steps.len() as u8;

    if initialized {
        let initialized_at = KeyValueConfig::get_value(&conn, "init_time", "")
            .await
            .unwrap_or_else(|_| "".to_string());

        let steps = all_steps
            .iter()
            .map(|step| InitStepDetail {
                step: step.clone(),
                name: match step {
                    InitStep::Database => "数据库配置".to_string(),
                    InitStep::Jwt => "JWT密钥生成".to_string(),
                    InitStep::Admin => "超级管理员创建".to_string(),
                    InitStep::Config => "系统配置保存".to_string(),
                },
                status: StepStatus::Completed,
                message: Some("已完成".to_string()),
                error: None,
            })
            .collect();

        let response = InitProgressResponse {
            current_step: None,
            total_steps,
            completed_steps: total_steps,
            progress_percentage: 100,
            steps,
            overall_status: InitStatus::Initialized { initialized_at },
        };

        Ok(Json(ApiResponse::success(response)))
    } else {
        let failed_step = KeyValueConfig::get_value(&conn, "init_failed_step", "")
            .await
            .unwrap_or_else(|_| "".to_string());

        let failed_reason = KeyValueConfig::get_value(&conn, "init_failed_reason", "")
            .await
            .unwrap_or_else(|_| "".to_string());

        if !failed_step.is_empty() {
            let failed_step_enum = match failed_step.as_str() {
                "database" => InitStep::Database,
                "jwt" => InitStep::Jwt,
                "admin" => InitStep::Admin,
                "config" => InitStep::Config,
                _ => InitStep::Database,
            };

            let steps = all_steps
                .iter()
                .map(|step| {
                    let status = if step == &failed_step_enum {
                        StepStatus::Failed
                    } else {
                        StepStatus::Pending
                    };

                    InitStepDetail {
                        step: step.clone(),
                        name: match step {
                            InitStep::Database => "数据库配置".to_string(),
                            InitStep::Jwt => "JWT密钥生成".to_string(),
                            InitStep::Admin => "超级管理员创建".to_string(),
                            InitStep::Config => "系统配置保存".to_string(),
                        },
                        status,
                        message: if step == &failed_step_enum {
                            Some("初始化失败".to_string())
                        } else {
                            Some("等待中".to_string())
                        },
                        error: if step == &failed_step_enum {
                            Some(failed_reason.clone())
                        } else {
                            None
                        },
                    }
                })
                .collect();

            let response = InitProgressResponse {
                current_step: Some(failed_step_enum),
                total_steps,
                completed_steps: 0,
                progress_percentage: 0,
                steps,
                overall_status: InitStatus::Failed {
                    reason: failed_reason,
                    failed_step,
                },
            };

            Ok(Json(ApiResponse::success(response)))
        } else {
            let database_status = KeyValueConfig::get_value(&conn, "init_step_database", "pending")
                .await
                .unwrap_or_else(|_| "pending".to_string());
            let jwt_status = KeyValueConfig::get_value(&conn, "init_step_jwt", "pending")
                .await
                .unwrap_or_else(|_| "pending".to_string());
            let admin_status = KeyValueConfig::get_value(&conn, "init_step_admin", "pending")
                .await
                .unwrap_or_else(|_| "pending".to_string());
            let config_status = KeyValueConfig::get_value(&conn, "init_step_config", "pending")
                .await
                .unwrap_or_else(|_| "pending".to_string());

            let mut completed_steps: u8 = 0;
            let mut current_step: Option<InitStep> = None;

            let steps = all_steps
                .iter()
                .map(|step| {
                    let (status, message) = match step {
                        InitStep::Database => {
                            if database_status == "completed" {
                                completed_steps += 1;
                                (StepStatus::Completed, Some("已完成".to_string()))
                            } else {
                                if current_step.is_none() {
                                    current_step = Some(InitStep::Database);
                                }
                                (StepStatus::Pending, Some("等待中".to_string()))
                            }
                        }
                        InitStep::Jwt => {
                            if jwt_status == "completed" {
                                completed_steps += 1;
                                (StepStatus::Completed, Some("已完成".to_string()))
                            } else {
                                if current_step.is_none() && database_status == "completed" {
                                    current_step = Some(InitStep::Jwt);
                                }
                                (StepStatus::Pending, Some("等待中".to_string()))
                            }
                        }
                        InitStep::Admin => {
                            if admin_status == "completed" {
                                completed_steps += 1;
                                (StepStatus::Completed, Some("已完成".to_string()))
                            } else {
                                if current_step.is_none() && jwt_status == "completed" {
                                    current_step = Some(InitStep::Admin);
                                }
                                (StepStatus::Pending, Some("等待中".to_string()))
                            }
                        }
                        InitStep::Config => {
                            if config_status == "completed" {
                                completed_steps += 1;
                                (StepStatus::Completed, Some("已完成".to_string()))
                            } else {
                                if current_step.is_none() && admin_status == "completed" {
                                    current_step = Some(InitStep::Config);
                                }
                                (StepStatus::Pending, Some("等待中".to_string()))
                            }
                        }
                    };

                    InitStepDetail {
                        step: step.clone(),
                        name: match step {
                            InitStep::Database => "数据库配置".to_string(),
                            InitStep::Jwt => "JWT密钥生成".to_string(),
                            InitStep::Admin => "超级管理员创建".to_string(),
                            InitStep::Config => "系统配置保存".to_string(),
                        },
                        status,
                        message,
                        error: None,
                    }
                })
                .collect();

            let progress_percentage = ((completed_steps as i32 * 100) / total_steps as i32) as u8;

            let overall_status = if completed_steps == total_steps {
                let initialized_at = KeyValueConfig::get_value(&conn, "init_time", "")
                    .await
                    .unwrap_or_else(|_| "".to_string());
                InitStatus::Initialized { initialized_at }
            } else {
                InitStatus::NotInitialized
            };

            let response = InitProgressResponse {
                current_step,
                total_steps,
                completed_steps,
                progress_percentage,
                steps,
                overall_status,
            };

            Ok(Json(ApiResponse::success(response)))
        }
    }
}

/// 用途：保存初始化配置
/// 说明：保存系统初始化配置到文件，并将系统标记为已初始化
#[utoipa::path(
    post,
    path = "/init/config",
    request_body = SaveInitConfigRequest,
    responses(
        (status = 200, description = "保存成功", body = ApiResponse<serde_json::Value>),
        (status = 403, description = "系统已初始化", body = ApiResponse<serde_json::Value>),
        (status = 500, description = "保存失败", body = ApiResponse<serde_json::Value>)
    ),
    tag = "init"
)]
pub async fn save_init_config(
    State(context): State<Arc<ServiceContext>>,
    Json(req): Json<SaveInitConfigRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    info!("[INIT] 收到保存初始化配置请求");

    {
        let conn = context.rb.acquire().await.map_err(|e| {
            log::error!("获取数据库连接失败: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        if check_system_initialized(&conn).await? {
            warn!("[INIT] 保存初始化配置请求被拒绝，系统已完成初始化");
            return Ok(Json(ApiResponse::error(
                "INIT_ALREADY_DONE",
                "系统已完成初始化，该功能仅能使用一次",
            )));
        }

        // sqlite 模式下连接池会被限制为 1（见 context.rs），后续步骤中会再次 acquire 连接。
        // 这里提前结束 conn 的生命周期，避免后续 acquire 死等导致前端超时。
    }

    let mut transaction_manager = InitTransactionManager::new(context.rb.clone());

    let transaction_id = transaction_manager.begin_transaction().await.map_err(|e| {
        error!("[INIT] 开始事务失败: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    info!("[INIT] 开始初始化事务: {}", transaction_id);

    let config_path = "config/application.json5";
    let config_backup = match std::fs::read_to_string(config_path) {
        Ok(content) => content,
        Err(e) => {
            error!("[INIT] 读取配置文件失败: {}", e);
            let _ = transaction_manager.rollback_transaction().await;
            return Ok(Json(ApiResponse::error(
                "READ_CONFIG_FAILED",
                "读取配置文件失败",
            )));
        }
    };

    transaction_manager.add_rollback_action(crate::service::RollbackAction::RestoreConfig {
        backup_config: config_backup.clone(),
    });

    if let Err(e) = transaction_manager
        .execute_step("save_config", || async {
            let mut config_file =
                File::open(config_path).map_err(|e| format!("打开配置文件失败: {}", e))?;
            let mut config_content = String::new();
            config_file
                .read_to_string(&mut config_content)
                .map_err(|e| format!("读取配置文件失败: {}", e))?;

            let mut config: serde_json::Map<String, serde_json::Value> =
                json5::from_str(&config_content).map_err(|e| format!("解析配置文件失败: {}", e))?;

            config.insert(
                "server_url".to_string(),
                serde_json::Value::String(req.server_url.clone()),
            );
            config.insert(
                "db_url".to_string(),
                serde_json::Value::String(req.db_url.clone()),
            );
            config.insert(
                "db_pool_len".to_string(),
                serde_json::Value::Number(serde_json::Number::from(req.db_pool_len)),
            );
            config.insert(
                "db_pool_timeout".to_string(),
                serde_json::Value::Number(serde_json::Number::from(req.db_pool_timeout)),
            );
            config.insert(
                "jwt_secret".to_string(),
                serde_json::Value::String(req.jwt_secret.clone()),
            );
            config.insert(
                "jwt_exp".to_string(),
                serde_json::Value::Number(serde_json::Number::from(req.jwt_exp)),
            );
            config.insert(
                "jwt_refresh_token".to_string(),
                serde_json::Value::Number(serde_json::Number::from(req.jwt_refresh_token)),
            );

            let updated_config =
                json5::to_string(&config).map_err(|e| format!("序列化配置文件失败: {}", e))?;
            std::fs::write(config_path, updated_config)
                .map_err(|e| format!("写入配置文件失败: {}", e))?;

            Ok::<(), String>(())
        })
        .await
    {
        error!("[INIT] 保存配置步骤失败: {}", e);
        let _ = transaction_manager.rollback_transaction().await;
        return Ok(Json(ApiResponse::error(
            "SAVE_CONFIG_FAILED",
            "保存配置失败",
        )));
    }

    transaction_manager.add_rollback_action(crate::service::RollbackAction::UnmarkInit);

    if let Err(e) = transaction_manager
        .execute_step("init_user_levels", || async {
            info!("[INIT] 开始初始化默认用户等级配置...");
            context
                .user_level_service
                .init_default_levels()
                .await
                .map_err(|e| format!("初始化默认用户等级失败: {}", e))?;
            info!("[INIT] 默认用户等级配置初始化成功");
            Ok::<(), String>(())
        })
        .await
    {
        error!("[INIT] 初始化用户等级步骤失败: {}", e);
        let _ = transaction_manager.rollback_transaction().await;
        return Ok(Json(ApiResponse::error(
            "INIT_USER_LEVELS_FAILED",
            &format!("初始化用户等级失败: {}", e),
        )));
    }

    if let Err(e) = transaction_manager
        .execute_step("mark_initialized", || async {
            let conn = context
                .rb
                .acquire()
                .await
                .map_err(|e| format!("获取数据库连接失败: {}", e))?;

            KeyValueConfig::mark_initialized(&conn)
                .await
                .map_err(|e| format!("标记初始化状态失败: {}", e))?;

            let init_time = DateTime::now().to_string();
            KeyValueConfig::set_value(&conn, "init_time", &init_time, Some("初始化时间"))
                .await
                .map_err(|e| format!("保存初始化时间失败: {}", e))?;

            KeyValueConfig::set_value(&conn, "init_step_config", "completed", Some("系统配置保存"))
                .await
                .map_err(|e| format!("保存配置步骤状态失败: {}", e))?;

            Ok::<(), String>(())
        })
        .await
    {
        error!("[INIT] 标记初始化状态步骤失败: {}", e);
        let _ = transaction_manager.rollback_transaction().await;
        return Ok(Json(ApiResponse::error(
            "MARK_INIT_FAILED",
            &format!("标记初始化状态失败: {}", e),
        )));
    }

    if let Err(e) = transaction_manager.commit_transaction().await {
        error!("[INIT] 提交事务失败: {}", e);
        return Ok(Json(ApiResponse::error("COMMIT_FAILED", "提交事务失败")));
    }

    info!("[INIT] 系统初始化配置保存成功，事务ID: {}", transaction_id);

    Ok(Json(ApiResponse::success(serde_json::json!({
        "transaction_id": transaction_id,
        "message": "系统初始化配置保存成功"
    }))))
}

/// 用途：用户注册请求结构
/// 说明：接收用户注册时的输入数据
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RegisterRequest {
    /// 用户名称
    pub name: String,
    /// 用户邮箱
    pub email: String,
    /// 用户密码
    pub password: String,
    pub email_code: Option<String>,
    pub captcha_code: Option<String>,
    pub captcha_account: Option<String>,
    pub invite_code: Option<String>,
    pub apply_reason: Option<String>,
    /// 是否同意服务条款
    pub agree_terms: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SendRegisterEmailCodeRequest {
    pub email: String,
    pub captcha_code: Option<String>,
    pub captcha_account: Option<String>,
}

async fn verify_register_captcha(
    context: &Arc<ServiceContext>,
    policy: &RegisterPolicy,
    email: &str,
    captcha_code: Option<&str>,
    captcha_account: Option<&str>,
) -> Result<(), Json<ApiResponse<serde_json::Value>>> {
    if !policy.register_captcha_enabled {
        return Ok(());
    }

    let account = captcha_account
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| email.to_string());
    let code = captcha_code.unwrap_or("").trim().to_lowercase();
    if code.is_empty() {
        return Err(Json(ApiResponse::error(
            "CAPTCHA_REQUIRED",
            "请先输入图形验证码",
        )));
    }

    let cache_key = format!("captch:account_{}", account);
    let cached = match context.cache_service.get_string(&cache_key).await {
        Ok(value) => value.trim().to_lowercase(),
        Err(_) => {
            return Err(Json(ApiResponse::error(
                "CAPTCHA_EXPIRED",
                "图形验证码不存在或已过期",
            )));
        }
    };

    if cached != code {
        return Err(Json(ApiResponse::error(
            "CAPTCHA_INVALID",
            "图形验证码不正确",
        )));
    }

    // 校验成功后立即删除，避免被重放
    let _ = context.cache_service.del(&cache_key).await;

    Ok(())
}

/// 用途：用户注册控制器方法
/// 说明：处理用户注册请求，验证输入并创建新用户
#[utoipa::path(
    post,
    path = "/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "注册成功", body = ApiResponse<serde_json::Value>),
        (status = 400, description = "注册失败", body = ApiResponse<serde_json::Value>)
    ),
    tag = "auth"
)]
pub async fn register(
    State(context): State<Arc<ServiceContext>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    info!("[AUTH] 收到用户注册请求: email={}", req.email);

    let policy = RegisterPolicyService::get_policy(&context.rb).await;
    if !policy.allow_register {
        return Ok(Json(ApiResponse::error(
            "REGISTER_DISABLED",
            "当前站点已关闭用户注册",
        )));
    }

    let email = req.email.trim().to_lowercase();
    // 图形验证码：verify_register_captcha 已在校验成功后删除缓存 key，确保一次性使用
    if let Err(resp) = verify_register_captcha(
        &context,
        &policy,
        &email,
        req.captcha_code.as_deref(),
        req.captcha_account.as_deref(),
    )
    .await
    {
        return Ok(resp);
    }

    if policy.register_email_verify_enabled {
        let input_email_code = req.email_code.as_deref().unwrap_or("").trim();
        if input_email_code.is_empty() {
            return Ok(Json(ApiResponse::error(
                "EMAIL_CODE_REQUIRED",
                "请先输入邮箱验证码",
            )));
        }
        let code_key = format!("auth:register:email_code:{}", email);
        let cached_code = match context.cache_service.get_string(&code_key).await {
            Ok(code) => code,
            Err(_) => {
                return Ok(Json(ApiResponse::error(
                    "EMAIL_CODE_EXPIRED",
                    "邮箱验证码不存在或已过期",
                )));
            }
        };
        if input_email_code != cached_code.trim() {
            return Ok(Json(ApiResponse::error(
                "EMAIL_CODE_INVALID",
                "邮箱验证码不正确",
            )));
        }

        // 校验成功后立即删除，避免 10 分钟内被重放
        let _ = context.cache_service.del(&code_key).await;
    }

    let invite_code = req
        .invite_code
        .as_ref()
        .map(|v| v.trim().to_uppercase())
        .filter(|v| !v.is_empty());

    if policy.invite_code_required && invite_code.is_none() {
        return Ok(Json(ApiResponse::error(
            "INVITE_CODE_REQUIRED",
            "当前注册需要邀请码",
        )));
    }

    let mut valid_invite_code: Option<InvitationCode> = None;
    if let Some(code) = &invite_code {
        match RegisterPolicyService::find_invite_code(&context.rb, code).await {
            Ok(invite) => valid_invite_code = Some(invite),
            Err(_) => {
                return Ok(Json(ApiResponse::error(
                    "INVITE_CODE_INVALID",
                    "邀请码无效",
                )));
            }
        }
    }

    let should_review = policy.register_review_enabled
        && !(policy.invite_code_bypass_review && valid_invite_code.is_some());

    let invite_user_level = valid_invite_code
        .as_ref()
        .and_then(|invite| invite.user_level.clone());

    if should_review {
        let apply_reason = req
            .apply_reason
            .as_ref()
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        if apply_reason.is_empty() {
            return Ok(Json(ApiResponse::error(
                "APPLY_REASON_REQUIRED",
                "已开启注册审核，请填写入站申请说明",
            )));
        }

        let review_res = RegisterPolicyService::create_review(
            &context.rb,
            req.name.clone(),
            email.clone(),
            req.password.clone(),
            req.agree_terms,
            invite_code.clone(),
            invite_user_level.clone(),
            Some(apply_reason),
        )
        .await;

        match review_res {
            Ok(RegistrationReview { .. }) => {
                let _ = RegisterPolicyService::send_mail(
                    &context.rb,
                    &email,
                    "RSLLM 注册申请已提交",
                    "您的注册申请已提交，当前状态：待审核。审核通过后可直接登录。",
                )
                .await;
                return Ok(Json(ApiResponse::success(serde_json::json!({
                    "status": "pending_review",
                    "message": "注册申请已提交，等待审核"
                }))));
            }
            Err(e) => {
                return Ok(Json(ApiResponse::error(
                    "REGISTER_REVIEW_CREATE_FAILED",
                    &format!("注册申请创建失败: {}", e),
                )));
            }
        }
    }

    if let Some(code) = &invite_code {
        if let Err(e) = RegisterPolicyService::consume_invite_code(&context.rb, code, None).await {
            log::warn!("[AUTH] 邀请码消费失败: {}", e);
            return Ok(Json(ApiResponse::error(
                "INVITE_CODE_EXHAUSTED",
                "邀请码已用尽或不可用",
            )));
        }
    }

    let register_dto = crate::domain::dto::basic::register::UserRegisterDTO {
        name: req.name,
        email,
        password: req.password,
        user_level: invite_user_level,
        agree_terms: req.agree_terms,
    };

    match context.sys_user_service.register(&register_dto).await {
        Ok(result) => {
            if result.success {
                info!(
                    "[AUTH] 用户注册成功: user_id={}",
                    result
                        .user_id
                        .as_ref()
                        .map(|s| s.as_str())
                        .unwrap_or_default()
                );
                Ok(Json(ApiResponse::success(serde_json::json!({
                    "user_id": result.user_id,
                    "message": result.message
                }))))
            } else {
                warn!("[AUTH] 用户注册失败: {}", result.message);
                Ok(Json(ApiResponse::error("REGISTER_FAILED", &result.message)))
            }
        }
        Err(e) => {
            error!("[AUTH] 用户注册异常: {}", e);
            Ok(Json(ApiResponse::error(
                "REGISTER_ERROR",
                &format!("注册过程中发生错误: {}", e),
            )))
        }
    }
}

#[utoipa::path(
    post,
    path = "/auth/register/send-email-code",
    request_body = SendRegisterEmailCodeRequest,
    responses(
        (status = 200, description = "发送成功", body = ApiResponse<serde_json::Value>),
        (status = 400, description = "发送失败", body = ApiResponse<serde_json::Value>)
    ),
    tag = "auth"
)]
pub async fn send_register_email_code(
    State(context): State<Arc<ServiceContext>>,
    Json(req): Json<SendRegisterEmailCodeRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    let email = req.email.trim().to_lowercase();
    let policy = RegisterPolicyService::get_policy(&context.rb).await;
    if !policy.allow_register {
        return Ok(Json(ApiResponse::error(
            "REGISTER_DISABLED",
            "当前站点已关闭用户注册",
        )));
    }
    if let Err(msg) = UserRegisterValidator::validate_email(&email) {
        return Ok(Json(ApiResponse::error("INVALID_EMAIL", &msg)));
    }

    // 邮箱验证码发送限流：仅按邮箱（最小实现）
    // 60 秒内最多 1 次
    // 1 小时内最多 5 次
    let rl_60s_key = format!("auth:register:email_code:rl:60s:{}", email);
    let rl_1h_key = format!("auth:register:email_code:rl:1h:{}", email);

    let ttl_60s = context.cache_service.ttl(&rl_60s_key).await.unwrap_or(-2);
    if ttl_60s > 0 {
        return Ok(Json(ApiResponse::error(
            "EMAIL_CODE_RATE_LIMITED",
            "验证码发送过于频繁，请稍后再试",
        )));
    }

    let count_1h: i32 = context
        .cache_service
        .get_string(&rl_1h_key)
        .await
        .ok()
        .and_then(|v| v.trim().parse::<i32>().ok())
        .unwrap_or(0);
    if count_1h >= 5 {
        return Ok(Json(ApiResponse::error(
            "EMAIL_CODE_RATE_LIMITED",
            "验证码发送次数已达上限，请稍后再试",
        )));
    }

    // 写入限流标记
    let _ = context
        .cache_service
        .set_string_ex(&rl_60s_key, "1", Some(Duration::from_secs(60)))
        .await;
    let _ = context
        .cache_service
        .set_string_ex(
            &rl_1h_key,
            &(count_1h + 1).to_string(),
            Some(Duration::from_secs(3600)),
        )
        .await;

    let mut conn = match context.rb.acquire().await {
        Ok(c) => c,
        Err(e) => {
            return Ok(Json(ApiResponse::error(
                "DB_ERROR",
                &format!("数据库连接失败: {}", e),
            )));
        }
    };

    if !policy.register_email_verify_enabled {
        return Ok(Json(ApiResponse::error(
            "REGISTER_EMAIL_VERIFY_DISABLED",
            "当前系统已关闭注册邮箱验证码",
        )));
    }

    // 图形验证码：verify_register_captcha 已在校验成功后删除缓存 key，确保一次性使用
    if let Err(resp) = verify_register_captcha(
        &context,
        &policy,
        &email,
        req.captcha_code.as_deref(),
        req.captcha_account.as_deref(),
    )
    .await
    {
        return Ok(resp);
    }

    let smtp_host = KeyValueConfig::get_value(&mut conn, "system.smtp_host", "")
        .await
        .unwrap_or_default();
    let smtp_port = KeyValueConfig::get_value(&mut conn, "system.smtp_port", "465")
        .await
        .unwrap_or_else(|_| "465".to_string());
    let smtp_username = KeyValueConfig::get_value(&mut conn, "system.smtp_username", "")
        .await
        .unwrap_or_default();
    let smtp_password = KeyValueConfig::get_value(&mut conn, "system.smtp_password", "")
        .await
        .unwrap_or_default();
    let smtp_from = KeyValueConfig::get_value(&mut conn, "system.smtp_from", "")
        .await
        .unwrap_or_default();
    let smtp_from_name = KeyValueConfig::get_value(&mut conn, "system.smtp_from_name", "RSLLM")
        .await
        .unwrap_or_else(|_| "RSLLM".to_string());
    let smtp_starttls = KeyValueConfig::get_value(&mut conn, "system.smtp_starttls", "true")
        .await
        .unwrap_or_else(|_| "true".to_string());

    if smtp_host.is_empty()
        || smtp_username.is_empty()
        || smtp_password.is_empty()
        || smtp_from.is_empty()
    {
        return Ok(Json(ApiResponse::error(
            "SMTP_NOT_CONFIGURED",
            "系统未配置发信服务，请联系管理员",
        )));
    }

    let port = smtp_port.parse::<u16>().unwrap_or(465);
    let starttls = !smtp_starttls.eq_ignore_ascii_case("false");
    let code = format!("{:06}", rand::rng().random_range(0..1_000_000));

    let from_address = match smtp_from.parse() {
        Ok(v) => v,
        Err(_) => {
            return Ok(Json(ApiResponse::error(
                "SMTP_FROM_INVALID",
                "发件人邮箱配置无效",
            )));
        }
    };

    let message = match Message::builder()
        .from(Mailbox::new(Some(smtp_from_name), from_address))
        .to(match email.parse() {
            Ok(v) => v,
            Err(_) => {
                return Ok(Json(ApiResponse::error("INVALID_EMAIL", "邮箱格式无效")));
            }
        })
        .subject("RSLLM 注册验证码")
        .body(format!(
            "您的注册验证码是：{}，10分钟内有效。如非本人操作请忽略。",
            code
        )) {
        Ok(v) => v,
        Err(e) => {
            return Ok(Json(ApiResponse::error(
                "EMAIL_BUILD_FAILED",
                &format!("邮件构造失败: {}", e),
            )));
        }
    };

    let credentials = Credentials::new(smtp_username, smtp_password);
    let mailer = if starttls {
        match AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_host) {
            Ok(builder) => builder.port(port).credentials(credentials).build(),
            Err(e) => {
                return Ok(Json(ApiResponse::error(
                    "SMTP_RELAY_FAILED",
                    &format!("SMTP配置错误: {}", e),
                )));
            }
        }
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&smtp_host)
            .port(port)
            .credentials(credentials)
            .build()
    };

    if let Err(e) = mailer.send(message).await {
        return Ok(Json(ApiResponse::error(
            "EMAIL_SEND_FAILED",
            &format!("验证码发送失败: {}", e),
        )));
    }

    let cache_key = format!("auth:register:email_code:{}", email);
    if let Err(e) = context
        .cache_service
        .set_string_ex(&cache_key, &code, Some(Duration::from_secs(600)))
        .await
    {
        return Ok(Json(ApiResponse::error(
            "CACHE_SET_FAILED",
            &format!("验证码缓存失败: {}", e),
        )));
    }

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "验证码已发送，请查收邮箱"
    }))))
}
