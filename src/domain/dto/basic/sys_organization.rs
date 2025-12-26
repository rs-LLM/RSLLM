// 用途：导入组织表结构
// 说明：用于DTO与表结构之间的转换
use crate::domain::table::basic::sys_organization::SysOrganization;

// 用途：导入分页请求类型
// 说明：用于构建分页查询请求
use rbatis::PageRequest;

// 用途：导入对象ID生成器
// 说明：用于生成唯一标识符
use ulid::Ulid;

// 用途：导入日期时间类型
// 说明：用于记录创建时间
use rbatis::rbdc::DateTime;

// 用途：导入serde的序列化和反序列化特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输数据
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：派生序列化、反序列化、克隆、调试和ToSchema特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试；支持OpenAPI文档生成
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
// 用途：组织分页查询DTO结构体
// 说明：封装组织分页查询所需的参数
pub struct OrgPageDTO {
    // 用途：页码
    // 说明：指定查询的页码，用于分页显示
    pub page_no: Option<u64>,
    // 用途：每页大小
    // 说明：指定每页显示的数据条数，用于分页显示
    pub page_size: Option<u64>,
    // 用途：组织名称
    // 说明：用于根据名称筛选组织
    pub name: Option<String>,
    // 用途：组织代码
    // 说明：用于根据代码筛选组织
    pub code: Option<String>,
    // 用途：组织类型
    // 说明：用于根据类型筛选组织
    pub org_type: Option<String>,
}

// 用途：实现&OrgPageDTO到PageRequest的转换
// 说明：方便将DTO引用转换为分页请求对象
impl From<&OrgPageDTO> for PageRequest {
    // 用途：转换方法
    // 说明：将DTO中的分页参数转换为PageRequest
    fn from(arg: &OrgPageDTO) -> Self {
        PageRequest::new(arg.page_no.unwrap_or(1), arg.page_size.unwrap_or(10))
    }
}

// 用途：派生序列化、反序列化、克隆、调试和ToSchema特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试；支持OpenAPI文档生成
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
// 用途：组织添加DTO结构体
// 说明：封装添加组织所需的参数
pub struct OrgAddDTO {
    // 用途：组织名称
    // 说明：用于标识组织
    pub name: Option<String>,
    // 用途：组织代码
    // 说明：组织的唯一编码
    pub code: Option<String>,
    // 用途：父组织ID
    // 说明：用于构建组织的层级关系
    pub parent_id: Option<String>,
    // 用途：组织类型
    // 说明：区分组织类型（company/department/team）
    pub org_type: Option<String>,
    // 用途：组织描述
    // 说明：组织的详细描述
    pub description: Option<String>,
    // 用途：排序字段
    // 说明：用于组织的排序显示
    pub sort_order: Option<i32>,
    // 用途：状态
    // 说明：组织启用/禁用状态（1启用，0禁用）
    pub status: Option<i32>,
}

// 用途：实现OrgAddDTO到SysOrganization的转换
// 说明：方便将添加DTO转换为组织表实体
impl From<OrgAddDTO> for SysOrganization {
    // 用途：转换方法
    // 说明：将添加DTO转换为组织表实体，自动生成ID和创建时间
    fn from(arg: OrgAddDTO) -> Self {
        SysOrganization {
            id: Ulid::new().to_string().into(),
            name: arg.name,
            code: arg.code,
            parent_id: arg.parent_id,
            org_type: arg.org_type,
            description: arg.description,
            sort_order: arg.sort_order,
            status: arg.status,
            create_date: Some(DateTime::now()),
        }
    }
}

// 用途：派生序列化、反序列化、克隆、调试和ToSchema特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试；支持OpenAPI文档生成
#[derive(Serialize, Deserialize, Clone, Debug, ToSchema)]
// 用途：组织编辑DTO结构体
// 说明：封装编辑组织所需的参数
pub struct OrgEditDTO {
    // 用途：组织ID
    // 说明：指定要编辑的组织
    pub id: Option<String>,
    // 用途：组织名称
    // 说明：用于修改组织名称
    pub name: Option<String>,
    // 用途：组织代码
    // 说明：用于修改组织代码
    pub code: Option<String>,
    // 用途：父组织ID
    // 说明：用于修改组织的层级关系
    pub parent_id: Option<String>,
    // 用途：组织类型
    // 说明：用于修改组织类型
    pub org_type: Option<String>,
    // 用途：组织描述
    // 说明：用于修改组织描述
    pub description: Option<String>,
    // 用途：排序字段
    // 说明：用于修改组织的排序显示
    pub sort_order: Option<i32>,
    // 用途：状态
    // 说明：用于修改组织启用/禁用状态
    pub status: Option<i32>,
}

// 用途：实现&OrgEditDTO到SysOrganization的转换
// 说明：方便将编辑DTO引用转换为组织表实体
impl From<&OrgEditDTO> for SysOrganization {
    // 用途：转换方法
    // 说明：将编辑DTO引用转换为组织表实体
    fn from(arg: &OrgEditDTO) -> Self {
        SysOrganization {
            id: arg.id.clone(),
            name: arg.name.clone(),
            code: arg.code.clone(),
            parent_id: arg.parent_id.clone(),
            org_type: arg.org_type.clone(),
            description: arg.description.clone(),
            sort_order: arg.sort_order.clone(),
            status: arg.status.clone(),
            create_date: None,
        }
    }
}
