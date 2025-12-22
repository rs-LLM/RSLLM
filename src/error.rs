//! 用途：错误和结果类型定义
//! 说明：提供应用程序统一的错误处理机制

// 用途：导入标准错误 trait
// 说明：用于实现自定义错误类型
use std::error::Error as StdError;

// 用途：导入格式化相关的 trait
// 说明：用于实现错误的显示和调试功能
use std::fmt::{self, Debug, Display};

// 用途：导入IO模块
// 说明：用于处理IO错误的转换
use std::io;

// 用途：导入serde的反序列化相关类型
// 说明：用于实现错误类型的反序列化
use serde::de::Visitor;

// 用途：导入serde的序列化相关类型
// 说明：用于实现错误类型的序列化
use serde::ser::{Serialize, Serializer};

// 用途：导入serde的核心 trait
// 说明：用于实现错误类型的序列化和反序列化
use serde::{Deserialize, Deserializer};

/// 用途：定义应用程序的结果类型别名
/// 说明：简化错误处理，统一使用自定义Error类型
pub type Result<T> = std::result::Result<T, Error>;

/// 用途：通用错误枚举
/// 说明：表示应用程序中所有可能的失败方式
#[derive(Debug)]
#[non_exhaustive] // 用途：标记为非详尽枚举
                  // 说明：允许未来扩展错误类型而不破坏现有代码
pub enum Error {
    /// 用途：默认错误类型
    /// 说明：用于包装字符串形式的错误信息
    E(String),
    /// 用途：配置错误
    /// 说明：用于配置相关错误
    ConfigError(String),
    /// 用途：加密错误
    /// 说明：用于加密相关错误
    EncryptionError(String),
    /// 用途：业务错误
    /// 说明：用于业务逻辑相关错误
    BusinessError(String),
    /// 用途：认证错误
    /// 说明：用于认证相关错误
    AuthError(String),
    /// 用途：外部服务错误
    /// 说明：用于外部服务调用错误
    ExternalServiceError(String),
    /// 用途：验证错误
    /// 说明：用于输入验证错误
    ValidationError(String),
    /// 用途：限流错误
    /// 说明：用于速率限制错误
    RateLimitExceeded,
    /// 用途：数据库错误
    /// 说明：用于数据库操作错误
    DatabaseError(String),
    /// 用途：未找到错误
    /// 说明：用于资源不存在的错误
    NotFound(String),
}

/// 用途：实现Display trait
/// 说明：允许错误类型以友好的方式显示给用户
impl Display for Error {
    // 用途：格式化错误信息
    // 说明：将错误转换为字符串形式
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 用途：匹配错误类型
        // 说明：根据不同的错误类型生成相应的字符串表示
        match self {
            // 用途：处理默认错误类型
            // 说明：将内部字符串作为错误信息输出
            Error::E(error) => write!(f, "{}", error),
            Error::ConfigError(error) => write!(f, "Config Error: {}", error),
            Error::EncryptionError(error) => write!(f, "Encryption Error: {}", error),
            Error::BusinessError(error) => write!(f, "Business Error: {}", error),
            Error::AuthError(error) => write!(f, "Auth Error: {}", error),
            Error::ExternalServiceError(error) => write!(f, "External Service Error: {}", error),
            Error::ValidationError(error) => write!(f, "Validation Error: {}", error),
            Error::RateLimitExceeded => write!(f, "Rate Limit Exceeded"),
            Error::DatabaseError(error) => write!(f, "Database Error: {}", error),
            Error::NotFound(error) => write!(f, "Not Found: {}", error),
        }
    }
}

/// 用途：实现标准错误 trait
/// 说明：允许自定义错误类型与标准库错误处理兼容
impl StdError for Error {}

/// 用途：实现Axum的IntoResponse trait
/// 说明：允许Error类型直接作为HTTP响应返回
impl axum::response::IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::Json;
        use serde_json::json;
        
        // 根据错误类型确定HTTP状态码
        let status_code = match self {
            Error::BusinessError(_) => StatusCode::BAD_REQUEST,
            Error::AuthError(_) => StatusCode::UNAUTHORIZED,
            Error::NotFound(_) => StatusCode::NOT_FOUND,
            Error::ValidationError(_) => StatusCode::BAD_REQUEST,
            Error::RateLimitExceeded => StatusCode::TOO_MANY_REQUESTS,
            Error::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::EncryptionError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::ExternalServiceError(_) => StatusCode::BAD_GATEWAY,
            Error::ConfigError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::E(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        
        let response = json!({
            "success": false,
            "code": status_code.as_u16().to_string(),
            "message": self.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        (status_code, Json(response)).into_response()
    }
}

/// 用途：实现从IO错误到自定义错误的转换
/// 说明：方便处理IO操作产生的错误
impl From<io::Error> for Error {
    #[inline] // 用途：内联提示
             // 说明：减少函数调用开销，提高性能
    fn from(err: io::Error) -> Self {
        // 用途：将IO错误转换为字符串，再转换为自定义错误
        // 说明：统一错误处理，方便上层调用
        Error::from(err.to_string())
    }
}

/// 用途：实现从字符串切片到自定义错误的转换
/// 说明：方便直接使用字符串创建错误
impl From<&str> for Error {
    fn from(arg: &str) -> Self {
        // 用途：将字符串切片转换为自定义错误
        // 说明：允许直接使用字符串字面量创建错误
        return Error::E(arg.to_string());
    }
}

/// 用途：实现从String到自定义错误的转换
/// 说明：方便使用String创建错误
impl From<std::string::String> for Error {
    fn from(arg: String) -> Self {
        // 用途：将String转换为自定义错误
        // 说明：允许直接使用String对象创建错误
        return Error::E(arg);
    }
}

/// 用途：实现从通用错误类型到自定义错误的转换
/// 说明：方便处理各种实现了StdError的错误类型
impl From<&dyn std::error::Error> for Error {
    fn from(arg: &dyn std::error::Error) -> Self {
        // 用途：将通用错误转换为字符串，再转换为自定义错误
        // 说明：统一处理各种类型的错误
        return Error::E(arg.to_string());
    }
}

/// 用途：实现从自定义错误到IO错误的转换
/// 说明：方便在需要IO错误的上下文中使用自定义错误
impl From<Error> for std::io::Error {
    fn from(arg: Error) -> Self {
        // 用途：将自定义错误转换为IO错误
        // 说明：允许在需要IO错误的函数中返回自定义错误
        io::Error::new(io::ErrorKind::Other, arg.to_string())
    }
}

/// 用途：实现从rbatis错误到自定义错误的转换
/// 说明：方便处理数据库操作产生的错误
impl From<rbatis::Error> for Error {
    fn from(arg: rbatis::Error) -> Self {
        // 用途：将rbatis错误转换为自定义错误
        // 说明：统一处理数据库操作错误
        Error::DatabaseError(arg.to_string())
    }
}

/// 用途：实现Clone trait
/// 说明：允许错误类型被克隆，方便在异步和多线程环境中使用
impl Clone for Error {
    /// 用途：克隆错误对象
    /// 说明：创建一个新的错误对象，包含相同的错误信息
    fn clone(&self) -> Self {
        // 用途：将错误转换为字符串，再创建新的错误对象
        // 说明：实现简单且安全的克隆
        Error::from(self.to_string())
    }

    /// 用途：从源错误克隆到当前错误
    /// 说明：允许将一个错误的内容复制到另一个错误
    fn clone_from(&mut self, source: &Self) {
        // 用途：将源错误转换为字符串，再赋值给当前错误
        // 说明：实现简单且安全的克隆赋值
        *self = Self::from(source.to_string());
    }
}

// 用途：序列化实现注释
// 说明：说明这是 #[derive(Serialize)] 会生成的代码
impl Serialize for Error {
    /// 用途：序列化错误对象
    /// 说明：允许错误类型被序列化为JSON等格式
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 用途：将错误转换为字符串并序列化
        // 说明：错误信息最适合以字符串形式序列化
        serializer.serialize_str(self.to_string().as_str())
    }
}

/// 用途：错误访问者结构体
/// 说明：用于反序列化错误对象
struct ErrorVisitor;

/// 用途：实现Visitor trait
/// 说明：用于自定义错误类型的反序列化
impl<'de> Visitor<'de> for ErrorVisitor {
    /// 用途：定义访问者返回的类型
    /// 说明：反序列化过程中需要临时存储字符串
    type Value = String;

    /// 用途：期望的输入格式
    /// 说明：告知反序列化器期望接收字符串类型
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        // 用途：输出期望的格式
        // 说明：帮助调试反序列化错误
        formatter.write_str("a string")
    }

    /// 用途：处理字符串值
    /// 说明：当反序列化器遇到字符串时调用
    fn visit_string<E>(self, v: String) -> std::result::Result<Self::Value, E>
    where
        E: std::error::Error,
    {
        // 用途：返回接收到的字符串
        // 说明：将字符串作为反序列化的结果
        Ok(v)
    }

    /// 用途：处理字符串切片
    /// 说明：当反序列化器遇到字符串切片时调用
    fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
    where
        E: std::error::Error,
    {
        // 用途：将字符串切片转换为String并返回
        // 说明：统一处理字符串和字符串切片
        Ok(v.to_string())
    }
}

/// 用途：实现Deserialize trait
/// 说明：允许错误类型从JSON等格式反序列化
impl<'de> Deserialize<'de> for Error {
    /// 用途：反序列化错误对象
    /// 说明：将字符串形式的错误转换为自定义错误类型
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // 用途：使用访问者反序列化字符串
        // 说明：将JSON字符串转换为Error类型
        let r = deserializer.deserialize_string(ErrorVisitor)?;
        // 用途：创建自定义错误对象
        // 说明：将反序列化得到的字符串转换为Error类型
        return Ok(Error::from(r));
    }
}

/// 用途：统一的应用程序错误类型
/// 说明：提供更详细的错误分类和上下文信息
#[derive(Debug, Clone)]
pub enum ApplicationError {
    /// 配置相关错误
    ConfigError { message: String, key: Option<String> },
    /// 加密相关错误
    EncryptionError { message: String, operation: Option<String> },
    /// 业务逻辑错误
    BusinessError { message: String, code: Option<String>, context: Option<String> },
    /// 认证授权错误
    AuthError { message: String, user_id: Option<String>, operation: Option<String> },
    /// 外部服务错误
    ExternalServiceError { message: String, service: Option<String>, endpoint: Option<String> },
    /// 验证错误
    ValidationError { message: String, field: Option<String>, value: Option<String> },
    /// 限流错误
    RateLimitExceeded { message: String, limit_type: Option<String>, reset_time: Option<String> },
    /// 数据库错误
    DatabaseError { message: String, operation: Option<String>, table: Option<String> },
    /// 资源未找到错误
    NotFound { message: String, resource: Option<String>, id: Option<String> },
    /// 配额不足错误
    QuotaExceeded { message: String, user_id: Option<String>, required: Option<f64>, remaining: Option<f64> },
    /// 账单相关错误
    BillingError { message: String, bill_id: Option<String>, user_id: Option<String> },
    /// 价格规则错误
    PriceRuleError { message: String, rule_id: Option<String> },
    /// JWT令牌错误
    TokenError { message: String, kind: Option<String> },
    /// 存储服务错误
    StorageError { message: String, operation: Option<String>, bucket: Option<String> },
    /// 通用错误（向后兼容）
    GenericError { message: String },
}

/// 用途：实现Display trait for ApplicationError
/// 说明：允许ApplicationError以友好的方式显示给用户
impl Display for ApplicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApplicationError::ConfigError { message, key } => {
                if let Some(k) = key {
                    write!(f, "配置错误 [{}]: {}", k, message)
                } else {
                    write!(f, "配置错误: {}", message)
                }
            }
            ApplicationError::EncryptionError { message, operation } => {
                if let Some(op) = operation {
                    write!(f, "加密错误 [{}]: {}", op, message)
                } else {
                    write!(f, "加密错误: {}", message)
                }
            }
            ApplicationError::BusinessError { message, code, context } => {
                match (code, context) {
                    (Some(c), Some(ctx)) => write!(f, "业务错误 [{}]: {} (上下文: {})", c, message, ctx),
                    (Some(c), None) => write!(f, "业务错误 [{}]: {}", c, message),
                    (None, Some(ctx)) => write!(f, "业务错误: {} (上下文: {})", message, ctx),
                    (None, None) => write!(f, "业务错误: {}", message),
                }
            }
            ApplicationError::AuthError { message, user_id, operation } => {
                match (user_id, operation) {
                    (Some(uid), Some(op)) => write!(f, "认证错误 [用户: {}, 操作: {}]: {}", uid, op, message),
                    (Some(uid), None) => write!(f, "认证错误 [用户: {}]: {}", uid, message),
                    (None, Some(op)) => write!(f, "认证错误 [操作: {}]: {}", op, message),
                    (None, None) => write!(f, "认证错误: {}", message),
                }
            }
            ApplicationError::ExternalServiceError { message, service, endpoint } => {
                match (service, endpoint) {
                    (Some(s), Some(e)) => write!(f, "外部服务错误 [{}@{}]: {}", s, e, message),
                    (Some(s), None) => write!(f, "外部服务错误 [{}]: {}", s, message),
                    (None, Some(e)) => write!(f, "外部服务错误 [端点: {}]: {}", e, message),
                    (None, None) => write!(f, "外部服务错误: {}", message),
                }
            }
            ApplicationError::ValidationError { message, field, value } => {
                match (field, value) {
                    (Some(fld), Some(val)) => write!(f, "验证错误 [字段: {}, 值: {}]: {}", fld, val, message),
                    (Some(fld), None) => write!(f, "验证错误 [字段: {}]: {}", fld, message),
                    (None, Some(val)) => write!(f, "验证错误 [值: {}]: {}", val, message),
                    (None, None) => write!(f, "验证错误: {}", message),
                }
            }
            ApplicationError::RateLimitExceeded { message, limit_type, reset_time } => {
                match (limit_type, reset_time) {
                    (Some(lt), Some(rt)) => write!(f, "限流错误 [类型: {}, 重置时间: {}]: {}", lt, rt, message),
                    (Some(lt), None) => write!(f, "限流错误 [类型: {}]: {}", lt, message),
                    (None, Some(rt)) => write!(f, "限流错误 [重置时间: {}]: {}", rt, message),
                    (None, None) => write!(f, "限流错误: {}", message),
                }
            }
            ApplicationError::DatabaseError { message, operation, table } => {
                match (operation, table) {
                    (Some(op), Some(tbl)) => write!(f, "数据库错误 [操作: {}, 表: {}]: {}", op, tbl, message),
                    (Some(op), None) => write!(f, "数据库错误 [操作: {}]: {}", op, message),
                    (None, Some(tbl)) => write!(f, "数据库错误 [表: {}]: {}", tbl, message),
                    (None, None) => write!(f, "数据库错误: {}", message),
                }
            }
            ApplicationError::NotFound { message, resource, id } => {
                match (resource, id) {
                    (Some(res), Some(i)) => write!(f, "资源未找到 [资源: {}, ID: {}]: {}", res, i, message),
                    (Some(res), None) => write!(f, "资源未找到 [资源: {}]: {}", res, message),
                    (None, Some(i)) => write!(f, "资源未找到 [ID: {}]: {}", i, message),
                    (None, None) => write!(f, "资源未找到: {}", message),
                }
            }
            ApplicationError::QuotaExceeded { message, user_id, required, remaining } => {
                match (user_id, required, remaining) {
                    (Some(uid), Some(req), Some(rem)) => {
                        write!(f, "配额不足 [用户: {}, 需要: {:.2}, 剩余: {:.2}]: {}", uid, req, rem, message)
                    }
                    (Some(uid), Some(req), None) => {
                        write!(f, "配额不足 [用户: {}, 需要: {:.2}]: {}", uid, req, message)
                    }
                    (Some(uid), None, Some(rem)) => {
                        write!(f, "配额不足 [用户: {}, 剩余: {:.2}]: {}", uid, rem, message)
                    }
                    (None, Some(req), Some(rem)) => {
                        write!(f, "配额不足 [需要: {:.2}, 剩余: {:.2}]: {}", req, rem, message)
                    }
                    _ => write!(f, "配额不足: {}", message),
                }
            }
            ApplicationError::BillingError { message, bill_id, user_id } => {
                match (bill_id, user_id) {
                    (Some(bid), Some(uid)) => write!(f, "账单错误 [账单: {}, 用户: {}]: {}", bid, uid, message),
                    (Some(bid), None) => write!(f, "账单错误 [账单: {}]: {}", bid, message),
                    (None, Some(uid)) => write!(f, "账单错误 [用户: {}]: {}", uid, message),
                    (None, None) => write!(f, "账单错误: {}", message),
                }
            }
            ApplicationError::PriceRuleError { message, rule_id } => {
                if let Some(rid) = rule_id {
                    write!(f, "价格规则错误 [规则: {}]: {}", rid, message)
                } else {
                    write!(f, "价格规则错误: {}", message)
                }
            }
            ApplicationError::TokenError { message, kind } => {
                if let Some(k) = kind {
                    write!(f, "令牌错误 [类型: {}]: {}", k, message)
                } else {
                    write!(f, "令牌错误: {}", message)
                }
            }
            ApplicationError::StorageError { message, operation, bucket } => {
                match (operation, bucket) {
                    (Some(op), Some(bkt)) => write!(f, "存储错误 [操作: {}, 桶: {}]: {}", op, bkt, message),
                    (Some(op), None) => write!(f, "存储错误 [操作: {}]: {}", op, message),
                    (None, Some(bkt)) => write!(f, "存储错误 [桶: {}]: {}", bkt, message),
                    (None, None) => write!(f, "存储错误: {}", message),
                }
            }
            ApplicationError::GenericError { message } => {
                write!(f, "{}", message)
            }
        }
    }
}

/// 用途：实现标准错误 trait for ApplicationError
/// 说明：允许ApplicationError与标准库错误处理兼容
impl StdError for ApplicationError {}

/// 用途：实现Axum的IntoResponse trait for ApplicationError
/// 说明：允许ApplicationError类型直接作为HTTP响应返回
impl axum::response::IntoResponse for ApplicationError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::Json;
        use serde_json::json;
        
        // 根据错误类型确定HTTP状态码
        let status_code = match self {
            ApplicationError::BusinessError { .. } => StatusCode::BAD_REQUEST,
            ApplicationError::AuthError { .. } => StatusCode::UNAUTHORIZED,
            ApplicationError::NotFound { .. } => StatusCode::NOT_FOUND,
            ApplicationError::ValidationError { .. } => StatusCode::BAD_REQUEST,
            ApplicationError::RateLimitExceeded { .. } => StatusCode::TOO_MANY_REQUESTS,
            ApplicationError::DatabaseError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApplicationError::EncryptionError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApplicationError::ExternalServiceError { .. } => StatusCode::BAD_GATEWAY,
            ApplicationError::ConfigError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApplicationError::QuotaExceeded { .. } => StatusCode::PAYMENT_REQUIRED,
            ApplicationError::BillingError { .. } => StatusCode::BAD_REQUEST,
            ApplicationError::PriceRuleError { .. } => StatusCode::BAD_REQUEST,
            ApplicationError::TokenError { .. } => StatusCode::UNAUTHORIZED,
            ApplicationError::StorageError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApplicationError::GenericError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        };
        
        let response = json!({
            "success": false,
            "code": status_code.as_u16().to_string(),
            "message": self.to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        
        (status_code, Json(response)).into_response()
    }
}

/// 用途：实现From trait for ApplicationError到Error的转换
/// 说明：允许ApplicationError转换为现有的Error类型，保持向后兼容
impl From<ApplicationError> for Error {
    fn from(err: ApplicationError) -> Self {
        match err {
            ApplicationError::ConfigError { message, .. } => Error::ConfigError(message),
            ApplicationError::EncryptionError { message, .. } => Error::EncryptionError(message),
            ApplicationError::BusinessError { message, .. } => Error::BusinessError(message),
            ApplicationError::AuthError { message, .. } => Error::AuthError(message),
            ApplicationError::ExternalServiceError { message, .. } => Error::ExternalServiceError(message),
            ApplicationError::ValidationError { message, .. } => Error::ValidationError(message),
            ApplicationError::RateLimitExceeded { .. } => Error::RateLimitExceeded,
            ApplicationError::DatabaseError { message, .. } => Error::DatabaseError(message),
            ApplicationError::NotFound { message, .. } => Error::NotFound(message),
            ApplicationError::GenericError { message } => Error::E(message),
            // 对于新的错误类型，转换为合适的Error类型
            ApplicationError::QuotaExceeded { message, .. } => Error::BusinessError(message),
            ApplicationError::BillingError { message, .. } => Error::BusinessError(message),
            ApplicationError::PriceRuleError { message, .. } => Error::BusinessError(message),
            ApplicationError::TokenError { message, .. } => Error::AuthError(message),
            ApplicationError::StorageError { message, .. } => Error::ExternalServiceError(message),
        }
    }
}

/// 用途：实现From trait for Error到ApplicationError的转换
/// 说明：允许Error类型转换为ApplicationError，便于统一处理
impl From<Error> for ApplicationError {
    fn from(err: Error) -> Self {
        match err {
            Error::ConfigError(msg) => ApplicationError::ConfigError { message: msg, key: None },
            Error::EncryptionError(msg) => ApplicationError::EncryptionError { message: msg, operation: None },
            Error::BusinessError(msg) => ApplicationError::BusinessError { message: msg, code: None, context: None },
            Error::AuthError(msg) => ApplicationError::AuthError { message: msg, user_id: None, operation: None },
            Error::ExternalServiceError(msg) => ApplicationError::ExternalServiceError { message: msg, service: None, endpoint: None },
            Error::ValidationError(msg) => ApplicationError::ValidationError { message: msg, field: None, value: None },
            Error::RateLimitExceeded => ApplicationError::RateLimitExceeded { message: "Rate limit exceeded".to_string(), limit_type: None, reset_time: None },
            Error::DatabaseError(msg) => ApplicationError::DatabaseError { message: msg, operation: None, table: None },
            Error::NotFound(msg) => ApplicationError::NotFound { message: msg, resource: None, id: None },
            Error::E(msg) => ApplicationError::GenericError { message: msg },
        }
    }
}

/// 用途：实现From trait for String到ApplicationError的转换
/// 说明：方便直接使用字符串创建ApplicationError
impl From<String> for ApplicationError {
    fn from(msg: String) -> Self {
        ApplicationError::GenericError { message: msg }
    }
}

/// 用途：实现From trait for &str到ApplicationError的转换
/// 说明：方便直接使用字符串字面量创建ApplicationError
impl From<&str> for ApplicationError {
    fn from(msg: &str) -> Self {
        ApplicationError::GenericError { message: msg.to_string() }
    }
}

/// 用途：实现From trait for rbatis::Error到ApplicationError的转换
/// 说明：统一处理数据库操作错误
impl From<rbatis::Error> for ApplicationError {
    fn from(err: rbatis::Error) -> Self {
        ApplicationError::DatabaseError {
            message: err.to_string(),
            operation: None,
            table: None,
        }
    }
}

/// 用途：实现From trait for io::Error到ApplicationError的转换
/// 说明：统一处理IO操作错误
impl From<io::Error> for ApplicationError {
    fn from(err: io::Error) -> Self {
        ApplicationError::StorageError {
            message: err.to_string(),
            operation: None,
            bucket: None,
        }
    }
}

/// 用途：实现From trait for ApplicationError到io::Error的转换
/// 说明：允许ApplicationError在需要io::Error的上下文中使用
impl From<ApplicationError> for io::Error {
    fn from(err: ApplicationError) -> Self {
        io::Error::new(io::ErrorKind::Other, err.to_string())
    }
}

/// 用途：实现From trait for &dyn std::error::Error到ApplicationError的转换
/// 说明：统一处理各种标准错误类型
impl From<&dyn std::error::Error> for ApplicationError {
    fn from(arg: &dyn std::error::Error) -> Self {
        ApplicationError::GenericError { message: arg.to_string() }
    }
}

/// 用途：定义ApplicationError的结果类型别名
/// 说明：简化错误处理，统一使用ApplicationError类型
pub type ApplicationResult<T> = std::result::Result<T, ApplicationError>;