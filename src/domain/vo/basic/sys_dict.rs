//! 系统字典响应视图对象模块。
//! 定义系统字典信息返回结构及表结构到视图对象的转换逻辑。

// 用途：导入全局上下文实例
// 说明：用于获取配置信息，如日期时间格式
use crate::context::CONTEXT;
// 用途：导入系统字典表结构
// 说明：用于从SysDict转换为系统字典视图对象
use crate::domain::table::sys_dict::SysDict;
// 用途：导入OpenAPI Schema支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：系统字典视图对象结构体
// 说明：用于返回给客户端的字典信息，包含格式化后的创建时间
/// 系统字典视图对象。
/// 用于向客户端返回字典基础信息与格式化后的创建时间字段。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct SysDictVO {
    // 用途：字典ID
    // 说明：字典的唯一标识符
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    // 用途：字典名称
    // 说明：用于显示和标识字典
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    // 用途：字典编码
    // 说明：用于系统中引用字典，如"status"、"type"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    // 用途：字典状态
    // 说明：用于控制字典是否可用，1表示启用，0表示禁用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<i32>,
    // 用途：创建时间
    // 说明：记录字典的创建时间，已格式化为字符串
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_date: Option<String>,
}

// 用途：实现SysDict到SysDictVO的转换
// 说明：将数据库表结构转换为视图对象，方便返回给客户端
impl From<SysDict> for SysDictVO {
    // 用途：转换方法
    // 说明：将SysDict转换为系统字典视图对象，格式化创建时间
    fn from(arg: SysDict) -> Self {
        Self {
            id: arg.id,
            name: arg.name,
            code: arg.code,
            state: arg.state,
            create_date: arg
                .create_date
                .map(|v| v.format(&CONTEXT.config.datetime_format)),
        }
    }
}

// 用途：系统字典视图对象实现
// 说明：为系统字典视图对象提供扩展方法的占位符
impl SysDictVO {}
