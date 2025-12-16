// 用途：导入全局上下文
// 说明：用于访问其他服务
use crate::context::CONTEXT;

// 用途：导入角色相关的数据传输对象
// 说明：用于接收角色的添加、编辑和分页查询请求参数
use crate::domain::dto::rbac::{RoleAddDTO, RoleEditDTO, RolePageDTO};

// 用途：导入IntoMap特性
// 说明：用于将向量转换为映射
use crate::domain::table::rbac::IntoMap;

// 用途：导入角色相关的表结构
// 说明：用于数据库操作
use crate::domain::table::rbac::{RbacPermission, RbacRole, RbacRolePermission};

// 用途：导入权限VO
// 说明：用于返回权限数据
use crate::domain::vo::rbac::RbacPermissionVO;

// 用途：导入角色VO
// 说明：用于返回角色数据
use crate::domain::vo::rbac::SysRoleVO;

// 用途：导入自定义结果类型
// 说明：用于统一错误处理
use crate::error::Result;

// 用途：导入数据库连接池宏
// 说明：用于获取数据库连接
use crate::pool;

// 用途：导入分页相关类型
// 说明：用于处理分页查询
use rbatis::{Page, PageRequest};

// 用途：导入rbs的value宏
// 说明：用于构建查询条件
use rbs::value;

// 用途：导入集合类型
// 说明：用于存储角色权限关系
use std::collections::{HashMap, HashSet};

/// 用途：角色服务
/// 说明：处理角色相关业务逻辑，如角色的增删改查和权限管理
pub struct RbacRoleService {}

impl RbacRoleService {
    pub async fn page(&self, arg: &RolePageDTO) -> Result<Page<SysRoleVO>> {
        // 用途：根据名称分页查询角色
        // 说明：从数据库中获取角色数据
        let data = RbacRole::select_page_by_name(
            pool!(),
            &PageRequest::from(arg),
            arg.name.as_deref().unwrap_or_default(),
        )
        .await?;
        
        // 用途：提取角色ID集合
        // 说明：用于查询角色权限关系
        let role_ids: Vec<String> = rbatis::table_field_set!(&data.records, id)
            .iter()
            .map(|v| v.to_string())
            .collect();
        
        // 用途：查询角色权限关系
        // 说明：获取角色和权限的关联关系
        let role_perms = CONTEXT
            .rbac_role_permission_service
            .find_by_role_ids(&role_ids)
            .await?;
        
        // 用途：提取权限ID集合
        // 说明：用于查询权限详细信息
        let perm_ids: Vec<String> = rbatis::table_field_set!(&role_perms, permission_id)
            .iter()
            .map(|v| v.to_string())
            .collect();
        
        // 用途：根据权限ID查询权限信息
        // 说明：获取权限的详细信息
        let perm_map = CONTEXT
            .rbac_permission_service
            .finds(perm_ids)
            .await?
            .into_map(|v| v.id.clone().unwrap_or_default());
        
        // 用途：构建角色权限映射
        // 说明：将角色ID映射到其拥有的权限集合
        let role_perms = {
            let mut map = HashMap::<String, HashSet<RbacPermission>>::new();
            for x in role_perms {
                let role_id = x.role_id.clone().unwrap_or_default();
                if !map.contains_key(&role_id) {
                    map.insert(role_id.clone(), HashSet::new());
                }
                if let Some(role_perms) = map.get_mut(&role_id) {
                    if let Some(v) = perm_map.get(x.permission_id.as_deref().unwrap_or_default()) {
                        role_perms.insert(v.clone());
                    }
                }
            }
            map
        };
        
        // 用途：转换为角色VO分页
        // 说明：将数据库实体转换为前端需要的VO
        let mut page = Page::<SysRoleVO>::from(data);
        
        // 用途：为每个角色设置权限
        // 说明：将权限信息添加到角色VO中
        for vo in &mut page.records {
            if let Some(perms) = role_perms.get(vo.id.as_deref().unwrap_or_default()) {
                vo.set_permissions(
                    perms
                        .iter()
                        .map(|v| RbacPermissionVO::from(v.clone()))
                        .collect(),
                );
            }
        }
        
        // 用途：返回带权限信息的角色分页数据
        // 说明：告知调用者查询成功并返回数据
        Ok(page)
    }

    /// 用途：添加角色
    /// 说明：向数据库中添加新角色
    pub async fn add(&self, arg: RoleAddDTO) -> Result<(u64, String)> {
        // 用途：转换为数据库实体
        // 说明：数据库操作需要使用实体对象
        let role = RbacRole::from(arg);
        // 用途：插入角色数据
        // 说明：将新角色数据保存到数据库
        let result = (
            RbacRole::insert(pool!(), &role).await?.rows_affected,
            role.id.clone().unwrap_or_default(),
        );
        // 用途：返回添加结果
        // 说明：告知调用者添加成功并返回角色ID
        Ok(result)
    }

    /// 用途：编辑角色
    /// 说明：更新数据库中的角色数据
    pub async fn edit(&self, arg: RoleEditDTO) -> Result<u64> {
        // 用途：转换为数据库实体
        // 说明：数据库操作需要使用实体对象
        let role = RbacRole::from(arg);
        // 用途：更新角色数据
        // 说明：根据ID更新角色信息
        let result = RbacRole::update_by_map(pool!(), &role, value! {"id": &role.id}).await;
        // 用途：返回更新结果
        // 说明：告知调用者更新成功
        Ok(result?.rows_affected)
    }

    /// 用途：删除角色
    /// 说明：从数据库中删除指定ID的角色
    pub async fn remove(&self, id: &str) -> Result<u64> {
        // 用途：删除角色数据
        // 说明：从数据库中删除角色
        let result = RbacRole::delete_by_map(pool!(), value! {"id": id}).await?;
        // 用途：返回删除结果
        // 说明：告知调用者删除成功
        Ok(result.rows_affected)
    }

    /// 用途：根据ID列表查询角色
    /// 说明：获取多个角色的详细信息
    pub async fn finds(&self, ids: &Vec<String>) -> Result<Vec<RbacRole>> {
        // 用途：检查ID列表是否为空
        // 说明：空列表直接返回空结果
        if ids.is_empty() {
            return Ok(vec![]);
        }
        // 用途：根据ID列表查询角色
        // 说明：从数据库中获取指定ID的角色
        Ok(RbacRole::select_by_map(pool!(), value! {"id":ids}).await?)
    }

    /// 用途：查询角色权限资源
    /// 说明：获取角色的权限关联信息
    pub async fn find_role_res(&self, role_ids: &Vec<String>) -> Result<Vec<RbacRolePermission>> {
        // 用途：检查角色ID列表是否为空
        // 说明：空列表直接返回空结果
        if role_ids.is_empty() {
            return Ok(vec![]);
        }
        // 用途：根据角色ID列表查询角色权限关联
        // 说明：获取角色和权限的关联关系
        Ok(RbacRolePermission::select_by_map(pool!(), value! {"role_id":role_ids}).await?)
    }

    /// 用途：查询所有角色
    /// 说明：获取系统中所有角色
    pub async fn find_all(&self) -> Result<Vec<RbacRole>> {
        // 用途：查询所有角色
        // 说明：从数据库中获取所有角色数据
        Ok(RbacRole::select_all(pool!()).await?)
    }
}
