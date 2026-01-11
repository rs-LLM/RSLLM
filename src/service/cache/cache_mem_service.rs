// 用途：导入自定义错误类型Result
// 说明：统一错误处理，确保所有错误都遵循相同的类型规范
use crate::error::Result;

// 用途：导入缓存服务接口
// 说明：实现缓存服务的统一接口，支持多种缓存实现的切换
use crate::service::cache::ICacheService;

// 用途：导入异步trait宏
// 说明：实现异步trait方法，支持异步缓存操作
use async_trait::async_trait;

// 用途：导入同步哈希表
// 说明：使用线程安全的哈希表存储缓存数据，支持多线程并发访问
use rbatis::dark_std::sync::SyncHashMap;

// 用途：导入Sub trait
// 说明：用于计算时间差，判断缓存是否过期
use std::ops::Sub;

// 用途：导入时间相关类型
// 说明：处理缓存的过期时间，使用Instant记录开始时间，Duration记录生存时间
use std::time::{Duration, Instant};

/// 用途：内存缓存服务
/// 说明：使用内存作为缓存存储，提供快速的缓存访问和管理
#[derive(Debug, Default)]
pub struct MemCacheService {
    // 用途：缓存存储容器
    // 说明：使用SyncHashMap存储键值对，每个值包含字符串内容和可选的过期时间信息
    // 格式：Map<Key,(Value,Option<(Instant(开始时间), Duration(生存时间))>)>
    pub cache: SyncHashMap<String, (String, Option<(Instant, Duration)>)>,
}

impl MemCacheService {
    // 用途：清理过期缓存
    // 说明：定期清理过期的缓存数据，释放内存空间，提高缓存效率
    pub fn recycling(&self) {
        // 用途：创建需要删除的键列表
        // 说明：预分配足够的容量，减少内存分配次数
        let mut need_removed = Vec::with_capacity(self.cache.len());

        // 用途：遍历所有缓存项
        // 说明：检查每个缓存项是否过期
        for (k, v) in self.cache.iter() {
            // 用途：检查缓存项是否有过期时间
            // 说明：只有设置了过期时间的缓存项才需要检查是否过期
            if let Some((i, d)) = v.1 {
                // 用途：判断缓存是否过期
                // 说明：如果当前时间减去开始时间大于等于生存时间，则缓存过期
                if i.elapsed() >= d {
                    // 用途：将过期的键添加到删除列表
                    // 说明：记录需要删除的键，避免在遍历过程中修改哈希表
                    need_removed.push(k.to_string());
                }
            }
        }

        // 用途：检查是否有需要删除的缓存项
        // 说明：如果没有过期项，直接返回，避免不必要的操作
        if !need_removed.is_empty() {
            // 用途：删除所有过期的缓存项
            // 说明：清理过期数据，释放内存
            for x in need_removed {
                self.cache.remove(&x);
            }
            // 用途：收缩哈希表容量
            // 说明：释放未使用的内存空间，优化内存使用
            self.cache.shrink_to_fit();
        }
    }
}

// 用途：实现Default trait
// 说明：提供默认的构造方法，方便创建MemCacheService实例

// 用途：实现ICacheService异步trait
// 说明：提供内存缓存的具体实现，支持统一的缓存服务接口
#[async_trait]
impl ICacheService for MemCacheService {
    // 用途：设置字符串缓存（无过期时间）
    // 说明：将键值对存储到内存缓存中，永久有效直到被覆盖或手动删除
    async fn set_string(&self, k: &str, v: &str) -> Result<String> {
        // 用途：清理过期缓存
        // 说明：在设置新缓存前清理过期项，保持缓存的有效性
        self.recycling();

        // 用途：转换键为字符串
        // 说明：缓存键使用String类型存储
        let k = k.to_string();

        // 用途：转换值为字符串
        // 说明：缓存值使用String类型存储
        let v = v.to_string();

        // 用途：插入缓存项
        // 说明：将键值对存储到哈希表中，过期时间设为None（永久有效）
        self.cache.insert(k.to_string(), (v.clone(), None));

        // 用途：返回设置的值
        // 说明：返回成功设置的值，方便调用者验证
        Ok(v.to_string())
    }

    // 用途：获取字符串缓存
    // 说明：根据键从内存缓存中获取对应的值
    async fn get_string(&self, k: &str) -> Result<String> {
        // 用途：清理过期缓存
        // 说明：在获取缓存前清理过期项，确保返回的是有效缓存
        self.recycling();

        // 用途：转换键为字符串
        // 说明：缓存键使用String类型存储
        let k = k.to_string();

        // 用途：初始化返回值
        // 说明：默认返回空字符串，避免Option类型的复杂性
        let mut v = String::new();

        // 用途：查找缓存项
        // 说明：根据键查找对应的缓存值
        if let Some(r) = self.cache.get(&k) {
            // 用途：获取缓存值
            // 说明：如果找到缓存项，复制其值
            v = r.0.to_string();
        }

        // 用途：返回缓存值
        // 说明：返回找到的值，未找到则返回空字符串
        Ok(v)
    }

    // 用途：设置带过期时间的字符串缓存
    // 说明：将键值对存储到内存缓存中，并设置过期时间
    async fn set_string_ex(&self, k: &str, v: &str, t: Option<Duration>) -> Result<String> {
        // 用途：清理过期缓存
        // 说明：在设置新缓存前清理过期项，保持缓存的有效性
        self.recycling();

        // 用途：转换键为字符串
        // 说明：缓存键使用String类型存储
        let k = k.to_string();

        // 用途：转换值为字符串
        // 说明：缓存值使用String类型存储
        let v = v.to_string();

        // 用途：初始化过期时间
        // 说明：默认无过期时间
        let mut e = None;

        // 用途：检查是否设置了过期时间
        // 说明：如果提供了过期时间，计算开始时间并记录
        if let Some(ex) = t {
            // 用途：设置过期时间信息
            // 说明：记录当前时间作为开始时间，结合生存时间计算过期时间
            e = Some((Instant::now(), ex));
        }

        // 用途：插入缓存项
        // 说明：将键值对和过期时间存储到哈希表中
        _ = self.cache.insert(k.to_string(), (v.clone(), e));

        // 用途：返回设置的值
        // 说明：返回成功设置的值，方便调用者验证
        Ok(v.to_string())
    }

    // 用途：获取缓存剩余生存时间
    // 说明：返回缓存项的剩余生存时间，用于判断缓存的有效性
    async fn ttl(&self, k: &str) -> Result<i64> {
        // 用途：清理过期缓存
        // 说明：在获取TTL前清理过期项，确保返回的是有效缓存的TTL
        self.recycling();

        // 用途：查找缓存项
        // 说明：根据键查找对应的缓存值和过期时间信息
        let v = self.cache.get(k).cloned();

        // 用途：计算剩余生存时间
        // 说明：根据缓存项的状态返回不同的TTL值
        let v = match v {
            // 用途：缓存项不存在
            // 说明：返回-2表示缓存项不存在
            None => -2,
            // 用途：缓存项存在
            // 说明：检查缓存项是否有过期时间
            Some((_r, o)) => match o {
                // 用途：缓存项无过期时间
                // 说明：返回-1表示永久有效
                None => -1,
                // 用途：缓存项有过期时间
                // 说明：计算并返回剩余生存时间
                Some((i, d)) => {
                    // 用途：计算已使用时间
                    // 说明：当前时间减去开始时间，得到缓存已存在的时间
                    let use_time = i.elapsed();

                    // 用途：判断缓存是否过期
                    // 说明：如果已使用时间小于生存时间，则返回剩余时间，否则返回0表示已过期
                    if d > use_time {
                        // 用途：计算剩余时间
                        // 说明：生存时间减去已使用时间，转换为秒
                        d.sub(use_time).as_secs() as i64
                    } else {
                        // 用途：缓存已过期
                        // 说明：返回0表示缓存已过期
                        0
                    }
                }
            },
        };

        // 用途：返回剩余生存时间
        // 说明：返回TTL值，单位为秒
        Ok(v)
    }
}
