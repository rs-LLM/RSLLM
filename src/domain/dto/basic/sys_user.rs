// 用途：导入登录检查枚举
// 说明：用于定义用户登录检查状态
use crate::domain::table::LoginCheck;
// 用途：导入系统用户表结构
// 说明：用于DTO与表结构之间的转换
use crate::domain::table::sys_user::SysUser;
// 用途：导入密码编码器
// 说明：用于密码加密，确保用户密码安全存储
use crate::util::password_encoder::PasswordEncoder;
// 用途：导入分页请求类型
// 说明：用于构建分页查询请求
use rbatis::PageRequest;
// 用途：导入对象ID生成器
// 说明：用于生成用户的唯一标识
use ulid::Ulid;
// 用途：导入日期时间类型
// 说明：用于记录用户的创建时间
use rbatis::rbdc::DateTime;
// 用途：导入serde的序列化和反序列化特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输数据
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI Schema支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：派生序列化、反序列化、克隆、调试和Schema特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试；支持OpenAPI文档生成
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
// 用途：用户添加DTO结构体
// 说明：封装添加用户所需的参数
pub struct UserAddDTO {
    // 用途：用户账号
    // 说明：用于标识用户身份，登录时使用
    pub account: Option<String>,
    // 用途：用户密码
    // 说明：用于验证用户身份，需要加密存储
    pub password: Option<String>,
    // 用途：用户姓名
    // 说明：用于显示用户名称
    pub name: Option<String>,
    // 用途：用户邮箱
    // 说明：用于用户注册和找回密码，唯一标识用户身份
    pub email: Option<String>,
    // 用途：登录检查状态
    // 说明：用于控制用户登录行为
    #[schema(value_type = Option<String>)]
    pub login_check: Option<LoginCheck>,
    // 用途：角色ID
    // 说明：用于关联用户和角色
    pub role_id: Option<String>,
    // 用途：用户状态
    // 说明：用于控制用户是否可以登录（启用/禁用）
    pub state: Option<i32>,
    // 用途：用户余额
    // 说明：用于设置用户的初始余额，默认为0
    pub balance: Option<f64>,
}

// 用途：实现UserAddDTO到SysUser的转换
// 说明：方便将添加DTO转换为用户表实体
impl From<UserAddDTO> for SysUser {
    // 用途：转换方法
    // 说明：将添加DTO转换为用户表实体，自动生成ID和加密密码
    fn from(arg: UserAddDTO) -> Self {
        SysUser {
            id: Ulid::new().to_string().into(),
            account: arg.account.clone(),
            password: PasswordEncoder::encode(&arg.password.unwrap_or_default()).into(),
            name: arg.name.clone(),
            email: arg.email.clone(),
            login_check: arg.login_check.clone(),
            state: Some(arg.state.unwrap_or(1)),
            balance: arg.balance,
            create_date: DateTime::now().into(),
            user_level: Some("L1".to_string()),
        }
    }
}

// 用途：派生序列化、反序列化、克隆、调试和Schema特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试；支持OpenAPI文档生成
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
// 用途：用户编辑DTO结构体
// 说明：封装编辑用户所需的参数
pub struct UserEditDTO {
    // 用途：用户ID
    // 说明：指定要编辑的用户唯一标识
    pub id: Option<String>,
    // 用途：用户账号
    // 说明：用于修改用户账号
    pub account: Option<String>,
    // 用途：用户密码
    // 说明：用于修改用户密码
    pub password: Option<String>,
    // 用途：用户姓名
    // 说明：用于修改用户姓名
    pub name: Option<String>,
    // 用途：用户状态
    // 说明：用于修改用户状态（启用/禁用）
    pub state: Option<i32>,
    // 用途：登录检查状态
    // 说明：用于修改用户登录检查状态
    #[schema(value_type = Option<String>)]
    pub login_check: Option<LoginCheck>,
    // 用途：角色ID
    // 说明：用于修改用户关联的角色
    pub role_id: Option<String>,
}

// 用途：实现UserEditDTO到SysUser的转换
// 说明：方便将编辑DTO转换为用户表实体
impl From<UserEditDTO> for SysUser {
    // 用途：转换方法
    // 说明：将编辑DTO转换为用户表实体
    fn from(arg: UserEditDTO) -> Self {
        SysUser {
            id: arg.id,
            account: arg.account,
            password: arg.password,
            name: arg.name,
            email: None,
            login_check: arg.login_check,
            state: arg.state,
            create_date: None,
            balance: None,
            user_level: None,
        }
    }
}

// 用途：派生序列化、反序列化、克隆和调试特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试
#[derive(Serialize, Deserialize, Clone, Debug)]
// 用途：用户分页查询DTO结构体
// 说明：封装用户分页查询所需的参数
pub struct UserPageDTO {
    // 用途：页码
    // 说明：指定查询的页码，用于分页显示
    pub page_no: Option<u64>,
    // 用途：每页大小
    // 说明：指定每页显示的数据条数，用于分页显示
    pub page_size: Option<u64>,
    // 用途：用户账号
    // 说明：用于根据账号筛选用户
    pub account: Option<String>,
    // 用途：用户姓名
    // 说明：用于根据姓名筛选用户
    pub name: Option<String>,
}

// 用途：实现UserPageDTO到PageRequest的转换
// 说明：方便将DTO转换为分页请求对象
impl From<UserPageDTO> for PageRequest {
    // 用途：转换方法
    // 说明：将DTO中的分页参数转换为PageRequest
    fn from(arg: UserPageDTO) -> Self {
        PageRequest::new(arg.page_no.unwrap_or(1), arg.page_size.unwrap_or(10))
    }
}

// 用途：实现&UserRolePageDTO到UserPageDTO的转换
// 说明：方便从用户角色分页DTO中提取用户分页参数
impl From<&UserRolePageDTO> for UserPageDTO {
    // 用途：转换方法
    // 说明：从用户角色分页DTO中提取分页和查询参数
    fn from(arg: &UserRolePageDTO) -> Self {
        Self {
            page_no: arg.page_no,
            page_size: arg.page_size,
            account: arg.account.clone(),
            name: arg.name.clone(),
        }
    }
}

// 用途：派生序列化、反序列化、克隆、调试和Schema特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试；支持OpenAPI文档生成
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
// 用途：用户角色分页查询DTO结构体
// 说明：封装用户角色分页查询所需的参数
pub struct UserRolePageDTO {
    // 用途：页码
    // 说明：指定查询的页码，用于分页显示
    pub page_no: Option<u64>,
    // 用途：每页大小
    // 说明：指定每页显示的数据条数，用于分页显示
    pub page_size: Option<u64>,
    // 用途：用户账号
    // 说明：用于根据账号筛选用户
    pub account: Option<String>,
    // 用途：用户姓名
    // 说明：用于根据姓名筛选用户
    pub name: Option<String>,

    // 用途：是否启用角色设置
    // 说明：用于控制是否返回可设置的角色列表
    pub resp_set_role: Option<bool>,
}

// 用途：实现&UserRolePageDTO到PageRequest的转换
// 说明：方便将用户角色分页DTO引用转换为分页请求对象
impl From<&UserRolePageDTO> for PageRequest {
    // 用途：转换方法
    // 说明：将用户角色分页DTO中的分页参数转换为PageRequest
    fn from(arg: &UserRolePageDTO) -> Self {
        PageRequest::new(arg.page_no.unwrap_or(1), arg.page_size.unwrap_or(10))
    }
}
