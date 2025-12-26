// 用途：导入执行器接口
// 说明：用于执行SQL查询
use rbatis::executor::Executor;

// 用途：导入日期时间类型
// 说明：用于记录创建时间
use rbatis::rbdc::DateTime;

// 用途：导入列映射器
// 说明：用于数据库表结构同步
use rbatis::table_sync::ColumnMapper;

// 用途：导入rbatis相关宏和结构体
// 说明：用于实现CRUD操作和HTML SQL查询
use rbatis::{RBatis, crud, htmlsql, htmlsql_select_page};

// 用途：导入组织分页查询DTO
// 说明：用于分页查询参数
use crate::domain::dto::basic::sys_organization::OrgPageDTO;

// 用途：组织架构表结构体
// 说明：用于存储系统中的组织架构信息
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SysOrganization {
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
    // 说明：记录组织的创建时间
    pub create_date: Option<DateTime>,
}

// 用途：生成CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(SysOrganization {});

// 用途：SysOrganization实现
// 说明：提供自定义的查询方法
impl SysOrganization {
    // 用途：分页查询组织
    // 说明：支持根据条件分页查询组织列表
    htmlsql_select_page!(select_page(dto: &OrgPageDTO) -> SysOrganization => "src/domain/table/basic/sys_organization.html");
    
    // 用途：查询所有组织
    // 说明：获取所有组织数据
    htmlsql!(select_all_custom(rb:&dyn Executor) -> Vec<SysOrganization> => "src/domain/table/basic/sys_organization.html");
    
    // 用途：根据父ID查询组织
    // 说明：获取指定父组织下的子组织列表
    htmlsql!(select_by_parent_id(rb:&dyn Executor, parent_id:&str) -> Vec<SysOrganization> => "src/domain/table/basic/sys_organization.html");
    
    // 用途：查询顶级组织
    // 说明：获取没有父组织的顶级组织列表
    htmlsql!(select_top_level(rb:&dyn Executor) -> Vec<SysOrganization> => "src/domain/table/basic/sys_organization.html");
}

// 用途：同步组织表结构
// 说明：负责初始化和同步数据库中的组织表结构
pub async fn sync_organization_tables(conn: &dyn Executor, mapper: &dyn ColumnMapper) {
    // 用途：创建组织表结构
    // 说明：初始化组织表，存储系统组织信息
    let table = SysOrganization {
        id: Some(Default::default()),
        name: Some(Default::default()),
        code: Some(Default::default()),
        parent_id: Some(Default::default()),
        org_type: Some(Default::default()),
        description: Some(Default::default()),
        sort_order: Some(Default::default()),
        status: Some(Default::default()),
        create_date: Some(Default::default()),
    };
    let _ = RBatis::sync(conn, mapper, &table, "sys_organization").await;
}
