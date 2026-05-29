// 用途：导入全局上下文实例
// 说明：用于访问其他服务实例
use crate::context::CONTEXT;
// 用途：导入用户角色添加的数据传输对象
// 说明：用于接收用户角色关联的请求数据
use crate::domain::dto::rbac::UserRoleAddDTO;
// 用途：导入用户角色表相关类型和扩展方法
// 说明：用于数据库操作和数据转换
use crate::domain::table::rbac::{IntoMap, IntoMapVec, RbacUserRole};
// 用途：导入权限视图对象
// 说明：用于返回给客户端的权限数据格式化
use crate::domain::vo::rbac::RbacPermissionVO;
// 用途：导入系统角色视图对象
// 说明：用于返回给客户端的角色数据格式化
use crate::domain::vo::rbac::SysRoleVO;
// 用途：导入自定义错误类型
// 说明：用于错误处理和返回
use crate::error::Error;
// 用途：导入自定义结果类型
// 说明：用于方法返回值的统一处理
use crate::error::Result;
// 用途：导入错误信息和数据库连接池
// 说明：用于获取数据库连接和错误信息
use crate::{error_info, pool};
// 用途：导入对象ID生成器
// 说明：用于生成唯一的ID
use ulid::Ulid;
// 用途：导入rbs值类型
// 说明：用于构建数据库查询条件
use rbs::value;
// 用途：导入HashMap集合
// 说明：用于数据映射和缓存
use std::collections::HashMap;

// 用途：用户角色设置视图对象
// 说明：用于返回用户及其角色的关联信息
#[derive(Clone)]
pub struct SetUserVO {
    // 用途：用户ID
    // 说明：标识要设置角色的用户
    pub id: Option<String>,
    // 用途：用户角色列表
    // 说明：存储用户关联的角色信息
    pub roles: Vec<SysRoleVO>,
}

// 用途：用户角色服务结构体
// 说明：封装用户角色关联的业务逻辑
#[derive(Clone)]
pub struct RbacUserRoleService {}

impl RbacUserRoleService {
    // 用途：为用户列表设置角色信息
    // 说明：批量获取并设置用户的角色及其权限信息
    pub async fn set_roles(&self, records: &mut Vec<SetUserVO>) -> Result<()> {
        use log::debug;

        // 用途：提取所有用户ID
        // 说明：用于批量查询用户角色关联
        let user_ids = rbatis::table_field_vec!(&*records, id);
        debug!("[set_roles] 查询用户角色关联，用户IDs: {:?}", user_ids);
        // 用途：根据用户ID查询用户角色关联
        // 说明：获取用户与角色的关联关系
        let user_roles =
            RbacUserRole::select_by_map(pool!(), value! {"user_id": &user_ids}).await?;
        debug!("[set_roles] 查询到 {} 条用户角色关联记录", user_roles.len());
        // 用途：提取所有角色ID
        // 说明：用于查询角色的权限信息
        let role_ids = rbatis::table_field_vec!(&user_roles, role_id)
            .into_iter()
            .map(|v| v.to_string())
            .collect();
        debug!("[set_roles] 角色IDs: {:?}", role_ids);
        // 用途：构建用户ID到角色列表的映射
        // 说明：方便后续快速查询用户关联的角色
        let user_id_map = user_roles.into_map(|v| v.user_id.clone().unwrap_or_default());

        // 用途：查询角色权限关联
        // 说明：获取角色与权限的关联关系
        let role_perms = CONTEXT
            .rbac_role_permission_service
            .find_by_role_ids(&role_ids)
            .await?;
        // 用途：提取所有权限ID
        // 说明：用于查询权限详情
        let perm_ids: Vec<String> = rbatis::table_field_vec!(&role_perms, permission_id)
            .into_iter()
            .map(|v| v.to_string())
            .collect();
        // 用途：查询权限详情
        // 说明：获取权限的完整信息
        let perms = CONTEXT
            .rbac_permission_service
            .finds(perm_ids)
            .await?
            .into_map(|v| v.id.clone().unwrap_or_default());
        // 用途：构建角色ID到权限列表的映射
        // 说明：方便后续快速查询角色关联的权限
        let mut role_perms_map = HashMap::new();
        for x in &role_perms {
            role_perms_map.insert(x.role_id.clone().unwrap_or_default(), vec![]);
        }
        // 用途：填充角色权限映射
        // 说明：将权限信息关联到对应的角色
        for x in role_perms {
            if let Some(v) = perms.get(&x.permission_id.clone().unwrap_or_default())
                && let Some(vec) = role_perms_map.get_mut(x.role_id.as_deref().unwrap_or_default())
            {
                vec.push(v.clone());
            }
        }
        // 用途：查询角色详情
        // 说明：获取角色的完整信息
        let role_map = CONTEXT
            .rbac_role_service
            .finds(&role_ids)
            .await?
            .into_map(|v| v.id.clone().unwrap_or_default());
        // 用途：为每个用户设置角色及其权限信息
        // 说明：将查询到的角色和权限信息填充到用户视图对象中
        for x in records {
            let user_id = x.id.as_deref().unwrap_or_default();
            let default_user_roles = vec![];
            let user_roles = user_id_map.get(user_id).unwrap_or(&default_user_roles);
            let mut roles = Vec::with_capacity(user_roles.len());
            for x in user_roles {
                let role_id = x.role_id.clone().unwrap_or_default();
                if let Some(v) = role_map.get(&role_id) {
                    let mut role = SysRoleVO::from(v.clone());
                    if let Some(perms) = role_perms_map.get(&role_id) {
                        let mut perm_vos = Vec::with_capacity(perms.len());
                        for x in perms {
                            perm_vos.push(RbacPermissionVO::from(x.clone()));
                        }
                        role.set_permissions(perm_vos);
                    } else {
                        role.set_permissions(vec![]);
                    }
                    roles.push(role);
                }
            }
            x.roles = roles;
        }
        Ok(())
    }

    // 用途：添加用户角色关联
    // 说明：为用户分配角色，先删除旧关联再添加新关联
    pub async fn add(&self, arg: UserRoleAddDTO) -> Result<u64> {
        // 用途：验证用户ID和角色ID是否为空
        // 说明：确保关联关系的完整性
        if arg.user_id.is_none() || arg.role_id.is_none() {
            return Err(Error::from(error_info!("role_user_cannot_empty")));
        }
        // 用途：获取用户ID字符串
        // 说明：用于后续删除操作
        let user_id = arg.user_id.as_deref().unwrap_or_default().to_string();
        // 用途：将DTO转换为实体对象
        // 说明：用于数据库插入操作
        let mut role = RbacUserRole::from(arg);
        // 用途：生成唯一ID
        // 说明：确保实体对象有唯一标识
        if role.id.is_none() {
            role.id = Some(Ulid::new().to_string());
        }
        // 用途：删除用户的旧角色关联
        // 说明：确保用户只有最新分配的角色
        self.remove_by_user_id(user_id.as_str()).await?;
        // 用途：插入新的用户角色关联
        // 说明：保存用户的角色分配
        Ok(RbacUserRole::insert(pool!(), &role).await?.rows_affected)
    }

    // 用途：根据角色ID删除用户角色关联
    // 说明：当角色被删除时，同时删除用户与该角色的关联
    pub async fn remove_by_role_id(&self, role_id: &str) -> Result<u64> {
        Ok(
            RbacUserRole::delete_by_map(pool!(), value! {"role_id": role_id})
                .await?
                .rows_affected,
        )
    }

    // 用途：根据用户ID删除用户角色关联
    // 说明：当用户被删除或需要重新分配角色时，删除旧的关联
    pub async fn remove_by_user_id(&self, user_id: &str) -> Result<u64> {
        Ok(
            RbacUserRole::delete_by_map(pool!(), value! {"user_id": user_id})
                .await?
                .rows_affected,
        )
    }

    // 用途：查询用户的角色信息
    // 说明：获取用户关联的角色及其权限
    pub async fn find_user_role(&self, user_id: &str) -> Result<Vec<SysRoleVO>> {
        // 用途：验证用户ID是否为空
        // 说明：避免无效查询
        if user_id.is_empty() {
            return Ok(vec![]);
        }
        // 用途：创建临时用户角色视图对象
        // 说明：用于调用set_roles方法获取角色信息
        let mut role_vos = vec![];
        role_vos.push(SetUserVO {
            id: Some(user_id.to_string()),
            roles: vec![],
        });
        // 用途：设置用户角色信息
        // 说明：填充角色及其权限信息
        self.set_roles(&mut role_vos).await?;
        // 用途：返回用户角色列表
        // 说明：返回查询结果
        Ok(role_vos.remove(0).roles)
    }
}
