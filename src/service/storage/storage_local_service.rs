// 用途：导入自定义错误类型Result
// 说明：统一错误处理，确保所有错误都遵循相同的类型规范
use crate::error::Result;

// 用途：导入存储服务接口
// 说明：实现存储服务的统一接口，支持多种存储实现的切换
use crate::service::storage::storage_service::IStorageService;

// 用途：导入异步trait宏
// 说明：实现异步trait方法，支持异步存储操作
use async_trait::async_trait;

// 用途：导入Debug trait
// 说明：为FileLocalService实现Debug trait，支持调试输出
use std::fmt::Debug;

// 用途：导入PathBuf类型
// 说明：处理文件路径，支持跨平台的路径操作
use std::path::PathBuf;

// 用途：导入Tokio异步IO扩展trait
// 说明：支持异步文件读写操作
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// 用途：派生Debug trait
// 说明：自动生成Debug实现，方便调试输出
#[derive(Debug)]

// 用途：本地文件存储服务结构体
// 说明：使用本地文件系统进行文件存储，提供简单的文件操作功能
pub struct FileLocalService {
}

impl FileLocalService {
    // 用途：创建FileLocalService实例
    // 说明：提供默认的构造方法，方便创建本地存储服务实例
    pub fn new() -> Self {
        // 用途：返回FileLocalService实例
        // 说明：创建并返回初始化后的本地存储服务实例
        Self {}
    }
}

// 用途：实现IStorageService异步trait
// 说明：提供本地文件存储的具体实现，支持统一的存储服务接口
#[async_trait]
impl IStorageService for FileLocalService {
    // 用途：上传文件到本地存储
    // 说明：将文件数据写入本地文件系统，支持文件上传功能
    async fn upload(&self, name: String, data: Vec<u8>) -> Result<String> {
        // 用途：将字符串路径转换为PathBuf
        // 说明：方便进行路径操作和文件IO
        let name = PathBuf::from(name);
        
        // 用途：获取文件的父目录
        // 说明：确保父目录存在，避免文件创建失败
        if let Some(parent) = name.parent() {
            // 用途：递归创建父目录
            // 说明：如果父目录不存在，创建所有必要的父目录
            tokio::fs::create_dir_all(&parent).await?;
        }
        
        // 用途：创建或覆盖文件
        // 说明：准备写入文件数据
        let mut f = tokio::fs::File::create(&name).await?;
        
        // 用途：将数据写入文件
        // 说明：保存文件内容到本地文件系统
        f.write(&data).await?;
        
        // 用途：刷新文件缓冲区
        // 说明：确保所有数据都写入磁盘，避免数据丢失
        f.flush().await?;
        
        // 用途：返回文件路径
        // 说明：返回成功上传的文件路径，方便后续访问
        Ok(name.to_str().unwrap_or_default().to_string())
    }

    // 用途：从本地存储下载文件
    // 说明：读取本地文件并返回文件数据，支持文件下载功能
    async fn download(&self, name: String) -> Result<Vec<u8>> {
        // 用途：将字符串路径转换为PathBuf
        // 说明：方便进行路径操作和文件IO
        let name = PathBuf::from(name);
        
        // 用途：获取文件的父目录
        // 说明：虽然下载时父目录应该存在，但仍进行检查以确保鲁棒性
        if let Some(parent) = name.parent() {
            // 用途：递归创建父目录
            // 说明：确保父目录存在，避免文件读取失败
            tokio::fs::create_dir_all(&parent).await?;
        }
        
        // 用途：打开文件
        // 说明：准备读取文件数据
        let mut f = tokio::fs::File::open(&name).await?;
        
        // 用途：创建数据缓冲区
        // 说明：用于存储读取的文件数据
        let mut data = Vec::new();
        
        // 用途：读取文件所有内容
        // 说明：将文件数据完整读取到缓冲区
        f.read_to_end(&mut data).await?;
        
        // 用途：返回文件数据
        // 说明：返回成功读取的文件数据
        Ok(data)
    }

    // 用途：列出本地目录中的文件
    // 说明：获取指定目录下的所有文件和子目录，支持目录浏览功能
    async fn list(&self, name: String) -> Result<Vec<String>> {
        // 用途：将字符串路径转换为PathBuf
        // 说明：方便进行目录操作
        let name = PathBuf::from(name);
        
        // 用途：打开目录读取器
        // 说明：用于遍历目录中的文件和子目录
        let mut rd = tokio::fs::read_dir(&name).await?;
        
        // 用途：创建结果列表
        // 说明：用于存储目录中的文件路径
        let mut result = Vec::new();
        
        // 用途：遍历目录条目
        // 说明：逐个获取目录中的文件和子目录
        while let Ok(v) = rd.next_entry().await {
            // 用途：检查条目是否存在
            // 说明：处理遍历结束的情况
            if let Some(v) = v {
                // 用途：将文件路径添加到结果列表
                // 说明：记录目录中的文件路径
                result.push(v.path().display().to_string());
            } else {
                // 用途：退出循环
                // 说明：目录遍历结束
                break;
            }
        }
        
        // 用途：返回目录中的文件列表
        // 说明：返回成功获取的文件路径列表
        Ok(result)
    }

    // 用途：从本地存储删除文件
    // 说明：删除指定路径的文件，支持文件删除功能
    async fn remove(&self, name: String) -> Result<()> {
        // 用途：将字符串路径转换为PathBuf
        // 说明：方便进行文件操作
        let name = PathBuf::from(name);
        
        // 用途：删除文件
        // 说明：从本地文件系统中删除指定文件
        let f = tokio::fs::remove_file(&name).await?;
        
        // 用途：返回操作结果
        // 说明：返回成功删除的结果
        Ok(f)
    }
}
