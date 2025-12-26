// 用途：导入serde的序列化和反序列化特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输数据
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI Schema支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：派生序列化、反序列化、克隆、调试和Schema特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试；支持OpenAPI文档生成
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
// 用途：登录请求数据传输对象
// 说明：封装用户登录所需的账号、密码和验证码信息
pub struct SignInDTO {
    // 用途：用户账号
    // 说明：用于标识用户身份
    pub username: String,
    // 用途：用户密码
    // 说明：用于验证用户身份
    pub password: String,
    // 用途：验证码（可选）
    // 说明：用于防止恶意登录攻击，前端可能不发送此字段
    #[serde(default)]
    pub vcode: String,
}

// 用途：派生序列化、反序列化、克隆、调试和Schema特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试；支持OpenAPI文档生成
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
// 用途：验证码请求数据传输对象
// 说明：封装获取验证码所需的账号信息
pub struct CatpchaDTO {
    // 用途：用户账号（可选）
    // 说明：用于关联验证码与特定用户，防止验证码滥用
    pub account: Option<String>,
}
