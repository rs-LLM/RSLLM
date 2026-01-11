// 用途：初始化事务日志表结构
// 说明：用于记录初始化事务执行过程中的详细日志
use rbatis::crud;
use rbatis::rbdc::DateTime;

/// 用途：初始化事务日志表结构体
/// 说明：记录初始化事务的详细执行日志，用于审计和调试
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct InitTransactionLog {
    /// 日志ID
    /// 说明：日志记录的唯一标识符
    pub log_id: String,

    /// 事务ID
    /// 说明：关联的事务ID
    #[serde(rename = "transactionId")]
    pub transaction_id: String,

    /// 日志级别
    /// 说明：日志级别，包括info、warn、error
    pub level: String,

    /// 日志消息
    /// 说明：日志的详细内容
    pub message: String,

    /// 时间戳
    /// 说明：日志记录的时间
    pub timestamp: Option<DateTime>,
}

// 用途：生成CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(InitTransactionLog {}, "init_transaction_log");

impl InitTransactionLog {
    /// 用途：创建新的事务日志
    /// 说明：生成一个新的事务日志实例
    pub fn new(transaction_id: &str, level: &str, message: &str) -> Self {
        Self {
            log_id: ulid::Ulid::new().to_string(),
            transaction_id: transaction_id.to_string(),
            level: level.to_string(),
            message: message.to_string(),
            timestamp: Some(DateTime::now()),
        }
    }

    /// 用途：查询事务的所有日志
    /// 说明：根据事务ID查询该事务的所有日志记录
    pub async fn get_logs_by_transaction(
        conn: &impl rbatis::executor::Executor,
        transaction_id: &str,
    ) -> Result<Vec<Self>, rbatis::error::Error> {
        InitTransactionLog::select_by_map(conn, rbs::value!({"transactionId": transaction_id}))
            .await
    }

    /// 用途：按级别查询日志
    /// 说明：根据事务ID和日志级别查询日志记录
    pub async fn get_logs_by_level(
        conn: &impl rbatis::executor::Executor,
        transaction_id: &str,
        level: &str,
    ) -> Result<Vec<Self>, rbatis::error::Error> {
        InitTransactionLog::select_by_map(
            conn,
            rbs::value!({"transactionId": transaction_id, "level": level}),
        )
        .await
    }
}
