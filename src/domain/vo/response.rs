//! 统一响应视图对象模块。
//! 提供标准化的 API 响应格式，用于前后端接口对接。

// 用途：导入序列化支持
// 说明：用于响应结构的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI Schema支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;
// 用途：导入Axum响应支持
// 说明：用于将ApiResponse转换为HTTP响应
use axum::{Json, response::IntoResponse};
// 用途：导入DateTime类型
// 说明：用于响应时间戳
use rbatis::rbdc::DateTime;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = "2024-01-01T00:00:00Z")]
pub struct DateTimeWrapper(pub String);

// 用途：标准API响应结构体
// 说明：所有API接口的统一响应格式
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    // 用途：请求是否成功
    // 说明：true表示成功，false表示失败
    pub success: bool,
    // 用途：响应码
    // 说明："0"表示成功，其他值表示错误码
    pub code: String,
    // 用途：响应消息
    // 说明：成功时为null，失败时包含错误描述
    pub message: Option<String>,
    // 用途：响应数据
    // 说明：成功时包含业务数据，失败时为null
    pub data: Option<T>,
    // 用途：时间戳
    // 说明：响应生成的时间，格式为RFC3339
    pub timestamp: Option<DateTimeWrapper>,
}

// 用途：实现ApiResponse的构造函数
// 说明：提供便捷的响应创建方法
impl<T> ApiResponse<T> {
    // 用途：创建成功响应
    // 说明：用于返回成功的API响应
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            code: "0".to_string(),
            message: None,
            data: Some(data),
            timestamp: Some(DateTimeWrapper(DateTime::now().to_string())),
        }
    }

    // 用途：创建无数据的成功响应
    // 说明：用于不需要返回数据的成功操作
    pub fn success_empty() -> Self {
        Self {
            success: true,
            code: "0".to_string(),
            message: None,
            data: None,
            timestamp: Some(DateTimeWrapper(DateTime::now().to_string())),
        }
    }

    // 用途：创建带数据的错误响应
    // 说明：用于需要返回错误码且同时携带业务数据的场景
    pub fn error_with_data(code: &str, message: &str, data: T) -> Self {
        Self {
            success: false,
            code: code.to_string(),
            message: Some(message.to_string()),
            data: Some(data),
            timestamp: Some(DateTimeWrapper(DateTime::now().to_string())),
        }
    }

    // 用途：创建错误响应
    // 说明：用于返回失败的API响应
    pub fn error(code: &str, message: &str) -> Self {
        Self {
            success: false,
            code: code.to_string(),
            message: Some(message.to_string()),
            data: None,
            timestamp: Some(DateTimeWrapper(DateTime::now().to_string())),
        }
    }
}

// 用途：为ApiResponse实现IntoResponse trait
// 说明：使ApiResponse可以直接作为Axum的HTTP响应返回
impl<T: Serialize + Send + Sync> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}

// 用途：分页响应结构体
// 说明：用于返回分页数据的响应格式
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PageResponse<T> {
    // 用途：当前页码
    // 说明：从1开始的页码
    pub page: u32,
    // 用途：每页大小
    // 说明：每页包含的数据条数
    pub page_size: u32,
    // 用途：总记录数
    // 说明：符合查询条件的总数据量
    pub total: u64,
    // 用途：总页数
    // 说明：根据总记录数和每页大小计算得出
    pub total_pages: u32,
    // 用途：数据列表
    // 说明：当前页的数据集合
    pub items: Vec<T>,
}

// 用途：实现PageResponse的构造函数
// 说明：提供便捷的分页响应创建方法
impl<T> PageResponse<T> {
    // 用途：创建分页响应
    // 说明：用于返回分页数据
    pub fn new(items: Vec<T>, total: u64, page: u32, page_size: u32) -> Self {
        let total_pages = if page_size > 0 {
            ((total as f64) / (page_size as f64)).ceil() as u32
        } else {
            0
        };

        Self {
            page,
            page_size,
            total,
            total_pages,
            items,
        }
    }
}

// 用途：错误响应结构体
// 说明：用于返回详细的错误信息
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    // 用途：请求是否成功
    // 说明：固定为false
    pub success: bool,
    // 用途：错误码
    // 说明：标准错误码或业务错误码
    pub code: String,
    // 用途：错误消息
    // 说明：错误的详细描述
    pub message: String,
    // 用途：错误详情
    // 说明：可选的额外错误信息
    pub details: Option<serde_json::Value>,
    // 用途：时间戳
    // 说明：错误发生的时间，格式为RFC3339
    pub timestamp: Option<DateTimeWrapper>,
}

// 用途：实现ErrorResponse的构造函数
// 说明：提供便捷的错误响应创建方法
impl ErrorResponse {
    // 用途：创建错误响应
    // 说明：用于返回详细的错误信息
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            success: false,
            code: code.to_string(),
            message: message.to_string(),
            details: None,
            timestamp: Some(DateTimeWrapper(DateTime::now().to_string())),
        }
    }

    // 用途：创建带详情的错误响应
    // 说明：用于返回包含额外信息的错误
    pub fn with_details(code: &str, message: &str, details: serde_json::Value) -> Self {
        Self {
            success: false,
            code: code.to_string(),
            message: message.to_string(),
            details: Some(details),
            timestamp: Some(DateTimeWrapper(DateTime::now().to_string())),
        }
    }
}

// 用途：rbatis Page的包装类型
// 说明：用于支持OpenAPI文档生成，避免orphan rule限制
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PageWrapper<T> {
    // 用途：当前页码
    // 说明：从1开始的页码
    pub page: u64,
    // 用途：每页大小
    // 说明：每页包含的数据条数
    pub page_size: u64,
    // 用途：总记录数
    // 说明：符合查询条件的总数据量
    pub total: u64,
    // 用途：数据列表
    // 说明：当前页的数据集合
    pub records: Vec<T>,
}
