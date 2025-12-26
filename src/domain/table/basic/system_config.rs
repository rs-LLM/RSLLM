// 用途：系统配置表结构
// 说明：用于存储系统初始化状态等核心配置信息
use rbatis::rbdc::DateTime;
use rbatis::crud;
use rbs::value;

/// 用途：系统配置表结构体
/// 说明：存储系统初始化状态和相关配置
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SystemConfig {
    /// 配置ID
    pub id: Option<String>,
    
    /// 系统是否已初始化
    /// 说明：true表示已完成初始化，false表示未初始化
    pub initialized: bool,
    
    /// 初始化时间
    /// 说明：记录系统完成初始化的时间
    pub init_time: Option<DateTime>,
    
    /// 更新时间
    /// 说明：记录配置的最后更新时间
    pub updated_at: Option<DateTime>,
}

// 用途：生成CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(SystemConfig {});

impl SystemConfig {
    /// 用途：获取系统配置实例
    /// 说明：系统配置采用单例模式，id固定为"1"
    pub fn new() -> Self {
        Self {
            id: Some("1".to_string()),
            initialized: false,
            init_time: None,
            updated_at: None,
        }
    }
    
    /// 用途：将系统标记为已初始化
    /// 说明：设置initialized为true，并记录初始化时间
    pub fn mark_initialized(mut self) -> Self {
        self.initialized = true;
        self.init_time = Some(DateTime::now());
        self.updated_at = Some(DateTime::now());
        self
    }
    
    /// 用途：从数据库获取系统配置
    /// 说明：如果不存在则返回默认配置
    pub async fn get_system_config(
        conn: &mut impl rbatis::executor::Executor
    ) -> Result<SystemConfig, rbatis::error::Error> {
        // 尝试获取所有配置
        let configs = SystemConfig::select_all(conn).await?;
        
        // 如果配置不存在，返回默认配置
        Ok(configs.into_iter().next().unwrap_or(SystemConfig::new()))
    }
    
    /// 用途：保存系统配置
    /// 说明：如果配置不存在则插入，否则更新
    pub async fn save_system_config(
        &self,
        conn: &mut impl rbatis::executor::Executor
    ) -> Result<u64, rbatis::error::Error> {
        // 尝试获取所有配置
        let configs = SystemConfig::select_all(conn).await?;
        
        if configs.is_empty() {
            // 插入新配置
            let result = SystemConfig::insert(conn, self).await?;
            Ok(result.rows_affected)
        } else {
            // 更新现有配置
            // 使用update_by_map方法，条件是id为1
            let result = SystemConfig::update_by_map(conn, self, value!({"id": "1"})).await?;
            Ok(result.rows_affected)
        }
    }
}