// 用途：导入时间相关类型
// 说明：处理缓存的过期时间，使用Duration表示生存时间
use std::time::Duration;

// 用途：导入自定义错误类型
// 说明：统一错误处理，确保所有错误都遵循相同的类型规范
use crate::error::{Error, Result};

// 用途：导入缓存服务接口
// 说明：实现缓存服务的统一接口，支持多种缓存实现的切换
use crate::service::ICacheService;

// 用途：导入异步trait宏
// 说明：实现异步trait方法，支持异步缓存操作
use async_trait::async_trait;


// 用途：导入Redis相关类型
// 说明：使用Redis作为缓存存储，需要Redis客户端和连接类型
use redis::{RedisResult, aio::MultiplexedConnection};
/// 用途：Redis缓存服务
/// 说明：使用Redis作为缓存存储，提供分布式缓存支持，适合多实例部署场景
#[derive(Debug)]
pub struct RedisCacheService {
    // 用途：Redis客户端
    // 说明：用于与Redis服务器建立连接和执行命令
    pub client: redis::Client,
}

impl RedisCacheService {
    // 用途：创建Redis缓存服务实例
    // 说明：根据Redis URL创建并初始化Redis客户端
    pub fn new(url: &str) -> Result<Self> {
        // 用途：打印Redis连接信息
        // 说明：调试时显示当前连接的Redis地址
        println!("[rsllm] connect redis ({})...", url);
        
        // 用途：打开Redis客户端连接
        // 说明：根据提供的URL创建Redis客户端，用于后续的Redis操作
        let client = redis::Client::open(url)
            .map_err(|e| Error::from(format!("open redis client failed={}", e)))?;
        
        // 用途：打印Redis连接成功信息
        // 说明：调试时确认Redis连接已成功建立
        println!("[rsllm] connect redis success!");
        
        // 用途：返回RedisCacheService实例
        // 说明：创建并返回初始化后的Redis缓存服务实例
        Ok(Self { client })
    }

    // 用途：获取Redis异步连接
    // 说明：获取多路复用的异步连接，用于执行Redis命令
    pub async fn get_conn(&self) -> Result<MultiplexedConnection> {
        // 用途：获取多路复用异步连接
        // 说明：使用多路复用连接可以在单个连接上执行多个并发命令，提高性能
        let conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| format!("RedisService connect fail:{}", e))?;
        
        // 用途：返回Redis连接
        // 说明：返回成功获取的Redis连接，用于后续的命令执行
        Ok(conn)
    }
}

// 用途：实现ICacheService异步trait
// 说明：提供Redis缓存的具体实现，支持统一的缓存服务接口
#[async_trait]
impl ICacheService for RedisCacheService {
    // 用途：设置字符串缓存（无过期时间）
    // 说明：将键值对存储到Redis缓存中，永久有效直到被覆盖或手动删除
    async fn set_string(&self, k: &str, v: &str) -> Result<String> {
        // 用途：转换键为字符串
        // 说明：Redis命令需要String类型的键
        let k = k.to_string();
        
        // 用途：转换值为字符串
        // 说明：Redis命令需要String类型的值
        let v = v.to_string();
        
        // 用途：调用set_string_ex方法设置缓存
        // 说明：复用带过期时间的设置方法，传入None表示永久有效
        return self.set_string_ex(&k, &v, None).await;
    }

    // 用途：获取字符串缓存
    // 说明：根据键从Redis缓存中获取对应的值
    async fn get_string(&self, k: &str) -> Result<String> {
        // 用途：转换键为字符串
        // 说明：Redis命令需要String类型的键
        let k = k.to_string();
        
        // 用途：获取Redis连接
        // 说明：获取连接用于执行Redis GET命令
        let mut conn = self.get_conn().await?;
        
        // 用途：执行Redis GET命令
        // 说明：从Redis中获取指定键的值
        let result: RedisResult<Option<String>> =
            redis::cmd("GET").arg(&[&k]).query_async(&mut conn).await;
        
        // 用途：处理GET命令结果
        // 说明：根据命令执行结果返回对应的值或错误
        return match result {
            // 用途：GET命令执行成功
            // 说明：返回获取到的值，如果值不存在则返回空字符串
            Ok(v) => Ok(v.unwrap_or_default()),
            // 用途：GET命令执行失败
            // 说明：返回错误信息，包含失败说明
            Err(e) => Err(Error::from(format!(
                "RedisService get_string({}) fail:{}",
                k,
                e.to_string()
            ))),
        };
    }

    // 用途：设置带过期时间的字符串缓存
    // 说明：将键值对存储到Redis缓存中，并设置过期时间
    async fn set_string_ex(&self, k: &str, v: &str, ex: Option<Duration>) -> Result<String> {
        // 用途：转换键为字符串
        // 说明：Redis命令需要String类型的键
        let k = k.to_string();
        
        // 用途：转换值为字符串
        // 说明：Redis命令需要String类型的值
        let v = v.to_string();
        
        // 用途：获取Redis连接
        // 说明：获取连接用于执行Redis SET命令
        let mut conn = self.get_conn().await?;
        
        // 用途：根据是否设置过期时间执行不同的SET命令
        // 说明：支持带过期时间和不带过期时间的SET操作
        return if ex.is_none() {
            // 用途：执行不带过期时间的SET命令
            // 说明：当没有提供过期时间时，使用普通的SET命令
            match redis::cmd("SET").arg(&[k, v]).query_async(&mut conn).await {
                // 用途：SET命令执行成功
                // 说明：返回操作结果
                Ok(v) => Ok(v),
                // 用途：SET命令执行失败
                // 说明：返回错误信息，包含失败说明
                Err(e) => Err(Error::from(format!(
                    "RedisService set_string_ex fail:{}",
                    e.to_string()
                ))),
            }
        } else {
            // 用途：执行带过期时间的SET命令
            // 说明：当提供了过期时间时，使用SET EX命令设置键值对和过期时间
            match redis::cmd("SET")
                .arg(&[&k, &v, "EX", &ex.unwrap_or_default().as_secs().to_string()])
                .query_async(&mut conn)
                .await
            {
                // 用途：SET EX命令执行成功
                // 说明：返回操作结果
                Ok(v) => Ok(v),
                // 用途：SET EX命令执行失败
                // 说明：返回错误信息，包含失败说明
                Err(e) => Err(Error::from(format!(
                    "RedisService set_string_ex fail:{}",
                    e.to_string()
                ))),
            }
        };
    }

    // 用途：获取缓存剩余生存时间
    // 说明：返回Redis缓存项的剩余生存时间，用于判断缓存的有效性
    async fn ttl(&self, k: &str) -> Result<i64> {
        // 用途：转换键为字符串
        // 说明：Redis命令需要String类型的键
        let k = k.to_string();
        
        // 用途：获取Redis连接
        // 说明：获取连接用于执行Redis TTL命令
        let mut conn = self.get_conn().await?;
        
        // 用途：执行Redis TTL命令
        // 说明：获取指定键的剩余生存时间
        return match redis::cmd("TTL").arg(&[k]).query_async(&mut conn).await {
            // 用途：TTL命令执行成功
            // 说明：返回缓存项的剩余生存时间
            Ok(v) => Ok(v),
            // 用途：TTL命令执行失败
            // 说明：返回错误信息，包含失败说明
            Err(e) => Err(Error::from(format!(
                "RedisService ttl fail:{}",
                e.to_string()
            ))),
        };
    }
}
