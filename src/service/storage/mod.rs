// 用途：导入本地存储服务模块
// 说明：使用本地文件系统作为存储，提供简单的文件存储功能
mod storage_local_service;
// 用途：声明存储服务模块
// 原因：提供存储操作的接口，封装存储的具体实现
mod storage_service;

// 用途：导出存储模块
// 原因：允许其他模块访问存储的具体实现
pub use storage_service::*;

// 用途：条件编译导入OSS存储服务模块
// 说明：当启用storage_s3特性时，导入OSS存储实现，支持阿里云OSS作为存储
#[cfg(feature = "storage_s3")]
mod storage_oss_service;

// 用途：导出本地存储服务的所有公共类型和函数
// 说明：方便外部模块直接使用本地存储服务，无需单独导入子模块
pub use storage_local_service::*;

// 用途：条件编译导出OSS存储服务的所有公共类型和函数
// 说明：当启用storage_s3特性时，外部模块可以直接使用OSS存储服务
#[cfg(feature = "storage_s3")]
pub use storage_oss_service::*;
