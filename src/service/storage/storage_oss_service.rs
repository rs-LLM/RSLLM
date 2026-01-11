// 用途：导入自定义错误类型Error
// 说明：统一错误处理，确保所有错误都遵循相同的类型规范
use crate::error::{ApplicationError, ApplicationResult};

// 用途：导入存储服务接口
// 说明：实现存储服务的统一接口，支持多种存储实现的切换
use crate::service::IStorageService;

// 用途：导入AWS SDK S3配置相关类型
// 说明：配置S3客户端，包括凭证、区域等
use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};

// 用途：导入ByteStream类型
// 说明：用于处理S3文件的字节流
use aws_sdk_s3::primitives::ByteStream;

// 用途：导入AWS SDK S3客户端和配置
// 说明：创建和配置S3客户端，用于与S3兼容存储服务交互
use aws_sdk_s3::{Client, Config};

// 用途：导入serde序列化和反序列化宏
// 说明：支持结构体的JSON序列化和反序列化
use serde::{Deserialize, Serialize};

// 用途：导入Cow类型
// 说明：用于处理字符串的借用和所有权，优化内存使用
use std::borrow::Cow;

// 用途：导入PathBuf类型
// 说明：处理文件路径，支持路径操作
use std::path::PathBuf;

// 用途：导入Tokio异步读取扩展trait
// 说明：支持异步读取S3返回的字节流
use tokio::io::AsyncReadExt;

// 用途：导入异步trait宏
// 说明：实现异步trait方法，支持异步存储操作
use async_trait::async_trait;

// 用途：派生Debug trait
// 说明：自动生成Debug实现，方便调试输出
#[derive(Debug)]

// 用途：S3兼容存储服务结构体
// 说明：使用S3兼容存储服务（如阿里云OSS、MinIO等）进行文件存储
pub struct FileS3Service {
    // 用途：S3客户端
    // 说明：用于与S3兼容存储服务进行交互
    client: Client,
    // 用途：存储桶名称
    // 说明：指定文件存储的S3存储桶
    bucket: String,
}

// 用途：派生序列化和反序列化trait
// 说明：支持S3Config结构体的JSON序列化和反序列化
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]

// 用途：S3配置结构体
// 说明：存储S3兼容存储服务的配置信息
pub struct S3Config {
    // 用途：存储桶名称
    // 说明：指定文件存储的S3存储桶
    pub bucket: String,
    // 用途：S3服务端点URL
    // 说明：指定S3兼容服务的访问地址
    pub endpoint: String,
    // 用途：访问密钥
    // 说明：用于身份验证，访问S3服务
    pub access_key: String,
    // 用途：密钥
    // 说明：用于身份验证，访问S3服务
    pub secret_key: String,
    // 用途：存储区域
    // 说明：指定S3存储的区域
    pub region: String,
}

impl S3Config {
    // 用途：从字符串加载S3配置
    // 说明：将字符串形式的S3配置转换为S3Config结构体
    pub fn load(arg: &str) -> ApplicationResult<S3Config> {
        // 用途：检查配置字符串是否以"s3://"开头
        // 说明：确保配置格式正确，符合预期
        if arg.starts_with("s3://") {
            // 用途：解析JSON配置
            // 说明：将去掉"s3://"前缀的字符串解析为S3Config结构体
            let v = serde_json::from_str(arg.trim_start_matches("s3://")).map_err(|e| {
                ApplicationError::ConfigError {
                    message: e.to_string(),
                    key: Some("s3_config".to_string()),
                }
            })?;
            Ok(v)
        } else {
            // 用途：返回错误
            // 说明：配置格式不正确，缺少"s3://"前缀
            Err(ApplicationError::ConfigError {
                message: "s3 must have prefix 's3://'".to_string(),
                key: Some("s3_config".to_string()),
            })
        }
    }
}

impl FileS3Service {
    // 用途：创建FileS3Service实例
    // 说明：根据S3Config配置创建并初始化S3存储服务实例
    pub fn new(cfg: S3Config) -> Self {
        // 用途：创建S3凭证
        // 说明：使用配置中的访问密钥和密钥创建凭证，用于身份验证
        let credentials = Credentials::new(cfg.access_key, cfg.secret_key, None, None, "minio");

        // 用途：构建S3客户端配置
        // 说明：配置S3客户端，包括区域、凭证、端点等
        let config = Config::builder()
            // 用途：设置存储区域
            // 说明：如果配置中未指定区域，则使用默认的"us-east-1"
            .region(Region::new({
                if cfg.region.is_empty() {
                    Cow::Borrowed("us-east-1")
                } else {
                    Cow::Owned(cfg.region)
                }
            }))
            // 用途：设置凭证提供器
            // 说明：使用创建的凭证进行身份验证
            .credentials_provider(credentials)
            // 用途：设置S3服务端点URL
            // 说明：指定S3兼容服务的访问地址
            .endpoint_url(cfg.endpoint)
            // 用途：设置行为版本
            // 说明：使用最新的行为版本，确保与S3服务兼容
            .behavior_version(BehaviorVersion::latest())
            // 用途：构建配置
            // 说明：完成配置构建，生成最终的S3客户端配置
            .build();

        // 用途：创建S3客户端
        // 说明：使用配置创建S3客户端实例
        let client = Client::from_conf(config);

        // 用途：返回FileS3Service实例
        // 说明：创建并返回初始化后的S3存储服务实例
        Self {
            client,
            bucket: cfg.bucket.to_string(),
        }
    }
}

// 用途：实现IStorageService异步trait
// 说明：提供S3兼容存储的具体实现，支持统一的存储服务接口
#[async_trait]
impl IStorageService for FileS3Service {
    // 用途：上传文件到S3存储
    // 说明：将文件数据上传到S3兼容存储服务
    async fn upload(&self, name: String, data: Vec<u8>) -> ApplicationResult<String> {
        // 用途：去除路径开头的斜杠
        // 说明：S3键名不应以斜杠开头，避免路径错误
        let name = name.trim_start_matches("/").to_string();

        // 用途：将字符串路径转换为PathBuf
        // 说明：方便进行路径操作
        let name = PathBuf::from(name);

        // 用途：执行S3 PutObject操作
        // 说明：将文件数据上传到指定的S3存储桶和键
        let _resp = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(name.to_str().unwrap_or_default())
            .body(ByteStream::from(data))
            .send()
            .await
            .map_err(|e| ApplicationError::StorageError {
                message: e.to_string(),
                operation: Some("put_object".to_string()),
                bucket: Some(self.bucket.clone()),
            })?;

        // 用途：返回文件路径
        // 说明：返回成功上传的文件路径，方便后续访问
        Ok(name.to_str().unwrap_or_default().to_string())
    }

    // 用途：从S3存储下载文件
    // 说明：从S3兼容存储服务下载文件数据
    async fn download(&self, name: String) -> ApplicationResult<Vec<u8>> {
        // 用途：去除路径开头的斜杠
        // 说明：S3键名不应以斜杠开头，避免路径错误
        let name = name.trim_start_matches("/").to_string();

        // 用途：将字符串路径转换为PathBuf
        // 说明：方便进行路径操作
        let name = PathBuf::from(name);

        // 用途：执行S3 GetObject操作
        // 说明：从指定的S3存储桶和键下载文件数据
        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(name.to_str().unwrap_or_default())
            .send()
            .await
            .map_err(|e| ApplicationError::StorageError {
                message: e.to_string(),
                operation: Some("get_object".to_string()),
                bucket: Some(self.bucket.clone()),
            })?;

        // 用途：创建数据缓冲区
        // 说明：用于存储下载的文件数据
        let mut buf = vec![];

        // 用途：读取文件数据
        // 说明：将S3返回的字节流读取到缓冲区
        resp.body.into_async_read().read_to_end(&mut buf).await?;

        // 用途：返回文件数据
        // 说明：返回成功下载的文件数据
        Ok(buf)
    }

    // 用途：列出S3存储中的文件
    // 说明：获取指定路径下的所有文件列表
    async fn list(&self, name: String) -> ApplicationResult<Vec<String>> {
        // 用途：去除路径开头的斜杠
        // 说明：S3键名不应以斜杠开头，避免路径错误
        let name = name.trim_start_matches("/").to_string();

        // 用途：将字符串路径转换为PathBuf
        // 说明：方便进行路径操作
        let name = PathBuf::from(name);

        // 用途：执行S3 ListObjectsV2操作
        // 说明：列出指定前缀下的所有对象
        let resp = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(name.to_str().unwrap_or_default())
            .send()
            .await
            .map_err(|e| ApplicationError::StorageError {
                message: e.to_string(),
                operation: Some("list_objects_v2".to_string()),
                bucket: Some(self.bucket.clone()),
            })?;

        // 用途：创建结果列表
        // 说明：用于存储文件列表
        let mut data = vec![];

        // 用途：遍历返回的对象列表
        // 说明：逐个获取对象的键名
        for object in resp.contents() {
            // 用途：将对象键名添加到结果列表
            // 说明：记录S3存储中的文件路径
            data.push(object.key().unwrap_or_default().to_string());
        }

        // 用途：返回文件列表
        // 说明：返回成功获取的文件路径列表
        Ok(data)
    }

    // 用途：从S3存储删除文件
    // 说明：删除指定路径的文件
    async fn remove(&self, name: String) -> ApplicationResult<()> {
        // 用途：去除路径开头的斜杠
        // 说明：S3键名不应以斜杠开头，避免路径错误
        let name = name.trim_start_matches("/").to_string();

        // 用途：将字符串路径转换为PathBuf
        // 说明：方便进行路径操作
        let name = PathBuf::from(name);

        // 用途：执行S3 DeleteObject操作
        // 说明：删除指定的S3对象
        let _resp = self
            .client
            .delete_object()
            .bucket(&self.bucket)
            .key(name.to_str().unwrap_or_default())
            .send()
            .await
            .map_err(|e| ApplicationError::StorageError {
                message: e.to_string(),
                operation: Some("delete_object".to_string()),
                bucket: Some(self.bucket.clone()),
            })?;

        // 用途：返回操作结果
        // 说明：返回成功删除的结果
        Ok(())
    }
}
