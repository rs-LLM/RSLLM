// 用途：导入权限分页查询DTO
// 说明：用于权限分页查询
use crate::domain::dto::rbac::PermissionPageDTO;
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
// 用途：导入集合类型
// 说明：用于实现IntoMap和IntoMapVec扩展
use std::collections::{HashMap, HashSet};
// 用途：导入哈希特性
// 说明：用于实现IntoMap和IntoMapVec扩展
use std::hash::Hash;

// 用途：权限资源表结构体
// 说明：用于存储系统中的权限信息，包括菜单权限和API权限
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Hash, Eq, PartialEq, Default)]
pub struct RbacPermission {
    // 用途：权限ID
    // 说明：权限的唯一标识符
    pub id: Option<String>,
    // 用途：权限名称
    // 说明：用于显示和标识权限
    pub name: Option<String>,
    // 用途：权限标识
    // 说明：用于权限验证，如"user:list"
    pub permission: Option<String>,
    // 用途：菜单路径
    // 说明：用于前端路由和菜单渲染
    pub path: Option<String>,
    // 用途：权限类型
    // 说明：区分权限类型（menu/button/api）
    pub permission_type: Option<String>,
    // 用途：权限描述
    // 说明：权限的详细描述
    pub description: Option<String>,
    // 用途：排序字段
    // 说明：用于权限的排序显示
    pub sort_order: Option<i32>,
    // 用途：前端图标
    // 说明：用于前端菜单图标显示
    pub icon: Option<String>,
    // 用途：状态
    // 说明：权限启用/禁用状态（1启用，0禁用）
    pub status: Option<i32>,
    // 用途：创建时间
    // 说明：记录权限的创建时间
    pub create_date: Option<DateTime>,
}

// 用途：生成CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(RbacPermission {});

// 用途：RbacPermission实现
// 说明：提供自定义的查询方法
impl RbacPermission {
    // 用途：分页查询权限
    // 说明：支持根据条件分页查询权限列表
    htmlsql_select_page!(select_page(dto: &PermissionPageDTO) -> RbacPermission => "src/domain/table/basic/rbac.html");
    // 用途：根据权限标识或名称查询
    // 说明：用于验证权限是否存在
    htmlsql!(select_by_permission_or_name(rb:&dyn Executor, permission:&str,name:&str) -> Vec<RbacPermission> => "src/domain/table/basic/rbac.html");
    // 用途：查询父ID为空的权限
    // 说明：用于获取顶级权限列表
    htmlsql!(select_by_parent_id_null(rb:&dyn Executor) -> Vec<RbacPermission> => "src/domain/table/basic/rbac.html");
}

// 用途：角色表结构体
// 说明：用于存储系统中的角色信息
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RbacRole {
    // 用途：角色ID
    // 说明：角色的唯一标识符
    pub id: Option<String>,
    // 用途：角色名称
    // 说明：用于显示和标识角色
    pub name: Option<String>,
    // 用途：创建时间
    // 说明：记录角色的创建时间
    pub create_date: Option<DateTime>,
}

// 用途：生成CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(RbacRole {});

// 用途：RbacRole实现
// 说明：提供自定义的查询方法
impl RbacRole {
    // 用途：根据名称分页查询角色
    // 说明：支持根据角色名称分页查询
    htmlsql_select_page!(select_page_by_name(name:&str) -> RbacRole => "src/domain/table/basic/rbac.html");
}

// 用途：角色权限关联表结构体
// 说明：用于存储角色和权限的多对多关联关系
#[derive(Clone, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub struct RbacRolePermission {
    // 用途：关联ID
    // 说明：关联记录的唯一标识符
    pub id: Option<String>,
    // 用途：角色ID
    // 说明：关联的角色ID
    pub role_id: Option<String>,
    // 用途：权限ID
    // 说明：关联的权限ID
    pub permission_id: Option<String>,
    // 用途：创建时间
    // 说明：记录关联的创建时间
    pub create_date: Option<DateTime>,
}

// 用途：生成CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(RbacRolePermission {});

// 用途：RbacRolePermission实现
// 说明：提供自定义的查询方法
impl RbacRolePermission {
    // 用途：根据角色ID列表查询角色权限关联
    // 说明：获取多个角色的所有权限关联信息
    htmlsql!(select_by_role_ids(rb:&dyn Executor, role_ids:&Vec<String>) -> Vec<RbacRolePermission> => "src/domain/table/basic/rbac.html");
}

// 用途：用户角色关联表结构体
// 说明：用于存储用户和角色的多对多关联关系
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Eq, PartialEq, Hash)]
pub struct RbacUserRole {
    // 用途：关联ID
    // 说明：关联记录的唯一标识符
    pub id: Option<String>,
    // 用途：用户ID
    // 说明：关联的用户ID
    pub user_id: Option<String>,
    // 用途：角色ID
    // 说明：关联的角色ID
    pub role_id: Option<String>,
    // 用途：创建时间
    // 说明：记录关联的创建时间
    pub create_date: Option<DateTime>,
}

// 用途：生成CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(RbacUserRole {});

// 用途：权限审计日志表结构体
// 说明：用于记录权限的创建、修改、删除操作历史
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RbacPermissionAuditLog {
    // 用途：审计日志ID
    // 说明：审计日志的唯一标识符
    pub id: Option<String>,
    // 用途：权限ID
    // 说明：被操作的权限ID
    pub permission_id: Option<String>,
    // 用途：操作类型
    // 说明：操作类型（create/update/delete）
    pub operation_type: Option<String>,
    // 用途：变更前的值
    // 说明：变更前的权限数据（JSON格式）
    pub old_value: Option<String>,
    // 用途：变更后的值
    // 说明：变更后的权限数据（JSON格式）
    pub new_value: Option<String>,
    // 用途：操作人ID
    // 说明：执行操作的用户ID
    pub operator_id: Option<String>,
    // 用途：操作人姓名
    // 说明：执行操作的用户姓名
    pub operator_name: Option<String>,
    // 用途：操作时间
    // 说明：记录操作的时间
    pub create_time: Option<DateTime>,
}

// 用途：生成CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(RbacPermissionAuditLog {});

// 用途：同步RBAC相关表结构
// 说明：负责初始化和同步数据库中的RBAC表结构
pub async fn sync_tables(conn: &dyn Executor, mapper: &dyn ColumnMapper) {
    // 用途：创建权限表结构
    // 说明：初始化权限表，存储系统权限信息
    let table = RbacPermission {
        id: Some(Default::default()),
        name: Some(Default::default()),
        permission: Some(Default::default()),
        path: Some(Default::default()),
        permission_type: Some(Default::default()),
        description: Some(Default::default()),
        sort_order: Some(Default::default()),
        icon: Some(Default::default()),
        status: Some(Default::default()),
        create_date: Some(Default::default()),
    };
    let _ = RBatis::sync(conn, mapper, &table, "rbac_permission").await;

    // 用途：创建角色表结构
    // 说明：初始化角色表，存储系统角色信息
    let table = RbacRole {
        id: Some(Default::default()),
        name: Some(Default::default()),
        create_date: Some(Default::default()),
    };
    let _ = RBatis::sync(conn, mapper, &table, "rbac_role").await;

    // 用途：创建角色权限关联表结构
    // 说明：初始化角色权限关联表，存储角色和权限的关联关系
    let table = RbacRolePermission {
        id: Some(Default::default()),
        role_id: Some(Default::default()),
        permission_id: Some(Default::default()),
        create_date: Some(Default::default()),
    };
    let _ = RBatis::sync(conn, mapper, &table, "rbac_role_permission").await;

    // 用途：创建用户角色关联表结构
    // 说明：初始化用户角色关联表，存储用户和角色的关联关系
    let table = RbacUserRole {
        id: Some(Default::default()),
        user_id: Some(Default::default()),
        role_id: Some(Default::default()),
        create_date: Some(Default::default()),
    };
    let _ = RBatis::sync(conn, mapper, &table, "rbac_user_role").await;

    // 用途：创建权限审计日志表结构
    // 说明：初始化权限审计日志表，存储权限变更历史
    let table = RbacPermissionAuditLog {
        id: Some(Default::default()),
        permission_id: Some(Default::default()),
        operation_type: Some(Default::default()),
        old_value: Some(Default::default()),
        new_value: Some(Default::default()),
        operator_id: Some(Default::default()),
        operator_name: Some(Default::default()),
        create_time: Some(Default::default()),
    };
    let _ = RBatis::sync(conn, mapper, &table, "rbac_permission_audit_log").await;
}

// 用途：IntoMap扩展trait
// 说明：提供将Vec转换为HashMap的便捷方法
pub trait IntoMap<K: Eq + Hash, V>: Sized + IntoIterator<Item = V> {
    // 用途：将Vec转换为HashMap
    // 说明：根据指定的ID函数将元素映射到HashMap中
    fn into_map(self, id_fn: fn(&V) -> K) -> HashMap<K, V> {
        let mut map = HashMap::new();
        for item in self {
            map.insert(id_fn(&item), item);
        }
        map
    }
}

// 用途：IntoMapVec扩展trait
// 说明：提供将Vec转换为HashMap<Vec<V>>的便捷方法
pub trait IntoMapVec<K: Eq + Hash + Clone, V: Eq + Hash>: Sized + IntoIterator<Item = V> {
    // 用途：将Vec转换为HashMap<Vec<V>>
    // 说明：根据指定的ID函数将元素分组映射到HashMap中，去重并转换为Vec
    fn into_map(self, id_fn: fn(&V) -> K) -> HashMap<K, Vec<V>> {
        let mut map = HashMap::new();
        for item in self {
            let key = id_fn(&item);
            if !map.contains_key(&key) {
                map.insert(key.clone(), HashSet::new());
            }
            if let Some(v) = map.get_mut(&key) {
                v.insert(item);
            }
        }
        let mut map2 = HashMap::with_capacity(map.len());
        for (k, v) in map {
            let vec = v.into_iter().collect::<Vec<V>>();
            map2.insert(k, vec);
        }
        map2
    }
}

// 用途：为Vec<RbacRole>实现IntoMap trait
// 说明：允许将角色列表转换为以角色ID为键的HashMap
impl IntoMap<String, RbacRole> for Vec<RbacRole> {}

// 用途：为Vec<RbacPermission>实现IntoMap trait
// 说明：允许将权限列表转换为以权限ID为键的HashMap
impl IntoMap<String, RbacPermission> for Vec<RbacPermission> {}

// 用途：为Vec<RbacRolePermission>实现IntoMapVec trait
// 说明：允许将角色权限关联列表转换为以角色ID为键的HashMap<Vec<RbacRolePermission>>
impl IntoMapVec<String, RbacRolePermission> for Vec<RbacRolePermission> {}

// 用途：为Vec<RbacUserRole>实现IntoMapVec trait
// 说明：允许将用户角色关联列表转换为以用户ID为键的HashMap<Vec<RbacUserRole>>
impl IntoMapVec<String, RbacUserRole> for Vec<RbacUserRole> {}
