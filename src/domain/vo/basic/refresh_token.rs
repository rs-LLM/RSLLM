// 用途：导入serde的序列化和反序列化特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输数据
use serde::{Deserialize, Serialize};

// 用途：派生序列化、反序列化、克隆和调试特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试
#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema)]
// 用途：刷新令牌响应视图对象
// 说明：用于返回给客户端的刷新令牌结果，包含新的访问令牌和刷新令牌
pub struct RefreshTokenVO {
    // 用途：访问令牌
    // 说明：用于后续请求的身份验证
    #[schema(example = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...")]
    pub access_token: String,
    // 用途：刷新令牌
    // 说明：用于后续刷新访问令牌
    #[schema(example = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...")]
    pub refresh_token: String,
}
