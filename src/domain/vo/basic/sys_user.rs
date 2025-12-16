// 用途：导入全局上下文实例
// 说明：用于获取配置信息，如日期时间格式
use crate::context::CONTEXT;
// 用途：导入登录检查枚举
// 说明：用于定义用户的登录验证方式
use crate::domain::table::LoginCheck;
// 用途：导入系统用户表结构
// 说明：用于从SysUser转换为SysUserVO
use crate::domain::table::sys_user::SysUser;
// 用途：导入系统角色视图对象
// 说明：用于在用户视图中包含角色信息
use crate::domain::vo::rbac::SysRoleVO;
// 用途：导入serde的序列化和反序列化特性
// 说明：支持SysUserVO的JSON序列化和反序列化，便于在网络中传输
use serde::{Deserialize, Serialize};

// 用途：系统用户视图对象结构体
// 说明：用于返回给客户端的用户信息，包含格式化后的创建时间和角色列表
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SysUserVO {
    // 用途：用户ID
    // 说明：用户的唯一标识符
    pub id: Option<String>,
    // 用途：用户账号
    // 说明：用于用户登录，唯一标识用户身份
    pub account: Option<String>,
    // 用途：用户密码
    // 说明：用户密码（通常在响应中会被过滤，不返回给客户端）
    pub password: Option<String>,
    // 用途：用户姓名
    // 说明：用于显示用户名称
    pub name: Option<String>,
    // 用途：登录检查方式
    // 说明：定义用户登录时需要的验证方式
    pub login_check: Option<LoginCheck>,
    // 用途：用户状态
    // 说明：控制用户是否可用，1表示启用，0表示禁用
    pub state: Option<i32>,
    // 用途：创建时间
    // 说明：记录用户的创建时间，已格式化为字符串
    pub create_date: Option<String>,
    // 用途：用户角色列表
    // 说明：存储用户拥有的角色信息
    pub roles: Vec<SysRoleVO>,
}

// 用途：实现SysUser到SysUserVO的转换
// 说明：将数据库表结构转换为视图对象，方便返回给客户端
impl From<SysUser> for SysUserVO {
    // 用途：转换方法
    // 说明：将SysUser转换为SysUserVO，格式化创建时间，初始化角色列表
    fn from(arg: SysUser) -> Self {
        Self {
            id: arg.id,
            account: arg.account,
            password: arg.password,
            name: arg.name,
            login_check: arg.login_check,
            state: arg.state,
            create_date: arg
                .create_date
                .map(|v| v.format(&CONTEXT.config.datetime_format)),
            roles: vec![],
        }
    }
}
