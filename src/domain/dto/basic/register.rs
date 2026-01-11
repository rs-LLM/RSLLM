// 用途：导入serde的序列化和反序列化特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输数据
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI Schema支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：派生序列化、反序列化、克隆、调试和Schema特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试；支持OpenAPI文档生成
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
// 用途：用户注册请求数据传输对象
// 说明：封装用户注册所需的所有信息
pub struct UserRegisterDTO {
    // 用途：用户名称
    // 说明：用于显示用户名称，必填字段
    #[schema(example = "张三")]
    pub name: String,
    // 用途：用户邮箱
    // 说明：用于用户注册和找回密码，唯一标识用户身份，必填字段
    #[schema(example = "user@example.com")]
    pub email: String,
    // 用途：用户密码
    // 说明：用于验证用户身份，必填字段
    #[schema(example = "SecurePass123!")]
    pub password: String,
    // 用途：同意服务条款
    // 说明：用户必须同意服务条款和隐私政策才能注册
    #[schema(example = true)]
    pub agree_terms: bool,
}

// 用途：派生序列化、反序列化、克隆、调试和Schema特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试；支持OpenAPI文档生成
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
// 用途：注册结果数据传输对象
// 说明：封装注册操作的结果信息
pub struct RegisterResultDTO {
    // 用途：操作是否成功
    // 说明：标识注册操作是否成功完成
    #[schema(example = true)]
    pub success: bool,
    // 用途：结果消息
    // 说明：描述注册操作的结果或错误信息
    #[schema(example = "注册成功")]
    pub message: String,
    // 用途：用户ID（可选）
    // 说明：注册成功时返回的用户ID
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "1234567890")]
    pub user_id: Option<String>,
}

// 用途：派生序列化、反序列化、克隆、调试和Schema特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试；支持OpenAPI文档生成
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
// 用途：注册错误数据传输对象
// 说明：封装注册失败的错误信息
pub struct RegisterErrorDTO {
    // 用途：错误代码
    // 说明：标识具体的错误类型
    #[schema(example = "EMAIL_EXISTS")]
    pub error_code: String,
    // 用途：错误消息
    // 说明：描述错误的具体原因
    #[schema(example = "该邮箱已被注册")]
    pub error_message: String,
}
