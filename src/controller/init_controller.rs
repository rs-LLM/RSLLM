// 用途：初始化向导控制器
// 说明：处理系统初始化过程中的各种请求
use axum::{extract::State, http::StatusCode, Json};
use std::io::Read;
use std::sync::Arc;
use std::fs::File;
use rand::Rng;
use serde::{Deserialize, Serialize};
use rbatis::RBatis;
use rbatis::rbdc::DateTime;
use log::{info, warn};

use crate::context::ServiceContext;
use crate::domain::table::LoginCheck;
use crate::domain::table::rbac::RbacUserRole;
use crate::domain::table::sys_user::SysUser;
use crate::domain::table::key_value_config::KeyValueConfig;
use crate::util::password_encoder::PasswordEncoder;
use crate::domain::vo::response::ApiResponse;

/// 用途：初始化状态检查响应
/// 说明：返回系统是否已完成初始化
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct InitStatusResponse {
    pub initialized: bool,          // 用途：是否已完成初始化
    pub message: String,            // 用途：状态描述
}

/// 用途：数据库连接验证请求
/// 说明：包含数据库连接所需的参数
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct VerifyDbRequest {
    pub db_type: String,            // 用途：数据库类型
    pub host: String,               // 用途：数据库主机
    pub port: u16,                  // 用途：数据库端口
    pub database: String,           // 用途：数据库名称
    pub username: String,           // 用途：数据库用户名
    pub password: String,           // 用途：数据库密码
}

/// 用途：JWT密钥生成响应
/// 说明：返回生成的JWT密钥
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct JwtSecretResponse {
    pub secret: String,             // 用途：生成的JWT密钥
    pub strength: String,           // 用途：密钥强度
}

/// 用途：超级管理员创建请求
/// 说明：包含创建超级管理员所需的参数
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateAdminRequest {
    pub username: String,           // 用途：管理员用户名
    pub password: String,           // 用途：管理员密码
    pub name: String,               // 用途：管理员姓名
}

/// 用途：初始化配置保存请求
/// 说明：包含系统初始化的所有配置
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SaveInitConfigRequest {
    pub server_url: String,         // 用途：服务器URL
    pub db_url: String,             // 用途：数据库URL
    pub db_pool_len: usize,         // 用途：数据库连接池大小
    pub db_pool_timeout: usize,     // 用途：数据库连接超时
    pub jwt_secret: String,         // 用途：JWT密钥
    pub jwt_exp: usize,             // 用途：JWT过期时间
    pub jwt_refresh_token: usize,   // 用途：JWT刷新令牌过期时间
}

/// 用途：检查系统是否已初始化
/// 说明：从数据库读取系统配置，判断是否已完成初始化
async fn check_system_initialized(
    conn: &impl rbatis::executor::Executor
) -> Result<bool, StatusCode> {
    match KeyValueConfig::is_initialized(conn).await {
        Ok(initialized) => {
            // 返回系统配置的initialized字段
            Ok(initialized)
        },
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
    path = "/api/v1/init/status",
    responses(
        (status = 200, description = "查询成功", body = ApiResponse<InitStatusResponse>),
        (status = 500, description = "查询失败", body = ApiResponse<InitStatusResponse>)
    ),
    tag = "init"
)]
pub async fn check_init_status(
    State(context): State<Arc<ServiceContext>>,
) -> Result<Json<InitStatusResponse>, StatusCode> {
    // 从键值配置获取初始化状态
    let mut conn = context.rb.acquire().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let initialized = KeyValueConfig::is_initialized(&mut conn)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let message = if initialized {
        "系统已初始化".to_string()
    } else {
        "系统未初始化".to_string()
    };
    
    let response = InitStatusResponse {
        initialized,
        message,
    };
    
    Ok(Json(response))
}

/// 用途：验证数据库连接
/// 说明：测试数据库连接是否可用
#[utoipa::path(
    post,
    path = "/api/v1/init/verify-db",
    request_body = VerifyDbRequest,
    responses(
        (status = 200, description = "验证成功", body = ApiResponse<serde_json::Value>),
        (status = 403, description = "系统已初始化", body = ApiResponse<serde_json::Value>),
        (status = 500, description = "验证失败", body = ApiResponse<serde_json::Value>)
    ),
    tag = "init"
)]
pub async fn verify_db_connection(
    State(context): State<Arc<ServiceContext>>,
    Json(req): Json<VerifyDbRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // 检查系统是否已初始化
    let mut conn = context.rb.acquire().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if check_system_initialized(&mut conn).await? {
        warn!("[INIT] 数据库连接验证请求被拒绝，系统已完成初始化");
        return Ok(Json(serde_json::json!({"success": false, "message": "系统已完成初始化，该功能仅能使用一次"})));
    }
    
    info!("[INIT] 收到数据库连接验证请求: db_type={}, host={}, port={}, database={}", 
          req.db_type, req.host, req.port, req.database);
    
    // 构建数据库连接URL
    let db_url = match req.db_type.as_str() {
        "mysql" => format!("mysql://{}:{}@{}:{}/{}", req.username, req.password, req.host, req.port, req.database),
        "postgres" => format!("postgres://{}:{}@{}:{}/{}", req.username, req.password, req.host, req.port, req.database),
        "sqlite" => req.database,
        _ => return Ok(Json(serde_json::json!({"success": false, "message": "不支持的数据库类型"}))),
    };
    
    // 创建临时RBatis实例进行连接测试
    let rb = RBatis::new();
    match rb.link(include!("../../target/driver.rs"), &db_url).await {
        Ok(_) => {
            info!("[INIT] 数据库连接验证成功: db_type={}, host={}", req.db_type, req.host);
            Ok(Json(serde_json::json!({"success": true, "message": "数据库连接成功"})))
        },
        Err(e) => {
            warn!("[INIT] 数据库连接验证失败: db_type={}, host={}, error={}", req.db_type, req.host, e);
            Ok(Json(serde_json::json!({"success": false, "message": format!("数据库连接失败: {}", e)})))
        }
    }
}

/// 用途：生成JWT密钥
/// 说明：生成随机的JWT密钥
#[utoipa::path(
    post,
    path = "/api/v1/init/generate-jwt-secret",
    responses(
        (status = 200, description = "生成成功", body = ApiResponse<JwtSecretResponse>),
        (status = 403, description = "系统已初始化", body = ApiResponse<JwtSecretResponse>),
        (status = 500, description = "生成失败", body = ApiResponse<JwtSecretResponse>)
    ),
    tag = "init"
)]
pub async fn generate_jwt_secret(
    State(context): State<Arc<ServiceContext>>,
) -> Result<Json<JwtSecretResponse>, StatusCode> {
    // 检查系统是否已初始化
    let mut conn = context.rb.acquire().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if check_system_initialized(&mut conn).await? {
        warn!("[INIT] JWT密钥生成请求被拒绝，系统已完成初始化");
        return Err(StatusCode::FORBIDDEN);
    }
    
    info!("[INIT] 收到JWT密钥生成请求");
    
    // 生成64字节的随机密钥，使用更安全的字符集
    let mut rng = rand::thread_rng();
    let charset: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+-=[]{}|;:,.<>?";
    
    let secret: String = (0..64)
        .map(|_| {
            let idx = rng.gen_range(0..charset.len());
            charset[idx] as char
        })
        .collect();
    
    info!("[INIT] JWT密钥生成成功，密钥强度: 强");
    
    // 密钥强度：64字节，包含大小写字母、数字和特殊字符，强度为"强"
    let response = JwtSecretResponse {
        secret,
        strength: "强".to_string(),
    };
    
    Ok(Json(response))
}

/// 用途：创建超级管理员
/// 说明：动态创建系统超级管理员账户
#[utoipa::path(
    post,
    path = "/api/v1/init/create-admin",
    request_body = CreateAdminRequest,
    responses(
        (status = 200, description = "创建成功", body = ApiResponse<serde_json::Value>),
        (status = 403, description = "系统已初始化", body = ApiResponse<serde_json::Value>),
        (status = 500, description = "创建失败", body = ApiResponse<serde_json::Value>)
    ),
    tag = "init"
)]
pub async fn create_super_admin(
    State(context): State<Arc<ServiceContext>>,
    Json(req): Json<CreateAdminRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // 检查系统是否已初始化
    let mut conn = context.rb.acquire().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if check_system_initialized(&mut conn).await? {
        warn!("[INIT] 超级管理员创建请求被拒绝，系统已完成初始化");
        return Ok(Json(serde_json::json!({"success": false, "message": "系统已完成初始化，该功能仅能使用一次"})));
    }
    
    info!("[INIT] 收到超级管理员创建请求: username={}, name={}", req.username, req.name);
    
    // 开始事务
    let tx = conn.begin().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // 创建超级管理员用户
    let user = SysUser {
        id: Some("1".to_string()),
        account: Some(req.username.clone()),
        password: Some(PasswordEncoder::encode(&req.password)),
        name: Some(req.name.clone()),
        login_check: Some(LoginCheck::PasswordCheck),
        state: Some(1),
        create_date: Some(DateTime::now()),
        balance: Some(0.0),
    };
    
    // 保存用户
    if let Err(e) = SysUser::insert(&tx, &user).await {
        warn!("[INIT] 创建超级管理员失败: {}", e);
        tx.rollback().await.ok();
        return Ok(Json(serde_json::json!({"success": false, "message": "创建超级管理员失败"})));
    }
    
    // 关联用户和管理员角色
    let user_role = RbacUserRole {
        id: Some("1".to_string()),
        user_id: Some("1".to_string()),
        role_id: Some("1".to_string()),
        create_date: Some(DateTime::now()),
    };
    
    if let Err(e) = RbacUserRole::insert(&tx, &user_role).await {
        warn!("[INIT] 关联管理员角色失败: {}", e);
        tx.rollback().await.ok();
        return Ok(Json(serde_json::json!({"success": false, "message": "关联管理员角色失败"})));
    }
    
    // 提交事务
    tx.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    info!("[INIT] 超级管理员创建成功: username={}", req.username);
    
    Ok(Json(serde_json::json!({"success": true, "message": "超级管理员创建成功"})))
}

/// 用途：保存初始化配置
/// 说明：保存系统初始化配置到文件，并将系统标记为已初始化
pub async fn save_init_config(
    State(context): State<Arc<ServiceContext>>,
    Json(req): Json<SaveInitConfigRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("[INIT] 收到保存初始化配置请求");
    
    // 检查系统是否已初始化
    let mut conn = context.rb.acquire().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if check_system_initialized(&mut conn).await? {
        warn!("[INIT] 保存初始化配置请求被拒绝，系统已完成初始化");
        return Ok(Json(serde_json::json!({"success": false, "message": "系统已完成初始化，该功能仅能使用一次"})));
    }
    
    // 读取当前配置文件内容
    let mut config_file = File::open("application.json5").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut config_content = String::new();
    config_file.read_to_string(&mut config_content).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // 解析配置文件
    let mut config: serde_json::Map<String, serde_json::Value> = json5::from_str(&config_content).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // 更新配置项
    config.insert("server_url".to_string(), serde_json::Value::String(req.server_url));
    config.insert("db_url".to_string(), serde_json::Value::String(req.db_url));
    config.insert("db_pool_len".to_string(), serde_json::Value::Number(serde_json::Number::from(req.db_pool_len)));
    config.insert("db_pool_timeout".to_string(), serde_json::Value::Number(serde_json::Number::from(req.db_pool_timeout)));
    config.insert("jwt_secret".to_string(), serde_json::Value::String(req.jwt_secret));
    config.insert("jwt_exp".to_string(), serde_json::Value::Number(serde_json::Number::from(req.jwt_exp)));
    config.insert("jwt_refresh_token".to_string(), serde_json::Value::Number(serde_json::Number::from(req.jwt_refresh_token)));
    
    // 保存配置到文件
    let updated_config = json5::to_string(&config).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    std::fs::write("application.json5", updated_config).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // 更新系统初始化状态
    if let Err(e) = KeyValueConfig::mark_initialized(&mut conn).await {
        warn!("[INIT] 更新系统初始化状态失败: {}", e);
        return Ok(Json(serde_json::json!({"success": false, "message": "保存配置成功，但更新系统状态失败"})));
    }
    
    info!("[INIT] 系统初始化配置保存成功，系统已标记为已初始化");
    
    Ok(Json(serde_json::json!({"success": true, "message": "系统初始化配置保存成功"})))
}
