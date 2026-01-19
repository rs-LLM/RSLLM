// 用途：初始化事务管理模块
// 说明：提供初始化流程的事务性支持，确保原子性和一致性
use crate::domain::table::basic::InitTransactionLog as DBInitTransactionLog;
use log::{error, info, warn};
use rbatis::RBatis;
use rbatis::rbdc::DateTime;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 用途：事务状态枚举
/// 说明：表示事务的当前状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// 未开始
    #[serde(rename = "not_started")]
    NotStarted,
    /// 进行中
    #[serde(rename = "in_progress")]
    InProgress,
    /// 已完成
    #[serde(rename = "completed")]
    Completed,
    /// 正在回滚
    #[serde(rename = "rolling_back")]
    RollingBack,
    /// 已回滚
    #[serde(rename = "rolled_back")]
    RolledBack,
    /// 失败
    #[serde(rename = "failed")]
    Failed,
}

/// 用途：初始化事务记录
/// 说明：记录初始化事务的基本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitTransaction {
    /// 事务ID
    pub transaction_id: String,
    /// 事务状态
    pub status: TransactionStatus,
    /// 开始时间
    pub started_at: Option<DateTime>,
    /// 完成时间
    pub completed_at: Option<DateTime>,
    /// 当前步骤
    pub current_step: Option<String>,
    /// 失败原因
    pub error_message: Option<String>,
}

/// 用途：初始化事务步骤记录
/// 说明：记录每个初始化步骤的执行情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitTransactionStep {
    /// 步骤ID
    pub step_id: String,
    /// 事务ID
    pub transaction_id: String,
    /// 步骤名称
    pub step_name: String,
    /// 步骤状态
    pub status: TransactionStatus,
    /// 开始时间
    pub started_at: Option<DateTime>,
    /// 完成时间
    pub completed_at: Option<DateTime>,
    /// 错误信息
    pub error_message: Option<String>,
}

/// 用途：初始化事务日志
/// 说明：记录事务执行过程中的详细日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitTransactionLog {
    /// 日志ID
    pub log_id: String,
    /// 事务ID
    pub transaction_id: String,
    /// 日志级别
    pub level: String,
    /// 日志消息
    pub message: String,
    /// 时间戳
    pub timestamp: Option<DateTime>,
}

/// 用途：回滚操作类型
/// 说明：定义可用的回滚操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RollbackAction {
    /// 删除超级管理员
    #[serde(rename = "delete_admin")]
    DeleteAdmin {
        /// 用户账号
        account: String,
    },
    /// 删除用户角色关联
    #[serde(rename = "delete_user_role")]
    DeleteUserRole {
        /// 用户ID
        user_id: String,
    },
    /// 恢复配置文件
    #[serde(rename = "restore_config")]
    RestoreConfig {
        /// 备份配置内容
        backup_config: String,
    },
    /// 取消初始化标记
    #[serde(rename = "unmark_init")]
    UnmarkInit,
}

/// 用途：事务管理器
/// 说明：管理初始化事务的生命周期
pub struct InitTransactionManager {
    /// 数据库连接
    rb: Arc<RBatis>,
    /// 当前事务
    current_transaction: Option<InitTransaction>,
    /// 已完成的步骤
    completed_steps: Vec<InitTransactionStep>,
    /// 回滚操作列表
    rollback_actions: Vec<RollbackAction>,
    /// 事务日志
    transaction_logs: Vec<InitTransactionLog>,
}

impl InitTransactionManager {
    /// 用途：创建新的事务管理器
    pub fn new(rb: Arc<RBatis>) -> Self {
        Self {
            rb,
            current_transaction: None,
            completed_steps: Vec::new(),
            rollback_actions: Vec::new(),
            transaction_logs: Vec::new(),
        }
    }

    /// 用途：开始新事务
    pub async fn begin_transaction(&mut self) -> Result<String, String> {
        let transaction_id = ulid::Ulid::new().to_string();

        let transaction = InitTransaction {
            transaction_id: transaction_id.clone(),
            status: TransactionStatus::InProgress,
            started_at: Some(DateTime::now()),
            completed_at: None,
            current_step: None,
            error_message: None,
        };

        self.current_transaction = Some(transaction);
        self.completed_steps.clear();
        self.rollback_actions.clear();
        self.transaction_logs.clear();

        self.log_transaction(
            &transaction_id,
            "info",
            &format!("事务开始: {}", transaction_id),
        );

        info!("[InitTransaction] 事务开始: {}", transaction_id);
        Ok(transaction_id)
    }

    /// 用途：执行事务步骤
    pub async fn execute_step<F, Fut>(&mut self, step_name: &str, action: F) -> Result<(), String>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<(), String>>,
    {
        let transaction_id = self
            .current_transaction
            .as_ref()
            .ok_or("事务未开始")?
            .transaction_id
            .clone();

        let step_id = ulid::Ulid::new().to_string();

        let mut step = InitTransactionStep {
            step_id: step_id.clone(),
            transaction_id: transaction_id.clone(),
            step_name: step_name.to_string(),
            status: TransactionStatus::InProgress,
            started_at: Some(DateTime::now()),
            completed_at: None,
            error_message: None,
        };

        info!("[InitTransaction] 执行步骤: {}", step_name);

        match action().await {
            Ok(_) => {
                step.status = TransactionStatus::Completed;
                step.completed_at = Some(DateTime::now());
                self.completed_steps.push(step.clone());
                info!("[InitTransaction] 步骤完成: {}", step_name);
                Ok(())
            }
            Err(e) => {
                step.status = TransactionStatus::Failed;
                step.completed_at = Some(DateTime::now());
                step.error_message = Some(e.clone());
                self.completed_steps.push(step);
                error!("[InitTransaction] 步骤失败: {}, 错误: {}", step_name, e);
                Err(e)
            }
        }
    }

    /// 用途：添加回滚操作
    pub fn add_rollback_action(&mut self, action: RollbackAction) {
        self.rollback_actions.push(action);
    }

    /// 用途：提交事务
    pub async fn commit_transaction(&mut self) -> Result<(), String> {
        let transaction = self.current_transaction.as_mut().ok_or("事务未开始")?;

        let transaction_id = transaction.transaction_id.clone();

        transaction.status = TransactionStatus::Completed;
        transaction.completed_at = Some(DateTime::now());

        info!("[InitTransaction] 事务提交: {}", transaction_id);

        self.save_transaction_logs(&transaction_id).await?;

        Ok(())
    }

    /// 用途：回滚事务
    pub async fn rollback_transaction(&mut self) -> Result<(), String> {
        let transaction_id = self
            .current_transaction
            .as_ref()
            .ok_or("事务未开始")?
            .transaction_id
            .clone();

        let rollback_actions: Vec<RollbackAction> =
            self.rollback_actions.iter().rev().cloned().collect();

        let mut rollback_errors: Vec<String> = Vec::new();
        let mut all_logs: Vec<(String, String)> = Vec::new();

        all_logs.push(("info".to_string(), "开始回滚事务".to_string()));

        for action in rollback_actions {
            let result =
                Self::execute_rollback_action_static(&self.rb, &transaction_id, &action).await;
            match result {
                Ok(logs) => {
                    all_logs.extend(logs);
                }
                Err(e) => {
                    all_logs.push((
                        "error".to_string(),
                        format!("回滚操作失败: {:?}, 错误: {}", action, e),
                    ));
                    rollback_errors.push(e);
                }
            }
        }

        let error_message = if !rollback_errors.is_empty() {
            Some(format!("部分回滚失败: {:?}", rollback_errors))
        } else {
            None
        };

        {
            let transaction = self.current_transaction.as_mut().ok_or("事务未开始")?;

            transaction.status = TransactionStatus::RolledBack;
            transaction.completed_at = Some(DateTime::now());
            transaction.error_message = error_message;
        }

        let mut log_entries: Vec<InitTransactionLog> = Vec::new();
        for (level, message) in all_logs {
            log_entries.push(InitTransactionLog {
                log_id: ulid::Ulid::new().to_string(),
                transaction_id: transaction_id.clone(),
                level: level.to_string(),
                message: message.to_string(),
                timestamp: Some(DateTime::now()),
            });

            match level.as_str() {
                "error" => error!("[InitTransaction] {}", message),
                "warn" => warn!("[InitTransaction] {}", message),
                _ => info!("[InitTransaction] {}", message),
            }
        }

        if !rollback_errors.is_empty() {
            log_entries.push(InitTransactionLog {
                log_id: ulid::Ulid::new().to_string(),
                transaction_id: transaction_id.clone(),
                level: "warn".to_string(),
                message: format!("事务回滚完成，但有错误: {}", transaction_id),
                timestamp: Some(DateTime::now()),
            });
            warn!(
                "[InitTransaction] 事务回滚完成，但有错误: {}",
                transaction_id
            );
        } else {
            log_entries.push(InitTransactionLog {
                log_id: ulid::Ulid::new().to_string(),
                transaction_id: transaction_id.clone(),
                level: "info".to_string(),
                message: format!("事务回滚完成: {}", transaction_id),
                timestamp: Some(DateTime::now()),
            });
            info!("[InitTransaction] 事务回滚完成: {}", transaction_id);
        }

        self.transaction_logs.extend(log_entries);

        self.save_transaction_logs(&transaction_id).await?;

        Ok(())
    }

    /// 用途：执行回滚操作（静态方法）
    async fn execute_rollback_action_static(
        rb: &Arc<RBatis>,
        _transaction_id: &str,
        action: &RollbackAction,
    ) -> Result<Vec<(String, String)>, String> {
        let mut logs = Vec::new();

        match action {
            RollbackAction::DeleteAdmin { account } => {
                let result = Self::delete_admin_static(rb, account).await;
                if result.is_ok() {
                    logs.push(("info".to_string(), format!("删除管理员: {}", account)));
                }
                result?;
            }
            RollbackAction::DeleteUserRole { user_id } => {
                let result = Self::delete_user_role_static(rb, user_id).await;
                if result.is_ok() {
                    logs.push(("info".to_string(), format!("删除用户角色关联: {}", user_id)));
                }
                result?;
            }
            RollbackAction::RestoreConfig { backup_config } => {
                let result = Self::restore_config_static(backup_config).await;
                if result.is_ok() {
                    logs.push(("info".to_string(), "恢复配置文件".to_string()));
                }
                result?;
            }
            RollbackAction::UnmarkInit => {
                let result = Self::unmark_init_static(rb).await;
                if result.is_ok() {
                    logs.push(("info".to_string(), "取消初始化标记".to_string()));
                }
                result?;
            }
        }

        Ok(logs)
    }

    /// 用途：删除管理员（静态方法）
    async fn delete_admin_static(rb: &Arc<RBatis>, account: &str) -> Result<(), String> {
        info!("[InitTransaction] 删除管理员: {}", account);

        let conn = rb
            .acquire()
            .await
            .map_err(|e| format!("获取数据库连接失败: {}", e))?;

        use crate::domain::table::sys_user::SysUser;

        match SysUser::delete_by_map(&conn, rbs::value!({"account": account})).await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("删除管理员失败: {}", e)),
        }
    }

    /// 用途：删除用户角色关联（静态方法）
    async fn delete_user_role_static(rb: &Arc<RBatis>, user_id: &str) -> Result<(), String> {
        info!("[InitTransaction] 删除用户角色关联: {}", user_id);

        let conn = rb
            .acquire()
            .await
            .map_err(|e| format!("获取数据库连接失败: {}", e))?;

        use crate::domain::table::basic::rbac::RbacUserRole;

        match RbacUserRole::delete_by_map(&conn, rbs::value!({"user_id": user_id})).await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("删除用户角色关联失败: {}", e)),
        }
    }

    /// 用途：恢复配置文件（静态方法）
    async fn restore_config_static(backup_config: &str) -> Result<(), String> {
        info!("[InitTransaction] 恢复配置文件");

        std::fs::write("config/application.json5", backup_config)
            .map_err(|e| format!("恢复配置文件失败: {}", e))?;

        Ok(())
    }

    /// 用途：取消初始化标记（静态方法）
    async fn unmark_init_static(rb: &Arc<RBatis>) -> Result<(), String> {
        info!("[InitTransaction] 取消初始化标记");

        let conn = rb
            .acquire()
            .await
            .map_err(|e| format!("获取数据库连接失败: {}", e))?;

        use crate::domain::table::key_value_config::KeyValueConfig;

        KeyValueConfig::set_value(&conn, "is_init", "false", Some("取消初始化标记"))
            .await
            .map_err(|e| format!("取消初始化标记失败: {}", e))?;

        Ok(())
    }

    /// 用途：记录事务日志
    fn log_transaction(&mut self, transaction_id: &str, level: &str, message: &str) {
        let log = InitTransactionLog {
            log_id: ulid::Ulid::new().to_string(),
            transaction_id: transaction_id.to_string(),
            level: level.to_string(),
            message: message.to_string(),
            timestamp: Some(DateTime::now()),
        };

        self.transaction_logs.push(log);

        match level {
            "error" => error!("[InitTransaction] {}", message),
            "warn" => warn!("[InitTransaction] {}", message),
            _ => info!("[InitTransaction] {}", message),
        }
    }

    /// 用途：获取事务状态
    pub fn get_transaction_status(&self) -> Option<&TransactionStatus> {
        self.current_transaction.as_ref().map(|t| &t.status)
    }

    /// 用途：获取当前事务
    pub fn get_current_transaction(&self) -> Option<&InitTransaction> {
        self.current_transaction.as_ref()
    }

    /// 用途：获取已完成步骤
    pub fn get_completed_steps(&self) -> &[InitTransactionStep] {
        &self.completed_steps
    }

    /// 用途：获取回滚操作列表
    pub fn get_rollback_actions(&self) -> &[RollbackAction] {
        &self.rollback_actions
    }

    /// 用途：获取事务日志
    pub fn get_transaction_logs(&self) -> &[InitTransactionLog] {
        &self.transaction_logs
    }

    /// 用途：保存事务日志到数据库
    async fn save_transaction_logs(&mut self, _transaction_id: &str) -> Result<(), String> {
        if self.transaction_logs.is_empty() {
            info!("[InitTransaction] 没有事务日志需要保存");
            return Ok(());
        }

        let conn = self
            .rb
            .acquire()
            .await
            .map_err(|e| format!("获取数据库连接失败: {}", e))?;

        info!(
            "[InitTransaction] 保存 {} 条事务日志到数据库",
            self.transaction_logs.len()
        );

        for log in &self.transaction_logs {
            let db_log = DBInitTransactionLog {
                log_id: log.log_id.clone(),
                transaction_id: log.transaction_id.clone(),
                level: log.level.clone(),
                message: log.message.clone(),
                timestamp: log.timestamp.clone(),
            };

            match DBInitTransactionLog::insert(&conn, &db_log).await {
                Ok(_) => {
                    info!("[InitTransaction] 日志保存成功: {}", log.message);
                }
                Err(e) => {
                    warn!(
                        "[InitTransaction] 日志保存失败: {}, 错误: {}",
                        log.message, e
                    );
                }
            }
        }

        Ok(())
    }
}
