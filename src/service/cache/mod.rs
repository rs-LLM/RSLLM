// 用途：导入内存缓存服务模块
// 说明：使用内存作为缓存存储，提供快速的缓存访问
mod cache_mem_service;

// 用途：声明缓存服务模块
// 原因：提供缓存操作的接口，封装缓存的具体实现
mod cache_service;

// 用途：导出缓存服务
// 原因：允许其他模块访问缓存功能
pub use cache_service::*;

// 用途：条件编译导入Redis缓存服务模块
// 说明：当启用cache_redis特性时，导入Redis缓存实现，支持Redis作为缓存存储
#[cfg(feature = "cache_redis")]
mod cache_redis_service;

// 用途：导出内存缓存服务的所有公共类型和函数
// 说明：方便外部模块直接使用内存缓存服务，无需单独导入子模块
pub use cache_mem_service::*;

// 用途：条件编译导出Redis缓存服务的所有公共类型和函数
// 说明：当启用cache_redis特性时，外部模块可以直接使用Redis缓存服务
#[cfg(feature = "cache_redis")]
pub use cache_redis_service::*;
