// 用途：导入系统字典表结构
// 说明：用于DTO与表结构之间的转换
use crate::domain::table::sys_dict::SysDict;
// 用途：导入分页请求类型
// 说明：用于构建分页查询请求
use rbatis::PageRequest;
// 用途：导入日期时间类型
// 说明：用于记录字典的创建时间
use rbatis::rbdc::DateTime;
// 用途：导入serde的序列化和反序列化特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输数据
use serde::{Deserialize, Serialize};

// 用途：派生序列化、反序列化、克隆和调试特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试
#[derive(Serialize, Deserialize, Clone, Debug)]
// 用途：字典分页查询DTO
// 说明：封装字典分页查询所需的参数
pub struct DictPageDTO {
    // 用途：页码
    // 说明：指定查询的页码，用于分页显示
    pub page_no: Option<u64>,
    // 用途：每页大小
    // 说明：指定每页显示的数据条数，用于分页显示
    pub page_size: Option<u64>,
    // 用途：字典名称
    // 说明：用于根据名称筛选字典
    pub name: Option<String>,
    // 用途：字典编码
    // 说明：用于根据编码筛选字典
    pub code: Option<String>,
    // 用途：字典状态
    // 说明：用于根据状态筛选字典（启用/禁用）
    pub state: Option<i32>,
}

// 用途：实现DictPageDTO到PageRequest的转换
// 说明：方便将DTO转换为分页请求对象
impl From<DictPageDTO> for PageRequest {
    // 用途：转换方法
    // 说明：将DTO中的分页参数转换为PageRequest
    fn from(arg: DictPageDTO) -> Self {
        PageRequest::new(arg.page_no.unwrap_or(1), arg.page_size.unwrap_or(10))
    }
}

// 用途：实现&DictPageDTO到PageRequest的转换
// 说明：方便将DTO引用转换为分页请求对象
impl From<&DictPageDTO> for PageRequest {
    // 用途：转换方法
    // 说明：将DTO引用中的分页参数转换为PageRequest
    fn from(arg: &DictPageDTO) -> Self {
        PageRequest::new(arg.page_no.unwrap_or(1), arg.page_size.unwrap_or(10))
    }
}

// 用途：派生序列化、反序列化、克隆和调试特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试
#[derive(Serialize, Deserialize, Clone, Debug)]
// 用途：字典添加DTO
// 说明：封装添加字典所需的参数
pub struct DictAddDTO {
    // 用途：字典ID
    // 说明：指定字典的唯一标识
    pub id: Option<String>,
    // 用途：字典名称
    // 说明：指定字典的名称
    pub name: Option<String>,
    // 用途：字典编码
    // 说明：指定字典的编码
    pub code: Option<String>,
    // 用途：字典状态
    // 说明：指定字典的状态（启用/禁用）
    pub state: Option<i32>,
}

// 用途：实现DictAddDTO到SysDict的转换
// 说明：方便将添加DTO转换为字典表实体
impl From<DictAddDTO> for SysDict {
    // 用途：转换方法
    // 说明：将添加DTO转换为字典表实体
    fn from(arg: DictAddDTO) -> Self {
        SysDict {
            id: {
                if let Some(id) = arg.id {
                    Some(id)
                } else {
                    arg.code.clone()
                }
            },
            name: arg.name.clone(),
            code: arg.code.clone(),
            state: arg.state.clone(),
            create_date: DateTime::now().into(),
        }
    }
}

// 用途：派生序列化、反序列化、克隆和调试特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试
#[derive(Serialize, Deserialize, Clone, Debug)]
// 用途：字典编辑DTO
// 说明：封装编辑字典所需的参数
pub struct DictEditDTO {
    // 用途：字典ID
    // 说明：指定要编辑的字典唯一标识
    pub id: Option<String>,
    // 用途：字典名称
    // 说明：指定要编辑的字典名称
    pub name: Option<String>,
    // 用途：字典编码
    // 说明：指定要编辑的字典编码
    pub code: Option<String>,
    // 用途：字典状态
    // 说明：指定要编辑的字典状态（启用/禁用）
    pub state: Option<i32>,
}

// 用途：实现&DictEditDTO到SysDict的转换
// 说明：方便将编辑DTO引用转换为字典表实体
impl From<&DictEditDTO> for SysDict {
    // 用途：转换方法
    // 说明：将编辑DTO引用转换为字典表实体
    fn from(arg: &DictEditDTO) -> Self {
        SysDict {
            id: arg.id.clone(),
            name: arg.name.clone(),
            code: arg.code.clone(),
            state: arg.state.clone(),
            create_date: None,
        }
    }
}
