// 用途：导入全局上下文实例
// 说明：用于获取配置信息，如日期时间格式
use crate::context::CONTEXT;
// 用途：导入RBAC相关表结构
// 说明：用于将表结构转换为VO（视图对象）
use crate::domain::table::rbac::{RbacPermission, RbacRole};

// 用途：权限视图对象结构体
// 说明：用于返回给客户端的权限信息，包含格式化后的创建时间
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RbacPermissionVO {
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
    // 用途：创建时间
    // 说明：记录权限的创建时间，已格式化为字符串
    pub create_date: Option<String>,
}

// 用途：实现RbacPermission到RbacPermissionVO的转换
// 说明：将数据库表结构转换为视图对象，方便返回给客户端
impl From<RbacPermission> for RbacPermissionVO {
    // 用途：转换方法
    // 说明：将RbacPermission转换为RbacPermissionVO，格式化创建时间
    fn from(arg: RbacPermission) -> Self {
        Self {
            id: arg.id,
            name: arg.name,
            permission: arg.permission,
            path: arg.path,
            create_date: arg
                .create_date
                .map(|v| v.format(&CONTEXT.config.datetime_format)),
        }
    }
}

// 用途：系统角色视图对象结构体
// 说明：用于返回给客户端的角色信息，包含角色的权限列表
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SysRoleVO {
    // 用途：角色ID
    // 说明：角色的唯一标识符
    pub id: Option<String>,
    // 用途：角色名称
    // 说明：用于显示和标识角色
    pub name: Option<String>,
    // 用途：创建时间
    // 说明：记录角色的创建时间，已格式化为字符串
    pub create_date: Option<String>,
    // 用途：角色权限列表
    // 说明：存储角色拥有的权限信息
    pub permissions: Vec<RbacPermissionVO>,
    // 用途：角色权限ID列表
    // 说明：存储角色拥有的权限ID，用于前端展示和编辑
    pub permission_ids: Vec<String>,
}

// 用途：SysRoleVO实现
// 说明：提供设置权限的方法
impl SysRoleVO {
    // 用途：设置角色权限
    // 说明：为角色分配权限，并自动提取权限ID列表
    pub fn set_permissions(&mut self, data: Vec<RbacPermissionVO>) {
        self.permissions = data;
        self.permission_ids = self
            .permissions
            .iter()
            .map(|v| v.id.clone().unwrap_or_default())
            .collect();
    }
}

// 用途：实现RbacRole到SysRoleVO的转换
// 说明：将数据库表结构转换为视图对象，方便返回给客户端
impl From<RbacRole> for SysRoleVO {
    // 用途：转换方法
    // 说明：将RbacRole转换为SysRoleVO，初始化权限列表为空
    fn from(arg: RbacRole) -> Self {
        Self {
            id: arg.id,
            name: arg.name,
            create_date: arg
                .create_date
                .map(|v| v.format(&CONTEXT.config.datetime_format)),
            permissions: vec![],
            permission_ids: vec![],
        }
    }
}

// 用途：SysRoleVO实现
// 说明：提供从Option<RbacRole>转换为Option<SysRoleVO>的方法
impl SysRoleVO {
    // 用途：从Option<RbacRole>转换为Option<SysRoleVO>
    // 说明：处理角色可能不存在的情况，返回可选的角色视图对象
    pub fn from_option(arg: Option<RbacRole>) -> Option<SysRoleVO> {
        match arg {
            Some(arg) => Some(SysRoleVO {
                id: arg.id,
                name: arg.name,
                create_date: arg
                    .create_date
                    .map(|v| v.format(&CONTEXT.config.datetime_format)),
                permissions: vec![],
                permission_ids: vec![],
            }),
            _ => None,
        }
    }
}
