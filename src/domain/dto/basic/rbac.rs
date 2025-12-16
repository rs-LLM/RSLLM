// 用途：导入RBAC相关的表结构
// 说明：用于DTO与表结构之间的转换
use crate::domain::table::rbac::{RbacPermission, RbacRole, RbacUserRole};
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

// 用途：派生序列化、反序列化、克隆和调试特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试
#[derive(Serialize, Deserialize, Clone, Debug)]
// 用途：权限分页查询DTO结构体
// 说明：封装权限分页查询所需的参数
pub struct PermissionPageDTO {
    // 用途：页码
    // 说明：指定查询的页码，用于分页显示
    pub page_no: Option<u64>,
    // 用途：每页大小
    // 说明：指定每页显示的数据条数，用于分页显示
    pub page_size: Option<u64>,
    // 用途：权限名称
    // 说明：用于根据名称筛选权限
    pub name: Option<String>,
}

// 用途：实现&PermissionPageDTO到PageRequest的转换
// 说明：方便将DTO引用转换为分页请求对象
impl From<&PermissionPageDTO> for PageRequest {
    // 用途：转换方法
    // 说明：将DTO中的分页参数转换为PageRequest
    fn from(arg: &PermissionPageDTO) -> Self {
        PageRequest::new(arg.page_no.unwrap_or(1), arg.page_size.unwrap_or(10))
    }
}

// 用途：派生序列化、反序列化、克隆和调试特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试
#[derive(Serialize, Deserialize, Clone, Debug)]
// 用途：权限添加DTO结构体
// 说明：封装添加权限所需的参数
pub struct PermissionAddDTO {
    // 用途：父权限ID
    // 说明：用于构建权限的层级关系
    pub parent_id: Option<String>,
    // 用途：权限名称
    // 说明：用于标识权限
    pub name: Option<String>,
    // 用途：权限标识
    // 说明：用于权限验证
    pub permission: Option<String>,
    // 用途：权限路径
    // 说明：用于API路径与权限的映射
    pub path: Option<String>,
}

// 用途：实现PermissionAddDTO到RbacPermission的转换
// 说明：方便将添加DTO转换为权限表实体
impl From<PermissionAddDTO> for RbacPermission {
    // 用途：转换方法
    // 说明：将添加DTO转换为权限表实体，自动生成ID和创建时间
    fn from(arg: PermissionAddDTO) -> Self {
        RbacPermission {
            id: Ulid::new().to_string().into(),
            name: arg.name.clone(),
            permission: arg.permission.clone(),
            path: arg.path.clone(),
            create_date: Some(DateTime::now()),
        }
    }
}

// 用途：派生序列化、反序列化、克隆和调试特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试
#[derive(Serialize, Deserialize, Clone, Debug)]
// 用途：权限编辑DTO结构体
// 说明：封装编辑权限所需的参数
pub struct ResEditDTO {
    // 用途：权限ID
    // 说明：指定要编辑的权限
    pub id: Option<String>,
    // 用途：父权限ID
    // 说明：用于修改权限的层级关系
    pub parent_id: Option<String>,
    // 用途：权限名称
    // 说明：用于修改权限名称
    pub name: Option<String>,
    // 用途：权限标识
    // 说明：用于修改权限标识
    pub permission: Option<String>,
    // 用途：权限路径
    // 说明：用于修改API路径与权限的映射
    pub path: Option<String>,
}

// 用途：实现&ResEditDTO到RbacPermission的转换
// 说明：方便将编辑DTO引用转换为权限表实体
impl From<&ResEditDTO> for RbacPermission {
    // 用途：转换方法
    // 说明：将编辑DTO引用转换为权限表实体
    fn from(arg: &ResEditDTO) -> Self {
        RbacPermission {
            id: arg.id.clone(),
            name: arg.name.clone(),
            permission: arg.permission.clone(),
            path: arg.path.clone(),
            create_date: None,
        }
    }
}

// 用途：派生序列化、反序列化、克隆和调试特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试
#[derive(Serialize, Deserialize, Clone, Debug)]
// 用途：角色分页查询DTO结构体
// 说明：封装角色分页查询所需的参数
pub struct RolePageDTO {
    // 用途：页码
    // 说明：指定查询的页码，用于分页显示
    pub page_no: Option<u64>,
    // 用途：每页大小
    // 说明：指定每页显示的数据条数，用于分页显示
    pub page_size: Option<u64>,
    // 用途：角色名称
    // 说明：用于根据名称筛选角色
    pub name: Option<String>,
}

// 用途：实现&RolePageDTO到PageRequest的转换
// 说明：方便将DTO引用转换为分页请求对象
impl From<&RolePageDTO> for PageRequest {
    // 用途：转换方法
    // 说明：将DTO中的分页参数转换为PageRequest
    fn from(arg: &RolePageDTO) -> Self {
        PageRequest::new(arg.page_no.unwrap_or(1), arg.page_size.unwrap_or(10))
    }
}

// 用途：派生序列化、反序列化、克隆和调试特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试
#[derive(Serialize, Deserialize, Clone, Debug)]
// 用途：角色添加DTO结构体
// 说明：封装添加角色所需的参数
pub struct RoleAddDTO {
    // 用途：角色名称
    // 说明：用于标识角色
    pub name: Option<String>,
    // 用途：父角色ID
    // 说明：用于构建角色的层级关系，可空
    pub parent_id: Option<String>,
}

// 用途：实现RoleAddDTO到RbacRole的转换
// 说明：方便将添加DTO转换为角色表实体
impl From<RoleAddDTO> for RbacRole {
    // 用途：转换方法
    // 说明：将添加DTO转换为角色表实体，自动生成ID和创建时间
    fn from(arg: RoleAddDTO) -> Self {
        RbacRole {
            id: Ulid::new().to_string().into(),
            name: arg.name,
            create_date: DateTime::now().into(),
        }
    }
}

// 用途：派生序列化、反序列化、克隆和调试特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试
#[derive(Serialize, Deserialize, Clone, Debug)]
// 用途：角色编辑DTO结构体
// 说明：封装编辑角色所需的参数
pub struct RoleEditDTO {
    // 用途：角色ID
    // 说明：指定要编辑的角色
    pub id: Option<String>,
    // 用途：角色名称
    // 说明：用于修改角色名称
    pub name: Option<String>,
    // 用途：父角色ID
    // 说明：用于修改角色的层级关系
    pub parent_id: Option<String>,
    // 用途：权限ID列表
    // 说明：用于关联角色和权限
    pub permission_ids: Vec<String>,
}

// 用途：实现RoleEditDTO到RbacRole的转换
// 说明：方便将编辑DTO转换为角色表实体
impl From<RoleEditDTO> for RbacRole {
    // 用途：转换方法
    // 说明：将编辑DTO转换为角色表实体
    fn from(arg: RoleEditDTO) -> Self {
        RbacRole {
            id: arg.id,
            name: arg.name,
            create_date: None,
        }
    }
}

// 用途：派生序列化、反序列化、克隆和调试特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试
#[derive(Serialize, Deserialize, Clone, Debug)]
// 用途：系统角色资源添加DTO结构体
// 说明：封装添加角色及其关联资源所需的参数
pub struct SysRoleResAddDTO {
    // 用途：角色名称
    // 说明：用于标识角色
    pub name: Option<String>,
    // 用途：父角色ID
    // 说明：用于构建角色的层级关系，可空
    pub parent_id: Option<String>,
    // 用途：权限ID列表
    // 说明：用于关联角色和权限
    pub permission_ids: Vec<String>,
}

// 用途：实现SysRoleResAddDTO到RoleAddDTO的转换
// 说明：方便将系统角色资源添加DTO转换为角色添加DTO
impl From<SysRoleResAddDTO> for RoleAddDTO {
    // 用途：转换方法
    // 说明：从系统角色资源添加DTO中提取角色添加所需的参数
    fn from(arg: SysRoleResAddDTO) -> Self {
        Self {
            name: arg.name,
            parent_id: arg.parent_id,
        }
    }
}

// 用途：派生序列化、反序列化、克隆和调试特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试
#[derive(Serialize, Deserialize, Clone, Debug)]
// 用途：系统角色资源更新DTO结构体
// 说明：封装更新角色及其关联资源所需的参数
pub struct SysRoleResUpdateDTO {
    // 用途：角色ID
    // 说明：指定要更新的角色
    pub id: Option<String>,
    // 用途：角色名称
    // 说明：用于修改角色名称
    pub name: Option<String>,
    // 用途：父角色ID
    // 说明：用于修改角色的层级关系
    pub parent_id: Option<String>,
    // 用途：权限ID列表
    // 说明：用于更新角色关联的权限
    pub permission_ids: Vec<String>,
}

// 用途：实现SysRoleResUpdateDTO到RoleEditDTO的转换
// 说明：方便将系统角色资源更新DTO转换为角色编辑DTO
impl From<SysRoleResUpdateDTO> for RoleEditDTO {
    // 用途：转换方法
    // 说明：从系统角色资源更新DTO中提取角色编辑所需的参数
    fn from(arg: SysRoleResUpdateDTO) -> Self {
        Self {
            id: arg.id,
            name: arg.name,
            permission_ids: arg.permission_ids,
            parent_id: arg.parent_id,
        }
    }
}

// 用途：派生序列化、反序列化、克隆和调试特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试
#[derive(Serialize, Deserialize, Clone, Debug)]
// 用途：系统角色资源分页查询DTO结构体
// 说明：封装系统角色资源分页查询所需的参数
pub struct SysRoleResPageDTO {
    // 用途：页码
    // 说明：指定查询的页码，用于分页显示
    pub page_no: Option<u64>,
    // 用途：每页大小
    // 说明：指定每页显示的数据条数，用于分页显示
    pub page_size: Option<u64>,
    // 用途：角色名称
    // 说明：用于根据名称筛选角色
    pub name: Option<String>,
}

// 用途：实现&SysRoleResPageDTO到PageRequest的转换
// 说明：方便将DTO引用转换为分页请求对象
impl From<&SysRoleResPageDTO> for PageRequest {
    // 用途：转换方法
    // 说明：将DTO中的分页参数转换为PageRequest
    fn from(arg: &SysRoleResPageDTO) -> Self {
        PageRequest::new(arg.page_no.unwrap_or(1), arg.page_size.unwrap_or(10))
    }
}

// 用途：派生序列化、反序列化、克隆和调试特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试
#[derive(Serialize, Deserialize, Clone, Debug)]
// 用途：用户角色添加DTO结构体
// 说明：封装添加用户角色关联所需的参数
pub struct UserRoleAddDTO {
    // 用途：关联ID
    // 说明：用于标识用户角色关联记录
    pub id: Option<String>,
    // 用途：用户ID
    // 说明：用于关联用户
    pub user_id: Option<String>,
    // 用途：角色ID
    // 说明：用于关联角色
    pub role_id: Option<String>,
}

// 用途：实现UserRoleAddDTO到RbacUserRole的转换
// 说明：方便将添加DTO转换为用户角色关联表实体
impl From<UserRoleAddDTO> for RbacUserRole {
    // 用途：转换方法
    // 说明：将添加DTO转换为用户角色关联表实体，自动生成创建时间
    fn from(arg: UserRoleAddDTO) -> Self {
        RbacUserRole {
            id: arg.id.clone(),
            user_id: arg.user_id.clone(),
            role_id: arg.role_id.clone(),
            create_date: DateTime::now().into(),
        }
    }
}
