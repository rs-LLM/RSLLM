// 用途：导入自定义错误和结果类型
// 说明：用于处理存储服务的错误情况
use crate::error::{Error, Result};
// 用途：导入本地文件存储服务
// 说明：用于实现本地存储功能
use crate::service::FileLocalService;
// 用途：导入异步trait宏
// 说明：用于定义异步trait接口
use async_trait::async_trait;
// 用途：导入Debug trait
// 说明：用于调试打印存储服务实例
use std::fmt::Debug;
// 用途：导入Deref trait
// 说明：用于实现StorageService的解引用，方便直接调用内部服务方法
use std::ops::Deref;
// 用途：导入Arc类型
// 说明：用于包装Box<dyn Trait>以支持Clone
use std::sync::Arc;

// 用途：存储服务异步trait标记
// 说明：表示该trait包含异步方法
#[async_trait]
// 用途：存储服务接口定义
// 说明：统一不同存储实现的接口，支持多态调用
pub trait IStorageService: Sync + Send + Debug {
    // 用途：上传文件
    // 说明：将文件数据上传到存储服务
    async fn upload(&self, name: String, data: Vec<u8>) -> Result<String>;
    // 用途：下载文件
    // 说明：从存储服务下载文件数据
    async fn download(&self, name: String) -> Result<Vec<u8>>;
    // 用途：列出文件
    // 说明：获取存储服务中的文件列表
    async fn list(&self, name: String) -> Result<Vec<String>>;
    // 用途：删除文件
    // 说明：从存储服务中删除文件
    async fn remove(&self, name: String) -> Result<()>;
}

// 用途：存储服务包装器
// 说明：用于封装不同的存储服务实现，提供统一的访问方式
#[derive(Clone)]
pub struct StorageService {
    // 用途：内部存储服务实例
    // 说明：使用Box<dyn IStorageService>实现多态，支持不同的存储后端
    pub inner: Arc<Box<dyn IStorageService>>,
}

// 用途：为StorageService实现Deref trait
// 说明：允许直接调用内部存储服务的方法，无需显式访问inner字段
impl Deref for StorageService {
    // 用途：定义解引用目标类型
    // 说明：指定解引用后得到的类型是IStorageService trait对象
    type Target = dyn IStorageService;

    // 用途：实现解引用方法
    // 说明：返回内部存储服务的引用，用于直接调用其方法
    fn deref(&self) -> &Self::Target {
        &**self.inner
    }
}

// 用途：StorageService实现
// 说明：提供存储服务的工厂方法，根据配置创建不同的存储服务实例
impl StorageService {
    // 用途：创建存储服务实例
    // 说明：根据配置字符串选择并初始化对应的存储服务
    pub fn new(storage: &str) -> Result<StorageService> {
        // 用途：判断是否使用本地存储
        // 说明：根据配置选择本地存储实现
        if storage == "local" {
            return Ok(Self {
                inner: Arc::new(Box::new(FileLocalService::new())),
            });
        }
        // 用途：判断是否使用S3存储
        // 说明：根据配置选择S3存储实现
        else if storage.starts_with("s3://") {
            // 用途：条件编译，仅在启用storage_s3特性时编译S3相关代码
            // 说明：支持按需编译，减少最终二进制文件大小
            #[cfg(feature = "storage_s3")]
            {
                return Ok(Self {
                    inner: Arc::new(Box::new(crate::service::FileS3Service::new(
                        crate::service::S3Config::load(storage)?,
                    ))),
                });
            }
        }
        // 用途：返回不支持的存储服务错误
        // 说明：当配置的存储服务类型不被支持时，返回明确的错误信息
        Err(Error::from(format!(
            "Unsupported storage service: {}",
            storage
        )))
    }
}
