// 用途：导入应用程序配置结构体
// 说明：用于获取缓存配置
use crate::config::application::ApplicationConfig;

// 用途：导入自定义错误类型和结果类型
// 说明：用于统一错误处理
use crate::error::{Error, Result};

// 用途：导入内存缓存服务
// 说明：用于实现基于内存的缓存
use crate::service::cache::MemCacheService;

// 用途：导入异步trait宏
// 说明：用于定义异步trait
use async_trait::async_trait;

// 用途：导入序列化特性
// 说明：用于JSON序列化
use serde::Serialize;

// 用途：导入反序列化特性
// 说明：用于JSON反序列化
use serde::de::DeserializeOwned;

//  用途：导入Debug特性
// 说明：用于调试
use std::fmt::Debug;

// 用途：导入Duration类型
// 说明：用于设置缓存过期时间
use std::time::Duration;
// 用途：导入Arc类型
// 说明：用于包装Box<dyn Trait>以支持Clone
use std::sync::Arc;

/// 用途：缓存服务接口
/// 说明：定义缓存服务的通用接口，支持不同的缓存实现
#[async_trait]
pub trait ICacheService: Sync + Send + Debug {
    /// 用途：设置字符串键值对
    /// 说明：将字符串值存储到缓存中
    async fn set_string(&self, k: &str, v: &str) -> Result<String>;

    /// 用途：根据键获取字符串值
    /// 说明：从缓存中获取字符串值
    async fn get_string(&self, k: &str) -> Result<String>;

    /// 用途：删除缓存键
    /// 说明：从缓存中移除指定键
    async fn del(&self, k: &str) -> Result<()>;

    /// 用途：设置带过期时间的字符串键值对
    /// 说明：存储具有生命周期的缓存数据
    async fn set_string_ex(&self, k: &str, v: &str, ex: Option<Duration>) -> Result<String>;

    /// 用途：获取键的剩余过期时间
    /// 说明：检查缓存数据的剩余有效期
    async fn ttl(&self, k: &str) -> Result<i64>;
}

/// 用途：缓存服务
/// 说明：封装不同的缓存实现，提供统一的缓存操作接口
#[derive(Clone)]
pub struct CacheService {
    /// 用途：内部缓存服务实现
    /// 说明：根据配置选择不同的缓存实现
    pub inner: Arc<Box<dyn ICacheService>>,
}

impl CacheService {
    /// 用途：创建缓存服务实例
    /// 说明：根据配置创建相应的缓存服务
    pub fn new(cfg: &ApplicationConfig) -> Result<Self> {
        // 用途：获取缓存配置
        // 说明：根据配置选择缓存类型
        let cache = cfg.cache.as_str();
        // 用途：检查是否为内存缓存
        // 说明：内存缓存配置简单，无需额外依赖
        if cache == "mem" {
            // 用途：输出缓存类型日志
            // 说明：告知用户当前使用的缓存类型
            println!("[rsllm] cache_type: mem");
            // 用途：创建内存缓存服务实例
            // 说明：返回内存缓存服务
            return Ok(Self {
                inner: Arc::new(Box::new(MemCacheService::default())),
            });
        // 用途：检查是否为Redis缓存
        // 说明：Redis缓存支持分布式和持久化
        } else if cache.starts_with("redis") {
            // 用途：Redis特性条件编译
            // 说明：只有启用redis特性时才支持Redis缓存
            #[cfg(feature = "cache_redis")]
            {
                // 用途：输出缓存类型日志
                // 说明：告知用户当前使用的缓存类型
                println!("[rsllm] cache_type: redis");
                // 用途：创建Redis缓存服务实例
                // 说明：返回Redis缓存服务
                return Ok(Self {
                    inner: Arc::new(Box::new(crate::service::RedisCacheService::new(&cache)?)),
                });
            }
        }
        // 用途：返回错误
        // 说明：不支持的缓存类型
        Err(Error::from(format!(
            "[rsllm] unknown of cache: \"{}\",current support 'mem' or 'redis'",
            cache
        )))
    }

    /// 用途：设置字符串键值对
    /// 说明：将字符串值存储到缓存中
    pub async fn set_string(&self, k: &str, v: &str) -> Result<String> {
        // 用途：调用内部缓存服务设置字符串
        // 说明：委托给具体的缓存实现
        self.inner.set_string(k, v).await
    }

    /// 用途：根据键获取字符串值
    /// 说明：从缓存中获取字符串值
    pub async fn get_string(&self, k: &str) -> Result<String> {
        // 用途：调用内部缓存服务获取字符串
        // 说明：委托给具体的缓存实现
        self.inner.get_string(k).await
    }

    /// 用途：删除缓存键
    /// 说明：从缓存中移除指定键
    pub async fn del(&self, k: &str) -> Result<()> {
        self.inner.del(k).await
    }

    /// 用途：设置JSON键值对
    /// 说明：将JSON对象存储到缓存中
    pub async fn set_json<T>(&self, k: &str, v: &T) -> Result<String>
    where
        T: Serialize + Sync,
    {
        // 用途：将对象序列化为JSON字符串
        // 说明：缓存服务只支持字符串存储
        let data = serde_json::to_string(v);
        // 用途：检查序列化结果
        // 说明：处理序列化失败的情况
        if let Err(e) = &data {
            return Err(crate::error::Error::from(format!(
                "MemCacheService set_json fail:{}",
                e
            )));
        }
        // 用途：转换为结果类型
        // 说明：统一错误处理
        let value = data.map_err(|e| Error::from(e.to_string()))?;
        // 用途：调用set_string方法存储JSON
        // 说明：利用已有的字符串存储功能
        let result = self.set_string(k, &value).await?;
        // 用途：返回成功结果
        // 说明：告知调用者存储成功
        Ok(result)
    }

    /// 用途：根据键获取JSON对象
    /// 说明：从缓存中获取JSON数据并反序列化为对象
    pub async fn get_json<T>(&self, k: &str) -> Result<T>
    where
        T: DeserializeOwned + Sync,
    {
        // 用途：获取字符串值
        // 说明：从缓存中获取存储的JSON字符串
        let mut r = self.get_string(k).await?;
        // 用途：检查结果是否为空
        // 说明：空字符串无法反序列化，需要转换为null
        if r.is_empty() {
            r = "null".to_string();
        }
        // 用途：将JSON字符串反序列化为对象
        // 说明：返回调用者需要的类型
        let data: T = serde_json::from_str(r.as_str())
            .map_err(|e| Error::from(format!("MemCacheService GET fail:{}", e)))?;
        // 用途：返回反序列化结果
        // 说明：告知调用者获取成功并返回数据
        Ok(data)
    }

    /// 用途：设置带过期时间的字符串键值对
    /// 说明：存储具有生命周期的缓存数据
    pub async fn set_string_ex(&self, k: &str, v: &str, ex: Option<Duration>) -> Result<String> {
        // 用途：调用内部缓存服务设置带过期时间的字符串
        // 说明：委托给具体的缓存实现
        self.inner.set_string_ex(k, v, ex).await
    }

    /// 用途：获取键的剩余过期时间
    /// 说明：检查缓存数据的剩余有效期
    pub async fn ttl(&self, k: &str) -> Result<i64> {
        // 用途：调用内部缓存服务获取过期时间
        // 说明：委托给具体的缓存实现
        self.inner.ttl(k).await
    }
}
