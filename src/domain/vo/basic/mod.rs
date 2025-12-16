// 用途：导出JWT相关VO模块
// 说明：允许其他模块访问JWT相关的VO
pub mod jwt;

// 用途：导出RBAC相关VO模块
// 说明：允许其他模块访问RBAC相关的VO
pub mod rbac;

// 用途：导出登录相关VO模块
// 说明：允许其他模块访问登录相关的VO
pub mod sign_in;

// 用途：导出系统字典相关VO模块
// 说明：允许其他模块访问系统字典相关的VO
pub mod sys_dict;

// 用途：导出系统用户相关VO模块
// 说明：允许其他模块访问系统用户相关的VO
pub mod sys_user;

// 用途：重导出JWT相关VO
// 说明：方便其他模块直接使用，无需嵌套导入
pub use jwt::*;

// 用途：重导出登录相关VO
// 说明：方便其他模块直接使用，无需嵌套导入
pub use sign_in::*;

// 用途：重导出系统字典相关VO
// 说明：方便其他模块直接使用，无需嵌套导入
pub use sys_dict::*;

// 用途：导入全局上下文
// 说明：用于访问配置信息
use crate::context::CONTEXT;

// 用途：导入自定义错误类型
// 说明：用于处理错误情况
use crate::error::Error;

// 用途：导入反序列化特性
// 说明：用于约束RespVO的泛型参数
use serde::de::DeserializeOwned;

// 用途：导入序列化和反序列化特性
// 说明：用于实现VO的序列化和反序列化
use serde::{Deserialize, Serialize};

/// 用途：成功状态码
/// 说明：表示API请求成功
pub const CODE_SUCCESS: &str = "0";

/// 用途：失败状态码
/// 说明：表示API请求失败
pub const CODE_FAIL: &str = "-1";

/// 用途：HTTP接口返回模型结构
/// 说明：提供统一的JSON数据结构，包含code、msg、data等基础字段
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RespVO<T> {
    /// 用途：状态码
    /// 说明：表示请求的处理结果（成功/失败）
    pub code: Option<String>,
    
    /// 用途：消息
    /// 说明：返回请求处理的消息，如错误信息
    pub msg: Option<String>,
    
    /// 用途：数据
    /// 说明：返回请求处理的结果数据
    pub data: Option<T>,
}

impl<T> RespVO<T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    /// 用途：从结果创建响应
    /// 说明：根据Result的结果自动创建成功或失败的响应
    pub fn from_result(result: Result<T, Error>) -> Self {
        match result {
            // 用途：处理成功结果
            // 说明：创建成功响应，包含数据
            Ok(data) => Self {
                code: Some(CODE_SUCCESS.to_string()),
                msg: None,
                data: Some(data),
            },
            // 用途：处理失败结果
            // 说明：创建失败响应，包含错误信息
            Err(e) => Self::from_error(e.to_string()),
        }
    }

    /// 用途：从数据创建响应
    /// 说明：创建成功响应，直接使用提供的数据
    pub fn from(data: T) -> Self {
        Self {
            code: Some(CODE_SUCCESS.to_string()),
            msg: None,
            data: Some(data),
        }
    }

    /// 用途：从错误信息创建响应
    /// 说明：创建失败响应，包含错误信息
    pub fn from_error(error: String) -> Self {
        // 用途：获取错误码
        // 说明：根据错误信息查找对应的错误码
        let code = CONTEXT
            .config
            .error_infos
            .get(&error)
            .map(|v| v.to_string())
            .unwrap_or_else(|| CODE_FAIL.to_string());
        
        // 用途：创建失败响应
        // 说明：返回错误码和错误信息
        Self {
            code: Some(code),
            msg: Some(error),
            data: None,
        }
    }
}

impl<T> ToString for RespVO<T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    /// 用途：转换为字符串
    /// 说明：将响应对象转换为JSON字符串
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}
