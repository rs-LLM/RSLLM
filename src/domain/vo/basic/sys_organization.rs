//! 系统组织响应视图对象模块。
//! 定义组织信息与组织树节点的返回结构及表结构到视图对象的转换逻辑。

// 用途：导入全局上下文实例
// 说明：用于获取配置信息，如日期时间格式
use crate::context::CONTEXT;

// 用途：导入组织表结构
// 说明：用于将表结构转换为视图对象
use crate::domain::table::basic::sys_organization::SysOrganization;

// 用途：组织视图对象结构体
// 说明：用于返回给客户端的组织信息，包含格式化后的创建时间
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
/// 系统组织视图对象。
/// 用于向客户端返回组织基础信息与格式化后的创建时间字段。
pub struct SysOrganizationVO {
    // 用途：组织ID
    // 说明：组织的唯一标识符
    pub id: Option<String>,
    // 用途：组织名称
    // 说明：用于显示和标识组织
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
    // 用途：创建时间
    // 说明：记录组织的创建时间，已格式化为字符串
    pub create_date: Option<String>,
}

// 用途：实现SysOrganization到系统组织视图对象的转换
// 说明：将数据库表结构转换为视图对象，方便返回给客户端
impl From<SysOrganization> for SysOrganizationVO {
    // 用途：转换方法
    // 说明：将SysOrganization转换为系统组织视图对象，格式化创建时间
    fn from(arg: SysOrganization) -> Self {
        Self {
            id: arg.id,
            name: arg.name,
            code: arg.code,
            parent_id: arg.parent_id,
            org_type: arg.org_type,
            description: arg.description,
            sort_order: arg.sort_order,
            status: arg.status,
            create_date: arg
                .create_date
                .map(|v| v.format(&CONTEXT.config.datetime_format)),
        }
    }
}

// 用途：组织树节点视图对象结构体
// 说明：用于返回组织树结构
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
/// 组织树节点视图对象。
/// 用于表示组织树的递归节点结构，包含当前节点与子节点列表。
pub struct OrganizationTreeNodeVO {
    // 用途：组织ID
    // 说明：组织的唯一标识符
    pub id: Option<String>,
    // 用途：组织名称
    // 说明：用于显示和标识组织
    pub name: Option<String>,
    // 用途：组织代码
    // 说明：组织的唯一编码
    pub code: Option<String>,
    // 用途：组织类型
    // 说明：区分组织类型（company/department/team）
    pub org_type: Option<String>,
    // 用途：排序字段
    // 说明：用于组织的排序显示
    pub sort_order: Option<i32>,
    // 用途：子组织列表
    // 说明：递归结构，包含子组织
    pub children: Vec<OrganizationTreeNodeVO>,
}
