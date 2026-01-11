// 用途：键值配置表结构
// 说明：用于存储系统配置，支持灵活的键值对存储方式
use rbatis::crud;
use rbatis::rbdc::DateTime;

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
        conn: &impl rbatis::executor::Executor,
        key: &str,
        default: &str,
    ) -> Result<String, rbatis::error::Error> {
        // 根据key查询配置
        let result = KeyValueConfig::select_by_map(conn, rbs::value!({"key": key})).await?;

        if let Some(config) = result.into_iter().next() {
            Ok(config.value)
        } else {
            Ok(default.to_string())
        }
    }

    /// 用途：设置配置值
    /// 说明：设置配置项的值，如果不存在则创建
    pub async fn set_value(
        conn: &impl rbatis::executor::Executor,
        key: &str,
        value: &str,
        description: Option<&str>,
    ) -> Result<u64, rbatis::error::Error> {
        // 查询是否已存在
        let existing = KeyValueConfig::select_by_map(conn, rbs::value!({"key": key})).await?;

        if existing.is_empty() {
            // 不存在则插入
            let config = KeyValueConfig::new(key, value, description);
            let result = KeyValueConfig::insert(conn, &config).await?;
            Ok(result.rows_affected)
        } else {
            // 存在则更新
            let mut config = existing.into_iter().next().unwrap();
            config.value = value.to_string();
            config.updated_at = Some(DateTime::now());
            if let Some(desc) = description {
                config.description = Some(desc.to_string());
            }
            let result =
                KeyValueConfig::update_by_map(conn, &config, rbs::value!({"key": key})).await?;
            Ok(result.rows_affected)
        }
    }

    /// 用途：检查系统是否已初始化
    /// 说明：通过查询is_init键的值判断系统初始化状态
    pub async fn is_initialized(
        conn: &impl rbatis::executor::Executor,
    ) -> Result<bool, rbatis::error::Error> {
        let value = KeyValueConfig::get_value(conn, "is_init", "false").await?;
        Ok(value == "true")
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
