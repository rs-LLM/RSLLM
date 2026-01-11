// 用途：导入全局上下文
// 说明：用于访问其他服务
use crate::context::CONTEXT;

// 用途：导入角色资源相关的数据传输对象
// 说明：用于接收角色资源的添加、编辑和分页查询请求参数
use crate::domain::dto::rbac::{
    RoleAddDTO, RoleEditDTO, RolePageDTO, SysRoleResAddDTO, SysRoleResPageDTO, SysRoleResUpdateDTO,
};

// 用途：导入角色权限关联表结构
// 说明：用于数据库操作
use crate::domain::table::rbac::RbacRolePermission;

// 用途：导入角色VO
// 说明：用于返回角色数据
use crate::domain::vo::rbac::SysRoleVO;

// 用途：导入自定义错误类型
// 说明：用于处理错误情况
use crate::error::Error;

// 用途：导入自定义结果类型
// 说明：用于统一错误处理
use crate::error::Result;

// 用途：导入错误信息宏和数据库连接池宏
// 说明：用于生成错误信息和获取数据库连接
use crate::{error_info, pool};

// 用途：导入分页相关类型
// 说明：用于处理分页查询
use rbatis::Page;

// 用途：导入Ulid生成器
// 说明：用于生成唯一ID
use ulid::Ulid;

// 用途：导入日期时间类型
// 说明：用于记录创建时间
use rbatis::rbdc::DateTime;

// 用途：导入rbs的value宏
// 说明：用于构建查询条件
use rbs::value;

/// 用途：角色资源服务
/// 说明：处理角色和权限的关联关系，如角色权限的添加、编辑、删除等
#[derive(Clone)]
pub struct RbacRolePermissionService {}

impl RbacRolePermissionService {
    /// 用途：分页查询角色资源
    /// 说明：从数据库中分页获取角色数据，用于角色列表展示
    pub async fn page(&self, arg: &SysRoleResPageDTO) -> Result<Page<SysRoleVO>> {
        // 用途：调用角色服务查询角色分页数据
        // 说明：复用角色服务的分页查询功能
        let role_page = CONTEXT
            .rbac_role_service
            .page(&RolePageDTO {
                page_no: arg.page_no,
                page_size: arg.page_size,
                name: arg.name.clone(),
            })
            .await?;
        // 用途：返回角色分页结果
        // 说明：告知调用者查询成功并返回数据
        Ok(role_page)
    }

    /// 用途：根据角色ID查询角色权限关联
    /// 说明：获取角色的所有权限关联信息
    pub async fn find_by_role_ids(
        &self,
        role_ids: &Vec<String>,
    ) -> Result<Vec<RbacRolePermission>> {
        // 用途：检查角色ID列表是否为空
        // 说明：空列表直接返回空结果
        if role_ids.is_empty() {
            return Ok(vec![]);
        }
        // 用途：根据角色ID列表查询角色权限关联
        // 说明：从数据库中获取指定角色的权限关联
        let datas = RbacRolePermission::select_by_role_ids(pool!(), role_ids).await?;
        // 用途：返回查询结果
        // 说明：告知调用者查询成功并返回数据
        Ok(datas)
    }

    /// 用途：添加角色资源
    /// 说明：创建角色并关联权限
    pub async fn add(&self, arg: &SysRoleResAddDTO) -> Result<u64> {
        // 用途：添加角色
        // 说明：先创建角色，再关联权限
        let (_, role_id) = CONTEXT
            .rbac_role_service
            .add(RoleAddDTO::from(arg.clone()))
            .await?;
        // 用途：保存角色权限关联
        // 说明：将角色与权限关联起来
        self.save_resources(&role_id, arg.permission_ids.clone())
            .await
    }

    /// 用途：编辑角色资源
    /// 说明：更新角色信息并重新关联权限
    pub async fn edit(&self, arg: &SysRoleResUpdateDTO) -> Result<u64> {
        // 用途：获取角色ID
        // 说明：用于后续操作
        let role_id = arg
            .id
            .as_ref()
            .ok_or_else(|| Error::from(error_info!("role_id_empty")))?;
        // 用途：编辑角色信息
        // 说明：更新角色的基本信息
        CONTEXT
            .rbac_role_service
            .edit(RoleEditDTO::from(arg.clone()))
            .await?;
        // 用途：保存角色权限关联
        // 说明：重新关联角色的权限
        self.save_resources(role_id, arg.permission_ids.clone())
            .await
    }

    /// 用途：保存角色资源关联
    /// 说明：将角色与权限关联起来，先删除旧关联，再添加新关联
    async fn save_resources(&self, role_id: &str, permission_ids: Vec<String>) -> Result<u64> {
        // 用途：删除旧的角色权限关联
        // 说明：先清除旧关联，避免重复关联
        self.remove_by_role_id(role_id).await?;
        // 用途：创建角色权限关联列表
        // 说明：用于批量插入角色权限关联
        let mut sys_role_permission = Vec::with_capacity(permission_ids.len());
        // 用途：遍历权限ID列表
        // 说明：为每个权限创建角色权限关联
        for resource_id in permission_ids {
            sys_role_permission.push(RbacRolePermission {
                // 用途：生成唯一ID
                // 说明：为每个角色权限关联创建唯一标识
                id: Ulid::new().to_string().into(),
                // 用途：设置角色ID
                // 说明：关联角色
                role_id: role_id.to_string().into(),
                // 用途：设置权限ID
                // 说明：关联权限
                permission_id: resource_id.clone().into(),
                // 用途：设置创建时间
                // 说明：记录角色权限关联的创建时间
                create_date: DateTime::now().into(),
            });
        }
        // 用途：批量插入角色权限关联
        // 说明：一次性插入所有角色权限关联，提高效率
        Ok(
            RbacRolePermission::insert_batch(pool!(), &sys_role_permission, 20)
                .await?
                .rows_affected,
        )
    }

    /// 用途：删除角色及其关联
    /// 说明：删除角色的同时，删除关联的用户角色关系和角色权限关系
    pub async fn remove_role(&self, role_id: &str) -> Result<u64> {
        // 用途：删除角色
        // 说明：删除角色本身
        let remove_roles = CONTEXT.rbac_role_service.remove(role_id).await?;
        // 用途：删除用户角色关联
        // 说明：级联删除用户与该角色的关联
        let remove_user_roles = CONTEXT
            .rbac_user_role_service
            .remove_by_role_id(role_id)
            .await?;
        // 用途：删除角色权限关联
        // 说明：级联删除角色与权限的关联
        let remove_role_res = CONTEXT
            .rbac_role_permission_service
            .remove_by_role_id(role_id)
            .await?;
        // 用途：返回删除总数
        // 说明：告知调用者总共删除了多少条数据
        Ok(remove_roles + remove_user_roles + remove_role_res)
    }

    /// 用途：删除角色权限关联
    /// 说明：根据ID删除特定的角色权限关联
    pub async fn remove(&self, id: &str) -> Result<u64> {
        // 用途：根据ID删除角色权限关联
        // 说明：从数据库中删除指定ID的角色权限关联
        Ok(RbacRolePermission::delete_by_map(pool!(), value! {"id":id})
            .await?
            .rows_affected)
    }

    /// 用途：根据权限ID删除角色权限关联
    /// 说明：当权限被删除时，级联删除相关的角色权限关联
    pub async fn remove_by_permission_id(&self, permission_id: &str) -> Result<u64> {
        // 用途：根据权限ID删除角色权限关联
        // 说明：从数据库中删除所有关联该权限的角色权限关联
        Ok(
            RbacRolePermission::delete_by_map(pool!(), value! {"permission_id": permission_id})
                .await?
                .rows_affected,
        )
    }

    /// 用途：根据角色ID删除角色权限关联
    /// 说明：当角色被删除或需要重新关联权限时，删除旧的角色权限关联
    pub async fn remove_by_role_id(&self, role_id: &str) -> Result<u64> {
        // 用途：根据角色ID删除角色权限关联
        // 说明：从数据库中删除所有关联该角色的角色权限关联
        Ok(
            RbacRolePermission::delete_by_map(pool!(), value! {"role_id": role_id})
                .await?
                .rows_affected,
        )
    }
}
