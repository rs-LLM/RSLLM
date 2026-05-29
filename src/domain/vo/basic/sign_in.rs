//! 登录信息响应视图对象模块。
//! 定义登录成功后返回的用户信息、权限与令牌载荷结构。

// 用途：导入登录检查枚举
// 说明：用于定义用户的登录验证方式
use crate::domain::table::LoginCheck;

// 用途：导入系统用户表结构
// 说明：用于从SysUser转换为登录信息响应视图对象
use crate::domain::table::sys_user::SysUser;
// 用途：导入serde的序列化和反序列化特性
// 说明：支持SignInVO的JSON序列化和反序列化，便于在网络中传输
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI Schema支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：登录信息响应视图对象
// 说明：用于返回给客户端的登录结果，包含用户信息、权限和令牌
/// 登录结果视图对象。
/// 用于向客户端返回登录后的用户资料、权限与访问令牌信息。
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct SignInVO {
    // 用途：用户ID
    // 说明：标识登录用户的唯一ID，对应前端的userId
    #[serde(rename = "userId")]
    pub id: Option<String>,
    // 用途：用户账号
    // 说明：用户的登录账号，对应前端的username
    #[serde(rename = "username")]
    pub account: Option<String>,
    // 用途：用户姓名
    // 说明：用户的显示名称，对应前端的realName
    #[serde(rename = "realName")]
    pub name: Option<String>,
    // 用途：登录检查方式
    // 说明：定义用户登录时需要的验证方式
    #[schema(value_type = Option<String>)]
    pub login_check: Option<LoginCheck>,
    // 用途：用户状态
    // 说明：控制用户是否可用，1表示启用，0表示禁用
    pub state: Option<i32>,
    // 用途：创建时间
    // 说明：用户的创建时间，已格式化为字符串
    pub create_date: Option<String>,
    // 用途：用户权限列表
    // 说明：用户拥有的权限标识列表
    pub permissions: Vec<String>,
    // 用途：访问令牌
    // 说明：用于后续请求的身份验证，对应前端的token
    #[serde(rename = "token")]
    pub access_token: String,
    // 用途：用户角色列表
    // 说明：用户拥有的角色名称列表，对应Vben前端的roles字段
    pub roles: Vec<String>,
    // 用途：用户头像
    // 说明：用户的头像URL，对应前端的avatar
    #[serde(default)]
    pub avatar: Option<String>,
    // 用途：用户描述
    // 说明：用户的描述信息
    #[serde(default)]
    pub desc: Option<String>,
    // 用途：首页地址
    // 说明：用户的默认首页路径，对应前端的homePath
    #[serde(rename = "homePath")]
    #[serde(default)]
    pub home_path: Option<String>,
    // 用途：用户邮箱
    // 说明：用户的邮箱地址
    #[serde(default)]
    pub email: Option<String>,
    // 用途：用户余额
    // 说明：用户的账户余额，用于计费系统
    #[serde(default)]
    pub balance: Option<f64>,
}

// 用途：实现SysUser到SignInVO的转换
// 说明：将系统用户表结构转换为登录信息响应视图对象
impl From<SysUser> for SignInVO {
    // 用途：转换方法
    // 说明：将SysUser转换为SignInVO，初始化权限、令牌和角色列表
    fn from(value: SysUser) -> Self {
        Self {
            id: value.id,
            account: value.account,
            name: value.name,
            login_check: value.login_check,
            state: value.state,
            // create_date 的格式化由 service/controller 在运行时读取 system.datetime_format 后写入。
            create_date: None,
            permissions: vec![],
            access_token: "".to_string(),
            roles: vec![],
            avatar: value.avatar.or(Some("/user.png".to_string())),
            desc: Some("RSLLM系统用户".to_string()),
            home_path: Some("/control/home".to_string()),
            email: value.email,
            balance: value.balance,
        }
    }
}
