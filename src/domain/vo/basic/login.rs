//! 登录信息响应视图对象模块。
//! 定义基础登录接口返回的令牌载荷结构。

// 用途：导入serde的序列化和反序列化特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输数据
use serde::{Deserialize, Serialize};

// 用途：派生序列化、反序列化、克隆、调试和Schema特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试；支持OpenAPI文档生成
/// 登录结果视图对象。
/// 用于向客户端返回访问令牌字段。
#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
// 用途：登录信息响应视图对象
// 说明：用于返回给Vben前端的登录结果，仅包含访问令牌
pub struct LoginVO {
    // 用途：访问令牌
    // 说明：用于后续请求的身份验证，前端字段名为accessToken
    #[serde(rename = "accessToken")]
    pub access_token: String,
}
