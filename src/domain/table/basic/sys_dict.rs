// 用途：导入字典分页查询DTO
// 说明：用于字典表的分页查询操作
use crate::domain::dto::DictPageDTO;
// 用途：导入日期时间类型
// 说明：用于记录字典的创建时间
use rbatis::rbdc::DateTime;
// 用途：导入rbatis的CRUD宏和HTML SQL分页查询宏
// 说明：用于自动生成增删改查操作和HTML SQL分页查询方法
use rbatis::{crud, htmlsql_select_page};

// 用途：系统字典表结构体
// 说明：用于存储系统配置的字典数据，如状态码、分类等
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SysDict {
    // 用途：字典ID
    // 说明：字典的唯一标识符
    pub id: Option<String>,
    // 用途：字典名称
    // 说明：用于显示和标识字典
    pub name: Option<String>,
    // 用途：字典编码
    // 说明：用于系统中引用字典，如"status"、"type"
    pub code: Option<String>,
    // 用途：字典状态
    // 说明：用于控制字典是否可用，1表示启用，0表示禁用
    pub state: Option<i32>,
    // 用途：创建时间
    // 说明：记录字典的创建时间
    pub create_date: Option<DateTime>,
}

// 用途：生成CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(SysDict {});

// 用途：SysDict实现
// 说明：提供自定义的查询方法
impl SysDict {
    // 用途：分页查询字典
    // 说明：支持根据条件分页查询字典列表
    htmlsql_select_page!(select_page(dto:&DictPageDTO) -> SysDict => "src/domain/table/basic/sys_dict.html");
}
