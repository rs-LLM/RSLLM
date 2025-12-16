// 用途：导入登录检查枚举
// 说明：用于定义用户的登录验证方式
use crate::domain::table::LoginCheck;

// 用途：导入全局上下文实例
// 说明：用于获取配置信息，如日期时间格式
use crate::context::CONTEXT;
// 用途：导入系统用户表结构
// 说明：用于从SysUser转换为SignInVO
use crate::domain::table::sys_user::SysUser;
// 用途：导入系统角色视图对象
// 说明：用于在登录响应中返回用户的角色信息
use crate::domain::vo::rbac::SysRoleVO;
// 用途：导入serde的序列化和反序列化特性
// 说明：支持SignInVO的JSON序列化和反序列化，便于在网络中传输
use serde::{Deserialize, Serialize};

// 用途：登录响应视图对象
// 说明：用于返回给客户端的登录结果，包含用户信息、权限和令牌
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SignInVO {
    // 用途：用户ID
    // 说明：标识登录用户的唯一ID
    pub id: Option<String>,
    // 用途：用户账号
    // 说明：用户的登录账号
    pub account: Option<String>,
    // 用途：用户密码
    // 说明：用户密码（通常在登录响应中会被过滤，不返回给客户端）
    pub password: Option<String>,
    // 用途：用户姓名
    // 说明：用户的显示名称
    pub name: Option<String>,
    // 用途：登录检查方式
    // 说明：定义用户登录时需要的验证方式
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
    // 说明：用于后续请求的身份验证
    pub access_token: String,
    // 用途：用户角色列表
    // 说明：用户拥有的角色信息列表
    pub roles: Vec<SysRoleVO>,
}

// 用途：实现SysUser到SignInVO的转换
// 说明：将系统用户表结构转换为登录响应视图对象
impl From<SysUser> for SignInVO {
    // 用途：转换方法
    // 说明：将SysUser转换为SignInVO，初始化权限、令牌和角色列表
    fn from(value: SysUser) -> Self {
        Self {
            id: value.id,
            account: value.account,
            password: value.password,
            name: value.name,
            login_check: value.login_check,
            state: value.state,
            create_date: value
                .create_date
                .map(|v| v.format(&CONTEXT.config.datetime_format)),
            permissions: vec![],
            access_token: "".to_string(),
            roles: vec![],
        }
    }
}
