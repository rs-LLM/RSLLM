// 用途：键值配置表结构
// 说明：用于存储系统配置，支持灵活的键值对存储方式
use rbatis::crud;
use rbatis::rbdc::DateTime;
use rbs::{Value, value};

use crate::domain::table::sys_user::SysUser;

/// 用途：键值配置表结构体
/// 说明：存储系统配置的键值对
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct KeyValueConfig {
    /// 配置键名
    pub key: String,

    /// 配置值
    /// 说明：存储配置的字符串值，可根据需要解析为不同类型
    pub value: String,

    /// 创建时间
    /// 说明：记录配置项的创建时间
    pub created_at: Option<DateTime>,

    /// 更新时间
    /// 说明：记录配置项的最后更新时间
    pub updated_at: Option<DateTime>,

    /// 配置描述
    /// 说明：可选的配置项描述信息
    pub description: Option<String>,
}

// 用途：生成CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(KeyValueConfig {});

impl KeyValueConfig {
    fn value_from_query_row(row: &Value) -> Option<String> {
        let row_json = serde_json::to_value(row).ok()?;
        let value = row_json.get("value")?;

        match value {
            serde_json::Value::String(text) => Some(text.clone()),
            serde_json::Value::Null => None,
            other => serde_json::to_string(other).ok(),
        }
    }

    /// 用途：创建新的键值配置项
    /// 说明：生成一个新的键值配置实例
    pub fn new(key: &str, value: &str, description: Option<&str>) -> Self {
        Self {
            key: key.to_string(),
            value: value.to_string(),
            created_at: Some(DateTime::now()),
            updated_at: Some(DateTime::now()),
            description: description.map(|s| s.to_string()),
        }
    }

    /// 用途：获取配置值
    /// 说明：根据键名获取配置值，如果不存在则返回默认值
    pub async fn get_value(
        conn: &(impl rbatis::executor::Executor + ?Sized),
        key: &str,
        default: &str,
    ) -> Result<String, rbatis::error::Error> {
        let result = conn
            .query(
                "SELECT value FROM key_value_config WHERE key = ? LIMIT 1",
                vec![rbs::value!(key)],
            )
            .await?;

        if let Some(row) = result.as_array().and_then(|rows| rows.first())
            && let Some(value) = Self::value_from_query_row(row)
        {
            return Ok(value);
        }

        Ok(default.to_string())
    }

    /// 用途：设置配置值
    /// 说明：设置配置项的值，如果不存在则创建
    pub async fn set_value(
        conn: &impl rbatis::executor::Executor,
        key: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<u64, rbatis::error::Error> {
        let now = DateTime::now().to_string();
        let update_result = conn
            .exec(
                "UPDATE key_value_config SET value = ?, updated_at = ?, description = COALESCE(?, description) WHERE key = ?",
                vec![
                    rbs::value!(value),
                    rbs::value!(now),
                    rbs::value!(description),
                    rbs::value!(key),
                ],
            )
            .await?;

        if update_result.rows_affected > 0 {
            Ok(update_result.rows_affected)
        } else {
            let config = KeyValueConfig::new(key, value, description);
            let result = KeyValueConfig::insert(conn, &config).await?;
            Ok(result.rows_affected)
        }
    }

    /// 用途：检查系统是否已初始化
    /// 说明：通过查询is_init键的值判断系统初始化状态
    pub async fn is_initialized(
        conn: &impl rbatis::executor::Executor,
    ) -> Result<bool, rbatis::error::Error> {
        let value = KeyValueConfig::get_value(conn, "is_init", "false").await?;
        if value != "true" {
            return Ok(false);
        }

        let admin_users = SysUser::select_by_map(conn, value! { "account": "admin" }).await?;
        Ok(!admin_users.is_empty())
    }

    /// 用途：标记系统为已初始化
    /// 说明：设置is_init键为true，并记录初始化时间
    pub async fn mark_initialized(
        conn: &impl rbatis::executor::Executor,
    ) -> Result<(), rbatis::error::Error> {
        // 设置is_init为true
        KeyValueConfig::set_value(conn, "is_init", "true", Some("系统初始化状态标记")).await?;

        // 记录初始化时间
        let now = DateTime::now();
        KeyValueConfig::set_value(conn, "init_time", &now.to_string(), Some("系统初始化时间"))
            .await?;

        Ok(())
    }
}
