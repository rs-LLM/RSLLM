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

/// 用途：标准错误码常量
/// 说明：定义HTTP标准状态码对应的错误码
pub mod error_codes {
    // 用途：成功
    pub const SUCCESS: &str = "0";

    // 用途：客户端错误 (4xx)
    pub const BAD_REQUEST: &str = "400";
    pub const UNAUTHORIZED: &str = "401";
    pub const PAYMENT_REQUIRED: &str = "402";
    pub const FORBIDDEN: &str = "403";
    pub const NOT_FOUND: &str = "404";
    pub const METHOD_NOT_ALLOWED: &str = "405";
    pub const CONFLICT: &str = "409";
    pub const UNPROCESSABLE_ENTITY: &str = "422";
    pub const TOO_MANY_REQUESTS: &str = "429";

    // 用途：服务器错误 (5xx)
    pub const INTERNAL_SERVER_ERROR: &str = "500";
    pub const BAD_GATEWAY: &str = "502";
    pub const SERVICE_UNAVAILABLE: &str = "503";

    // 用途：认证错误码 (10000-10999)
    pub const AUTH_INVALID_TOKEN: &str = "10001";
    pub const AUTH_TOKEN_EXPIRED: &str = "10002";
    pub const AUTH_TOKEN_INVALID: &str = "10003";
    pub const AUTH_UNAUTHORIZED: &str = "10004";
    pub const AUTH_FORBIDDEN: &str = "10005";

    // 用途：验证错误码 (11000-11999)
    pub const VALIDATION_INVALID_PARAM: &str = "11001";
    pub const VALIDATION_MISSING_PARAM: &str = "11002";
    pub const VALIDATION_INVALID_FORMAT: &str = "11003";
    pub const VALIDATION_OUT_OF_RANGE: &str = "11004";

    // 用途：资源未找到错误码 (12000-12999)
    pub const NOT_FOUND_RESOURCE: &str = "12001";
    pub const NOT_FOUND_USER: &str = "12002";
    pub const NOT_FOUND_PROVIDER: &str = "12003";
    pub const NOT_FOUND_BILL: &str = "12004";
    pub const NOT_FOUND_PRICE_RULE: &str = "12005";

    // 用途：限流错误码 (13000-13999)
    pub const RATE_LIMIT_EXCEEDED: &str = "13001";
    pub const RATE_LIMIT_API: &str = "13002";
    pub const RATE_LIMIT_USER: &str = "13003";

    // 用途：服务端错误码 (20000-20999)
    pub const SERVER_INTERNAL_ERROR: &str = "20001";
    pub const SERVER_CONFIG_ERROR: &str = "20002";
    pub const SERVER_ENCRYPTION_ERROR: &str = "20003";

    // 用途：数据库错误码 (21000-21999)
    pub const DATABASE_CONNECTION_ERROR: &str = "21001";
    pub const DATABASE_QUERY_ERROR: &str = "21002";
    pub const DATABASE_TRANSACTION_ERROR: &str = "21003";
    pub const DATABASE_DUPLICATE_KEY: &str = "21004";

    // 用途：外部服务错误码 (22000-22999)
    pub const EXTERNAL_SERVICE_ERROR: &str = "22001";
    pub const EXTERNAL_SERVICE_TIMEOUT: &str = "22002";
    pub const EXTERNAL_SERVICE_UNAVAILABLE: &str = "22003";

    // 用途：业务错误码 (30000-30999)
    pub const BUSINESS_INVALID_OPERATION: &str = "30001";
    pub const BUSINESS_INVALID_STATE: &str = "30002";
    pub const BUSINESS_QUOTA_EXCEEDED: &str = "30003";
    pub const BUSINESS_BILLING_ERROR: &str = "30004";
    pub const BUSINESS_PRICE_RULE_ERROR: &str = "30005";
    pub const BUSINESS_STORAGE_ERROR: &str = "30006";

    // 用途：通用错误
    pub const UNKNOWN_ERROR: &str = "99999";
}

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
    /// 用途：应用错误
    /// 说明：用于包装ApplicationError
    Application(ApplicationError),
    /// 用途：限流错误
    /// 说明：用于速率限制错误
    RateLimitExceeded,
    /// 用途：数据库错误
    /// 说明：用于数据库操作错误
    DatabaseError(String),
    /// 用途：未找到错误
    /// 说明：用于资源不存在的错误
    NotFound(String),
    /// 用途：规则数量超限
    /// 说明：用于规则数量超过限制的错误
    TooManyRules(usize),
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
            Error::Application(error) => write!(f, "Application Error: {}", error),
            Error::RateLimitExceeded => write!(f, "Rate Limit Exceeded"),
            Error::DatabaseError(error) => write!(f, "Database Error: {}", error),
            Error::NotFound(error) => write!(f, "Not Found: {}", error),
            Error::TooManyRules(limit) => {
                write!(f, "规则数量超限，最大允许: {}", limit)
            }
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
        use crate::domain::vo::ApiResponse;
        use crate::error::error_codes::*;
        use axum::Json;
        use axum::http::StatusCode;

        let (status_code, error_code) = match &self {
            Error::AuthError(msg) => {
                let code = if msg.contains("token") || msg.contains("Token") {
                    if msg.contains("expired") || msg.contains("过期") {
                        AUTH_TOKEN_EXPIRED
                    } else {
                        AUTH_INVALID_TOKEN
                    }
                } else if msg.contains("forbidden") || msg.contains("禁止") {
                    AUTH_FORBIDDEN
                } else {
                    AUTH_UNAUTHORIZED
                };
                (StatusCode::UNAUTHORIZED, code)
            }
            Error::ValidationError(msg) => {
                let code = if msg.contains("missing") || msg.contains("缺失") {
                    VALIDATION_MISSING_PARAM
                } else if msg.contains("format") || msg.contains("格式") {
                    VALIDATION_INVALID_FORMAT
                } else if msg.contains("range") || msg.contains("范围") {
                    VALIDATION_OUT_OF_RANGE
                } else {
                    VALIDATION_INVALID_PARAM
                };
                (StatusCode::BAD_REQUEST, code)
            }
            Error::NotFound(msg) => {
                let code = if msg.contains("user") || msg.contains("用户") {
                    NOT_FOUND_USER
                } else if msg.contains("provider") || msg.contains("供应商") {
                    NOT_FOUND_PROVIDER
                } else if msg.contains("bill") || msg.contains("账单") {
                    NOT_FOUND_BILL
                } else if msg.contains("price") || msg.contains("价格") {
                    NOT_FOUND_PRICE_RULE
                } else {
                    NOT_FOUND_RESOURCE
                };
                (StatusCode::NOT_FOUND, code)
            }
            Error::RateLimitExceeded => (StatusCode::TOO_MANY_REQUESTS, RATE_LIMIT_EXCEEDED),
            Error::TooManyRules(_) => (StatusCode::BAD_REQUEST, BUSINESS_INVALID_OPERATION),
            Error::BusinessError(msg) => {
                let code = if msg.contains("quota") || msg.contains("配额") {
                    BUSINESS_QUOTA_EXCEEDED
                } else if msg.contains("bill") || msg.contains("账单") {
                    BUSINESS_BILLING_ERROR
                } else if msg.contains("price") || msg.contains("价格") {
                    BUSINESS_PRICE_RULE_ERROR
                } else if msg.contains("storage") || msg.contains("存储") {
                    BUSINESS_STORAGE_ERROR
                } else {
                    BUSINESS_INVALID_OPERATION
                };
                (StatusCode::BAD_REQUEST, code)
            }
            Error::DatabaseError(msg) => {
                let code = if msg.contains("connection") || msg.contains("连接") {
                    DATABASE_CONNECTION_ERROR
                } else if msg.contains("transaction") || msg.contains("事务") {
                    DATABASE_TRANSACTION_ERROR
                } else if msg.contains("duplicate") || msg.contains("重复") {
                    DATABASE_DUPLICATE_KEY
                } else {
                    DATABASE_QUERY_ERROR
                };
                (StatusCode::INTERNAL_SERVER_ERROR, code)
            }
            Error::EncryptionError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, SERVER_ENCRYPTION_ERROR)
            }
            Error::ExternalServiceError(msg) => {
                let code = if msg.contains("timeout") || msg.contains("超时") {
                    EXTERNAL_SERVICE_TIMEOUT
                } else if msg.contains("unavailable") || msg.contains("不可用") {
                    EXTERNAL_SERVICE_UNAVAILABLE
                } else {
                    EXTERNAL_SERVICE_ERROR
                };
                (StatusCode::BAD_GATEWAY, code)
            }
            Error::Application(_) => (StatusCode::INTERNAL_SERVER_ERROR, SERVER_INTERNAL_ERROR),
            Error::ConfigError(_) => (StatusCode::INTERNAL_SERVER_ERROR, SERVER_CONFIG_ERROR),
            Error::E(_) => (StatusCode::INTERNAL_SERVER_ERROR, SERVER_INTERNAL_ERROR),
        };

        let public_message: &str = match &self {
            Error::AuthError(_) => "未授权，请重新登录",
            Error::NotFound(msg) => msg,
            Error::ValidationError(msg) => msg,
            Error::BusinessError(msg) => msg,
            Error::TooManyRules(_) => "请求参数不合法",
            Error::RateLimitExceeded => "请求过于频繁，请稍后再试",
            Error::DatabaseError(_) => "数据库错误，请稍后重试",
            Error::ExternalServiceError(_) => "外部服务异常，请稍后重试",
            Error::ConfigError(_) | Error::EncryptionError(_) => "服务内部错误",
            Error::Application(_) | Error::E(_) => "服务器内部错误",
        };

        match &self {
            Error::DatabaseError(e)
            | Error::ExternalServiceError(e)
            | Error::ConfigError(e)
            | Error::EncryptionError(e)
            | Error::E(e) => {
                log::error!("[error] {}", e);
            }
            Error::Application(e) => {
                log::error!("[error] {}", e);
            }
            _ => {}
        }

        let response = ApiResponse::<()>::error(error_code, public_message);

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
        Error::E(arg.to_string())
    }
}

/// 用途：实现从String到自定义错误的转换
/// 说明：方便使用String创建错误
impl From<std::string::String> for Error {
    fn from(arg: String) -> Self {
        // 用途：将String转换为自定义错误
        // 说明：允许直接使用String对象创建错误
        Error::E(arg)
    }
}

/// 用途：实现从通用错误类型到自定义错误的转换
/// 说明：方便处理各种实现了StdError的错误类型
impl From<&dyn std::error::Error> for Error {
    fn from(arg: &dyn std::error::Error) -> Self {
        // 用途：将通用错误转换为字符串，再转换为自定义错误
        // 说明：统一处理各种类型的错误
        Error::E(arg.to_string())
    }
}

/// 用途：实现从自定义错误到IO错误的转换
/// 说明：方便在需要IO错误的上下文中使用自定义错误
impl From<Error> for std::io::Error {
    fn from(arg: Error) -> Self {
        // 用途：将自定义错误转换为IO错误
        // 说明：允许在需要IO错误的函数中返回自定义错误
        io::Error::other(arg.to_string())
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
        Ok(Error::from(r))
    }
}

/// 用途：统一的应用程序错误类型
/// 说明：提供更详细的错误分类和上下文信息
#[derive(Debug, Clone)]
pub enum ApplicationError {
    /// 配置相关错误
    ConfigError {
        message: String,
        key: Option<String>,
    },
    /// 加密相关错误
    EncryptionError {
        message: String,
        operation: Option<String>,
    },
    /// 业务逻辑错误
    BusinessError {
        message: String,
        code: Option<String>,
        context: Option<String>,
    },
    /// 认证授权错误
    AuthError {
        message: String,
        user_id: Option<String>,
        operation: Option<String>,
    },
    /// 外部服务错误
    ExternalServiceError {
        message: String,
        service: Option<String>,
        endpoint: Option<String>,
    },
    /// 验证错误
    ValidationError {
        message: String,
        field: Option<String>,
        value: Option<String>,
    },
    /// 限流错误
    RateLimitExceeded {
        message: String,
        limit_type: Option<String>,
        reset_time: Option<String>,
    },
    /// 数据库错误
    DatabaseError {
        message: String,
        operation: Option<String>,
        table: Option<String>,
    },
    /// 资源未找到错误
    NotFound {
        message: String,
        resource: Option<String>,
        id: Option<String>,
    },
    /// 配额不足错误
    QuotaExceeded {
        message: String,
        user_id: Option<String>,
        required: Option<f64>,
        remaining: Option<f64>,
    },
    /// 余额不足错误
    BalanceExceeded {
        message: String,
        user_id: Option<String>,
        required: Option<f64>,
        remaining: Option<f64>,
    },
    /// 账单相关错误
    BillingError {
        message: String,
        bill_id: Option<String>,
        user_id: Option<String>,
    },
    /// 价格规则错误
    PriceRuleError {
        message: String,
        rule_id: Option<String>,
    },
    /// JWT令牌错误
    TokenError {
        message: String,
        kind: Option<String>,
    },
    /// 存储服务错误
    StorageError {
        message: String,
        operation: Option<String>,
        bucket: Option<String>,
    },
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
            ApplicationError::BusinessError {
                message,
                code,
                context,
            } => match (code, context) {
                (Some(c), Some(ctx)) => {
                    write!(f, "业务错误 [{}]: {} (上下文: {})", c, message, ctx)
                }
                (Some(c), None) => write!(f, "业务错误 [{}]: {}", c, message),
                (None, Some(ctx)) => write!(f, "业务错误: {} (上下文: {})", message, ctx),
                (None, None) => write!(f, "业务错误: {}", message),
            },
            ApplicationError::AuthError {
                message,
                user_id,
                operation,
            } => match (user_id, operation) {
                (Some(uid), Some(op)) => {
                    write!(f, "认证错误 [用户: {}, 操作: {}]: {}", uid, op, message)
                }
                (Some(uid), None) => write!(f, "认证错误 [用户: {}]: {}", uid, message),
                (None, Some(op)) => write!(f, "认证错误 [操作: {}]: {}", op, message),
                (None, None) => write!(f, "认证错误: {}", message),
            },
            ApplicationError::ExternalServiceError {
                message,
                service,
                endpoint,
            } => match (service, endpoint) {
                (Some(s), Some(e)) => write!(f, "外部服务错误 [{}@{}]: {}", s, e, message),
                (Some(s), None) => write!(f, "外部服务错误 [{}]: {}", s, message),
                (None, Some(e)) => write!(f, "外部服务错误 [端点: {}]: {}", e, message),
                (None, None) => write!(f, "外部服务错误: {}", message),
            },
            ApplicationError::ValidationError {
                message,
                field,
                value,
            } => match (field, value) {
                (Some(fld), Some(val)) => {
                    write!(f, "验证错误 [字段: {}, 值: {}]: {}", fld, val, message)
                }
                (Some(fld), None) => write!(f, "验证错误 [字段: {}]: {}", fld, message),
                (None, Some(val)) => write!(f, "验证错误 [值: {}]: {}", val, message),
                (None, None) => write!(f, "验证错误: {}", message),
            },
            ApplicationError::RateLimitExceeded {
                message,
                limit_type,
                reset_time,
            } => match (limit_type, reset_time) {
                (Some(lt), Some(rt)) => {
                    write!(f, "限流错误 [类型: {}, 重置时间: {}]: {}", lt, rt, message)
                }
                (Some(lt), None) => write!(f, "限流错误 [类型: {}]: {}", lt, message),
                (None, Some(rt)) => write!(f, "限流错误 [重置时间: {}]: {}", rt, message),
                (None, None) => write!(f, "限流错误: {}", message),
            },
            ApplicationError::DatabaseError {
                message,
                operation,
                table,
            } => match (operation, table) {
                (Some(op), Some(tbl)) => {
                    write!(f, "数据库错误 [操作: {}, 表: {}]: {}", op, tbl, message)
                }
                (Some(op), None) => write!(f, "数据库错误 [操作: {}]: {}", op, message),
                (None, Some(tbl)) => write!(f, "数据库错误 [表: {}]: {}", tbl, message),
                (None, None) => write!(f, "数据库错误: {}", message),
            },
            ApplicationError::NotFound {
                message,
                resource,
                id,
            } => match (resource, id) {
                (Some(res), Some(i)) => {
                    write!(f, "资源未找到 [资源: {}, ID: {}]: {}", res, i, message)
                }
                (Some(res), None) => write!(f, "资源未找到 [资源: {}]: {}", res, message),
                (None, Some(i)) => write!(f, "资源未找到 [ID: {}]: {}", i, message),
                (None, None) => write!(f, "资源未找到: {}", message),
            },
            ApplicationError::QuotaExceeded {
                message,
                user_id,
                required,
                remaining,
            } => match (user_id, required, remaining) {
                (Some(uid), Some(req), Some(rem)) => {
                    write!(
                        f,
                        "配额不足 [用户: {}, 需要: {:.2}, 剩余: {:.2}]: {}",
                        uid, req, rem, message
                    )
                }
                (Some(uid), Some(req), None) => {
                    write!(f, "配额不足 [用户: {}, 需要: {:.2}]: {}", uid, req, message)
                }
                (Some(uid), None, Some(rem)) => {
                    write!(f, "配额不足 [用户: {}, 剩余: {:.2}]: {}", uid, rem, message)
                }
                (None, Some(req), Some(rem)) => {
                    write!(
                        f,
                        "配额不足 [需要: {:.2}, 剩余: {:.2}]: {}",
                        req, rem, message
                    )
                }
                _ => write!(f, "配额不足: {}", message),
            },
            ApplicationError::BalanceExceeded {
                message,
                user_id,
                required,
                remaining,
            } => match (user_id, required, remaining) {
                (Some(uid), Some(req), Some(rem)) => {
                    write!(
                        f,
                        "余额不足 [用户: {}, 需要: {:.2}, 剩余: {:.2}]: {}",
                        uid, req, rem, message
                    )
                }
                (Some(uid), Some(req), None) => {
                    write!(f, "余额不足 [用户: {}, 需要: {:.2}]: {}", uid, req, message)
                }
                (Some(uid), None, Some(rem)) => {
                    write!(f, "余额不足 [用户: {}, 剩余: {:.2}]: {}", uid, rem, message)
                }
                (None, Some(req), Some(rem)) => {
                    write!(
                        f,
                        "余额不足 [需要: {:.2}, 剩余: {:.2}]: {}",
                        req, rem, message
                    )
                }
                _ => write!(f, "余额不足: {}", message),
            },
            ApplicationError::BillingError {
                message,
                bill_id,
                user_id,
            } => match (bill_id, user_id) {
                (Some(bid), Some(uid)) => {
                    write!(f, "账单错误 [账单: {}, 用户: {}]: {}", bid, uid, message)
                }
                (Some(bid), None) => write!(f, "账单错误 [账单: {}]: {}", bid, message),
                (None, Some(uid)) => write!(f, "账单错误 [用户: {}]: {}", uid, message),
                (None, None) => write!(f, "账单错误: {}", message),
            },
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
            ApplicationError::StorageError {
                message,
                operation,
                bucket,
            } => match (operation, bucket) {
                (Some(op), Some(bkt)) => {
                    write!(f, "存储错误 [操作: {}, 桶: {}]: {}", op, bkt, message)
                }
                (Some(op), None) => write!(f, "存储错误 [操作: {}]: {}", op, message),
                (None, Some(bkt)) => write!(f, "存储错误 [桶: {}]: {}", bkt, message),
                (None, None) => write!(f, "存储错误: {}", message),
            },
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
        use crate::domain::vo::ApiResponse;
        use axum::Json;
        use axum::http::StatusCode;

        // 根据错误类型确定HTTP状态码和错误码
        let (status_code, error_code) = match &self {
            ApplicationError::BusinessError { code, .. } => {
                (StatusCode::BAD_REQUEST, code.as_deref().unwrap_or("400"))
            }
            ApplicationError::AuthError { .. } => (StatusCode::UNAUTHORIZED, "401"),
            ApplicationError::NotFound { .. } => (StatusCode::NOT_FOUND, "404"),
            ApplicationError::ValidationError { .. } => (StatusCode::BAD_REQUEST, "422"),
            ApplicationError::RateLimitExceeded { .. } => (StatusCode::TOO_MANY_REQUESTS, "429"),
            ApplicationError::DatabaseError { .. } => (StatusCode::INTERNAL_SERVER_ERROR, "500"),
            ApplicationError::EncryptionError { .. } => (StatusCode::INTERNAL_SERVER_ERROR, "500"),
            ApplicationError::ExternalServiceError { .. } => (StatusCode::BAD_GATEWAY, "502"),
            ApplicationError::ConfigError { .. } => (StatusCode::INTERNAL_SERVER_ERROR, "500"),
            ApplicationError::QuotaExceeded { .. } => (StatusCode::PAYMENT_REQUIRED, "402"),
            ApplicationError::BalanceExceeded { .. } => (StatusCode::PAYMENT_REQUIRED, "402"),
            ApplicationError::BillingError { .. } => (StatusCode::BAD_REQUEST, "400"),
            ApplicationError::PriceRuleError { .. } => (StatusCode::BAD_REQUEST, "400"),
            ApplicationError::TokenError { .. } => (StatusCode::UNAUTHORIZED, "401"),
            ApplicationError::StorageError { .. } => (StatusCode::INTERNAL_SERVER_ERROR, "500"),
            ApplicationError::GenericError { .. } => (StatusCode::INTERNAL_SERVER_ERROR, "500"),
        };

        match &self {
            ApplicationError::DatabaseError { message, .. }
            | ApplicationError::ExternalServiceError { message, .. }
            | ApplicationError::ConfigError { message, .. }
            | ApplicationError::EncryptionError { message, .. }
            | ApplicationError::StorageError { message, .. }
            | ApplicationError::GenericError { message }
            | ApplicationError::TokenError { message, .. } => {
                log::error!("[app-error] {}", message);
            }
            _ => {}
        }

        let public_message: &str = match &self {
            ApplicationError::AuthError { .. } => "未授权，请重新登录",
            ApplicationError::NotFound { message, .. }
            | ApplicationError::ValidationError { message, .. }
            | ApplicationError::BusinessError { message, .. }
            | ApplicationError::RateLimitExceeded { message, .. }
            | ApplicationError::QuotaExceeded { message, .. }
            | ApplicationError::BalanceExceeded { message, .. }
            | ApplicationError::BillingError { message, .. }
            | ApplicationError::PriceRuleError { message, .. } => message,
            ApplicationError::DatabaseError { .. } => "数据库错误，请稍后重试",
            ApplicationError::ExternalServiceError { .. } => "外部服务异常，请稍后重试",
            ApplicationError::ConfigError { .. } | ApplicationError::EncryptionError { .. } => {
                "服务内部错误"
            }
            ApplicationError::StorageError { .. } => "存储服务异常，请稍后重试",
            ApplicationError::TokenError { .. } => "令牌无效，请重新登录",
            ApplicationError::GenericError { .. } => "服务器内部错误",
        };

        let response = ApiResponse::<()>::error(error_code, public_message);

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
            ApplicationError::ExternalServiceError { message, .. } => {
                Error::ExternalServiceError(message)
            }
            ApplicationError::ValidationError { message, .. } => Error::ValidationError(message),
            ApplicationError::RateLimitExceeded { .. } => Error::RateLimitExceeded,
            ApplicationError::DatabaseError { message, .. } => Error::DatabaseError(message),
            ApplicationError::NotFound { message, .. } => Error::NotFound(message),
            ApplicationError::GenericError { message } => Error::E(message),
            // 对于新的错误类型，转换为合适的Error类型
            ApplicationError::QuotaExceeded { message, .. } => Error::BusinessError(message),
            ApplicationError::BalanceExceeded { message, .. } => Error::BusinessError(message),
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
    fn from(error: Error) -> Self {
        use crate::error::error_codes::*;
        match error {
            Error::ConfigError(msg) => ApplicationError::ConfigError {
                message: msg,
                key: None,
            },
            Error::EncryptionError(msg) => ApplicationError::EncryptionError {
                message: msg,
                operation: None,
            },
            Error::BusinessError(msg) => ApplicationError::BusinessError {
                message: msg,
                code: None,
                context: None,
            },
            Error::AuthError(msg) => ApplicationError::AuthError {
                message: msg,
                user_id: None,
                operation: None,
            },
            Error::ExternalServiceError(msg) => ApplicationError::ExternalServiceError {
                message: msg,
                service: None,
                endpoint: None,
            },
            Error::ValidationError(msg) => ApplicationError::ValidationError {
                message: msg,
                field: None,
                value: None,
            },
            Error::RateLimitExceeded => ApplicationError::RateLimitExceeded {
                message: "Rate limit exceeded".to_string(),
                limit_type: None,
                reset_time: None,
            },
            Error::DatabaseError(msg) => ApplicationError::DatabaseError {
                message: msg,
                operation: None,
                table: None,
            },
            Error::NotFound(msg) => ApplicationError::NotFound {
                message: msg,
                resource: None,
                id: None,
            },
            Error::Application(app_err) => app_err,
            Error::TooManyRules(limit) => ApplicationError::BusinessError {
                message: format!("规则数量超限，最大允许: {}", limit),
                code: Some(BUSINESS_INVALID_OPERATION.to_string()),
                context: None,
            },
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
        ApplicationError::GenericError {
            message: msg.to_string(),
        }
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
        io::Error::other(err.to_string())
    }
}

/// 用途：实现From trait for &dyn std::error::Error到ApplicationError的转换
/// 说明：统一处理各种标准错误类型
impl From<&dyn std::error::Error> for ApplicationError {
    fn from(arg: &dyn std::error::Error) -> Self {
        ApplicationError::GenericError {
            message: arg.to_string(),
        }
    }
}

/// 用途：定义ApplicationError的结果类型别名
/// 说明：简化错误处理，统一使用ApplicationError类型
pub type ApplicationResult<T> = std::result::Result<T, ApplicationError>;
