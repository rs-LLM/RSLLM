//! Basic DTO module.
//! Exports foundational DTO submodules and common DTO types used by basic domain capabilities.

// 用途：导出认证相关DTO模块
// 说明：允许其他模块访问认证相关的DTO
pub mod auth;

// 用途：导出登录相关DTO模块
// 说明：允许其他模块访问登录相关的DTO
pub mod sign_in;

// 用途：导出注册相关DTO模块
// 说明：允许其他模块访问注册相关的DTO
pub mod register;

// 用途：导出刷新令牌相关DTO模块
// 说明：允许其他模块访问刷新令牌相关的DTO
pub mod refresh_token;

// 用途：导出系统字典相关DTO模块
// 说明：允许其他模块访问系统字典相关的DTO
pub mod sys_dict;

// 用途：导出系统用户相关DTO模块
// 说明：允许其他模块访问系统用户相关的DTO
pub mod sys_user;

// 用途：导出系统组织相关DTO模块
// 说明：允许其他模块访问系统组织相关的DTO
pub mod sys_organization;

// 用途：导出RBAC相关DTO模块
// 说明：允许其他模块访问RBAC相关的DTO
pub mod rbac;

// 用途：导出 TwoFA 相关DTO模块
// 说明：允许其他模块访问2FA相关的DTO
pub mod twofa;

// 用途：重导出 TwoFA 相关DTO
// 说明：方便其他模块直接使用，无需嵌套导入
pub use twofa::*;

// 用途：重导出认证相关DTO
// 说明：方便其他模块直接使用，无需嵌套导入
pub use auth::*;

// 用途：重导出登录相关DTO
// 说明：方便其他模块直接使用，无需嵌套导入
pub use sign_in::*;

// 用途：重导出刷新令牌相关DTO
// 说明：方便其他模块直接使用，无需嵌套导入
pub use refresh_token::*;

// 用途：重导出系统字典相关DTO
// 说明：方便其他模块直接使用，无需嵌套导入
pub use sys_dict::*;

// 用途：重导出系统用户相关DTO
// 说明：方便其他模块直接使用，无需嵌套导入
pub use sys_user::*;

// 用途：重导出RBAC相关DTO
// 说明：方便其他模块直接使用，无需嵌套导入
pub use rbac::*;

// 用途：导入序列化和反序列化特性
// 说明：用于实现DTO的序列化和反序列化
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI Schema支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

/// 用途：空DTO
/// 说明：用于不需要请求体的情况
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct EmptyDTO {}

/// 用途：ID数据传输对象
/// 说明：用于接收和传输ID参数
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
pub struct IdDTO {
    /// 用途：ID字段
    /// 说明：用于标识资源的唯一ID
    pub id: Option<String>,
}
