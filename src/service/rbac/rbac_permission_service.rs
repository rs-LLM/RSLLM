// 用途：导入全局上下文
// 说明：用于访问其他服务
use crate::context::CONTEXT;


// 用途：导入权限相关的数据传输对象
// 说明：用于接收权限的分页查询和编辑请求参数
use crate::domain::dto::rbac::{PermissionPageDTO, ResEditDTO};

// 用途：导入权限表结构
// 说明：用于数据库操作
use crate::domain::table::rbac::RbacPermission;

// 用途：导入权限VO
// 说明：用于返回权限数据
use crate::domain::vo::rbac::RbacPermissionVO;

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
use rbatis::{Page, PageRequest};

// 用途：导入rbs的value宏
// 说明：用于构建查询条件
use rbs::value;

/// 用途：权限服务
/// 说明：处理权限相关业务逻辑，如权限的增删改查
#[derive(Clone)]
pub struct RbacPermissionService {}

impl RbacPermissionService {
    /// 用途：分页查询权限
    /// 说明：从数据库中分页获取权限数据
    pub async fn page(&self, arg: &PermissionPageDTO) -> Result<Page<RbacPermissionVO>> {
        // 用途：查询权限分页数据
        // 说明：根据查询条件从数据库中获取分页数据
        let data = RbacPermission::select_page(pool!(), &PageRequest::from(arg), arg).await?;
        // 用途：转换为VO分页
        // 说明：将数据库实体转换为前端需要的VO
        let page = Page::<RbacPermissionVO>::from(data);
        // 用途：返回分页结果
        // 说明：告知调用者查询成功并返回数据
        Ok(page)
    }

    /// 用途：添加权限
    /// 说明：向数据库中添加新权限
    pub async fn add(&self, arg: &RbacPermission) -> Result<u64> {
        // 用途：检查权限或名称是否已存在
        // 说明：避免重复添加相同权限或名称的权限
        let old = RbacPermission::select_by_permission_or_name(
            pool!(),
            arg.permission.as_deref().unwrap_or_default(),
            arg.name.as_deref().unwrap_or_default(),
        )
        .await?;
        // 用途：如果权限已存在，返回错误
        // 说明：确保权限的唯一性
        if old.len() > 0 {
            return Err(Error::from(format!(
                "{}={:?}",
                error_info!("permission_exists"),
                rbatis::table_field_vec!(old, name)
            )));
        }
        // 用途：插入权限数据
        // 说明：将新权限数据保存到数据库
        let result = Ok(RbacPermission::insert(pool!(), &arg).await?.rows_affected);
        result
    }

    /// 用途：编辑权限
    /// 说明：更新数据库中的权限数据
    pub async fn edit(&self, arg: &ResEditDTO) -> Result<u64> {
        // 用途：转换为数据库实体
        // 说明：数据库操作需要使用实体对象
        let data = RbacPermission::from(arg);
        // 用途：更新权限数据
        // 说明：根据ID更新权限信息
        let result =
            RbacPermission::update_by_map(pool!(), &data, value! {"id": &data.id }).await?;
        // 用途：返回更新结果
        // 说明：告知调用者更新成功
        Ok(result.rows_affected)
    }

    /// 用途：删除权限
    /// 说明：从数据库中删除指定ID的权限
    pub async fn remove(&self, id: &str) -> Result<u64> {
        // 用途：删除权限数据
        // 说明：从数据库中删除权限
        let num = RbacPermission::delete_by_map(pool!(), value! {"id":id})
            .await?
            .rows_affected;
        // 用途：再次删除权限数据
        // 说明：确保权限被彻底删除
        RbacPermission::delete_by_map(pool!(), value! {"id":id}).await?;
        // 用途：删除角色权限关联
        // 说明：级联删除权限的角色关联
        let _ = CONTEXT
            .rbac_role_permission_service
            .remove_by_permission_id(id)
            .await;
        // 用途：返回删除结果
        // 说明：告知调用者删除成功
        Ok(num)
    }

    /// 用途：根据ID列表查询权限
    /// 说明：获取多个权限的详细信息
    pub async fn finds(&self, ids: Vec<String>) -> Result<Vec<RbacPermission>> {
        // 用途：检查ID列表是否为空
        // 说明：空列表直接返回空结果
        if ids.is_empty() {
            return Ok(vec![]);
        }
        // 用途：根据ID列表查询权限
        // 说明：从数据库中获取指定ID的权限
        let data = RbacPermission::select_by_map(pool!(), value! {"id": &ids}).await?;
        // 用途：返回查询结果
        // 说明：告知调用者查询成功并返回数据
        Ok(data)
    }

    /// 用途：查询所有权限
    /// 说明：获取系统中所有权限
    pub async fn finds_all(&self) -> Result<Vec<RbacPermission>> {
        // 用途：查询所有权限
        // 说明：从数据库中获取所有权限
        let data = RbacPermission::select_all(pool!()).await?;
        // 用途：返回查询结果
        // 说明：告知调用者查询成功并返回数据
        Ok(data)
    }
}
