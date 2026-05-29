// 用途：导入登录检查枚举
// 说明：用于定义用户的登录验证方式
use crate::domain::table::LoginCheck;
// 用途：导入日期时间类型
// 说明：用于记录用户的创建时间
use rbatis::rbdc::DateTime;
// 用途：导入rbatis的CRUD宏和HTML SQL分页查询宏
// 说明：用于自动生成增删改查操作和HTML SQL分页查询方法
use rbatis::{crud, htmlsql_select_page};
// 用途：导入用户分页查询DTO
// 说明：用于用户表的分页查询操作
use crate::domain::dto::UserPageDTO;

// 用途：系统用户表结构体
// 说明：用于存储系统后台用户的基本信息
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SysUser {
    // 用途：用户ID
    // 说明：用户的唯一标识符
    pub id: Option<String>,
    // 用途：用户账号
    // 说明：用于用户登录，唯一标识用户身份
    pub account: Option<String>,
    // 用途：用户密码
    // 说明：用于用户登录验证，存储加密后的密码
    pub password: Option<String>,
    // 用途：用户姓名
    // 说明：用于显示用户名称
    pub name: Option<String>,
    // 用途：用户邮箱
    // 说明：用于用户注册和找回密码，唯一标识用户身份
    pub email: Option<String>,
    pub avatar: Option<String>,
    // 用途：登录检查方式
    // 说明：定义用户登录时需要的验证方式，如密码、验证码等
    pub login_check: Option<LoginCheck>,
    // 用途：用户状态
    // 说明：控制用户是否可用，1表示启用，0表示禁用
    pub state: Option<i32>,
    // 用途：用户余额
    // 说明：存储用户的账户余额，用于计费系统
    pub balance: Option<f64>,
    // 用途：用户等级
    // 说明：用户等级（L1/L2/L3/L4/L5），用于限流和配额管理，默认L1
    pub user_level: Option<String>,
    // 用途：创建时间
    // 说明：记录用户的创建时间
    pub create_date: Option<DateTime>,
}

// 用途：生成CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(SysUser {});

// 用途：SysUser实现
// 说明：提供自定义的查询方法
impl SysUser {
    // 用途：分页查询用户
    // 说明：支持根据条件分页查询用户列表
    htmlsql_select_page!(select_page(dto:&UserPageDTO) -> SysUser => "src/domain/table/basic/sys_user.html");
}
