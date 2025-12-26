// 用途：导入权限审计日志表结构
// 说明：用于数据库操作
use crate::domain::table::rbac::RbacPermissionAuditLog;

// 用途：导入权限表结构
// 说明：用于记录权限数据
use crate::domain::table::rbac::RbacPermission;

// 用途：导入自定义结果类型
// 说明：用于统一错误处理
use crate::error::Result;

// 用途：导入数据库连接池宏
// 说明：用于获取数据库连接
use crate::pool;

// 用途：导入rbs的value宏
// 说明：用于构建查询条件
use rbs::value;

// 用途：导入日期时间类型
// 说明：用于记录创建时间
use rbatis::rbdc::DateTime;

/// 用途：权限审计日志服务
/// 说明：处理权限审计日志相关业务逻辑，如记录权限的创建、修改、删除操作
#[derive(Clone)]
pub struct RbacPermissionAuditLogService {}

impl RbacPermissionAuditLogService {
    /// 用途：记录权限创建
    /// 说明：记录权限创建操作的审计日志
    pub async fn log_create(
        &self,
        permission_id: &str,
        new_value: &RbacPermission,
        operator_id: &str,
        operator_name: &str,
    ) -> Result<u64> {
        let log = RbacPermissionAuditLog {
            id: Some(uuid::Uuid::new_v4().to_string()),
            permission_id: Some(permission_id.to_string()),
            operation_type: Some("create".to_string()),
            old_value: None,
            new_value: Some(serde_json::to_string(new_value).unwrap_or_default()),
            operator_id: Some(operator_id.to_string()),
            operator_name: Some(operator_name.to_string()),
            create_time: Some(DateTime::now()),
        };
        let result = RbacPermissionAuditLog::insert(pool!(), &log).await?;
        Ok(result.rows_affected)
    }

    /// 用途：记录权限更新
    /// 说明：记录权限更新操作的审计日志
    pub async fn log_update(
        &self,
        permission_id: &str,
        old_value: &RbacPermission,
        new_value: &RbacPermission,
        operator_id: &str,
        operator_name: &str,
    ) -> Result<u64> {
        let log = RbacPermissionAuditLog {
            id: Some(uuid::Uuid::new_v4().to_string()),
            permission_id: Some(permission_id.to_string()),
            operation_type: Some("update".to_string()),
            old_value: Some(serde_json::to_string(old_value).unwrap_or_default()),
            new_value: Some(serde_json::to_string(new_value).unwrap_or_default()),
            operator_id: Some(operator_id.to_string()),
            operator_name: Some(operator_name.to_string()),
            create_time: Some(DateTime::now()),
        };
        let result = RbacPermissionAuditLog::insert(pool!(), &log).await?;
        Ok(result.rows_affected)
    }

    /// 用途：记录权限删除
    /// 说明：记录权限删除操作的审计日志
    pub async fn log_delete(
        &self,
        permission_id: &str,
        old_value: &RbacPermission,
        operator_id: &str,
        operator_name: &str,
    ) -> Result<u64> {
        let log = RbacPermissionAuditLog {
            id: Some(uuid::Uuid::new_v4().to_string()),
            permission_id: Some(permission_id.to_string()),
            operation_type: Some("delete".to_string()),
            old_value: Some(serde_json::to_string(old_value).unwrap_or_default()),
            new_value: None,
            operator_id: Some(operator_id.to_string()),
            operator_name: Some(operator_name.to_string()),
            create_time: Some(DateTime::now()),
        };
        let result = RbacPermissionAuditLog::insert(pool!(), &log).await?;
        Ok(result.rows_affected)
    }

    /// 用途：查询审计日志
    /// 说明：根据权限ID查询审计日志
    pub async fn query_logs(&self, permission_id: &str) -> Result<Vec<RbacPermissionAuditLog>> {
        let data = RbacPermissionAuditLog::select_by_map(pool!(), value! {"permission_id": permission_id}).await?;
        Ok(data)
    }

    /// 用途：查询所有审计日志
    /// 说明：获取系统中所有审计日志
    pub async fn query_all_logs(&self) -> Result<Vec<RbacPermissionAuditLog>> {
        let data = RbacPermissionAuditLog::select_all(pool!()).await?;
        Ok(data)
    }
}
