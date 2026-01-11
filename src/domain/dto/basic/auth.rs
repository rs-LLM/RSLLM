// 用途：认证数据传输对象
// 说明：用于在系统中传输认证相关信息

// 用途：派生序列化、反序列化、克隆和调试特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, utoipa::ToSchema)]
// 用途：系统认证DTO结构体
// 说明：封装认证所需的令牌和路径信息
pub struct SysAuthDTO {
    // 用途：访问令牌
    // 说明：用于标识和验证用户身份，实现无状态认证
    #[schema(example = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...")]
    pub access_token: String,
    // 用途：认证路径
    // 说明：指定需要认证的资源路径
    #[schema(example = "/user/info")]
    pub path: String,
}
