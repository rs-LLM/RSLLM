//! 系统用户响应视图对象模块。
//! 定义系统用户信息返回结构及表结构到视图对象的转换逻辑。

// 用途：导入登录检查枚举
// 说明：用于定义用户的登录验证方式
use crate::domain::table::LoginCheck;
// 用途：导入系统用户表结构
// 说明：用于从SysUser转换为系统用户视图对象
use crate::domain::table::sys_user::SysUser;
// 用途：导入系统角色视图对象
// 说明：用于在用户视图中包含角色信息
use crate::domain::vo::rbac::SysRoleVO;
// 用途：导入serde的序列化和反序列化特性
// 说明：支持SysUserVO的JSON序列化和反序列化，便于在网络中传输
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI Schema支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：系统用户视图对象结构体
// 说明：用于返回给客户端的用户信息，包含格式化后的创建时间和角色列表
/// 系统用户视图对象。
/// 用于向客户端返回用户基础信息与格式化后的创建时间字段。
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
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
    pub avatar: Option<String>,
    // 用途：登录检查方式
    // 说明：定义用户登录时需要的验证方式
    #[schema(value_type = Option<String>)]
    pub login_check: Option<LoginCheck>,
    // 用途：用户状态
    // 说明：控制用户是否可用，1表示启用，0表示禁用
    pub state: Option<i32>,
    // 用途：用户余额
    // 说明：存储用户的账户余额，用于计费系统
    pub balance: Option<f64>,
    // 用途：创建时间
    // 说明：记录用户的创建时间，已格式化为字符串
    pub create_date: Option<String>,
    // 用途：用户等级
    // 说明：用户等级（L1/L2/L3/L4/L5），用于速率限制和配额管理
    pub user_level: Option<String>,
    // 用途：用户角色列表
    // 说明：存储用户拥有的角色信息
    pub roles: Vec<SysRoleVO>,
}

// 用途：实现SysUser到SysUserVO的转换
// 说明：将数据库表结构转换为视图对象，方便返回给客户端
impl From<SysUser> for SysUserVO {
    // 用途：转换方法
    // 说明：将SysUser转换为SysUserVO，创建时间格式化由 service/controller 在运行时处理
    fn from(arg: SysUser) -> Self {
        Self {
            id: arg.id,
            account: arg.account,
            password: arg.password,
            name: arg.name,
            avatar: arg.avatar,
            login_check: arg.login_check,
            state: arg.state,
            balance: arg.balance,
            create_date: None,
            user_level: arg.user_level,
            roles: vec![],
        }
    }
}
