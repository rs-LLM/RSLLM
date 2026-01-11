// 用途：导入标准库中的HashMap集合
// 说明：用于存储配置中的错误信息映射
use std::collections::HashMap;
// 用途：导入标准库中的File结构体
// 说明：用于打开和读取配置文件
use std::fs::File;
// 用途：导入标准库中的Read trait
// 说明：用于从文件中读取配置数据
use std::io::Read;

/// 用途：应用程序配置结构体
/// 说明：集中管理应用程序的所有配置项
#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize, Clone)]
pub struct ApplicationConfig {
    pub server_url: String, // 用途：服务器监听地址
    // 说明：指定服务器启动时监听的IP和端口
    pub db_url: String, // 用途：数据库连接URL
    // 说明：用于连接数据库服务器
    pub db_pool_len: usize, // 用途：数据库连接池大小
    // 说明：控制同时连接数据库的最大连接数
    pub db_pool_timeout: usize, // 用途：数据库连接超时时间
    // 说明：设置获取数据库连接的超时时间
    pub log_dir: String, // 用途：日志文件存储目录
    // 说明：指定日志文件的保存位置
    pub log_rolling: String, // 用途：日志滚动策略
    // 说明：控制日志文件的滚动方式（按大小、按时间等）
    pub log_pack_compress: String, // 用途：日志压缩策略
    // 说明：指定日志文件的压缩方式
    pub log_keep_type: String, // 用途：日志保留策略
    // 说明：控制日志文件的保留时间或数量
    pub log_level: String, // 用途：日志级别
    // 说明：指定日志输出的最低级别（debug、info、warn、error）
    pub log_chan_len: Option<usize>, // 用途：日志通道长度
    // 说明：控制日志处理的缓冲大小
    pub sms_cache_send_key_prefix: String, // 用途：短信发送缓存键前缀
    // 说明：用于生成短信发送缓存的唯一标识
    pub jwt_secret: String, // 用途：JWT密钥
    // 说明：用于JWT令牌的签名和验证
    pub jwt_exp: usize, // 用途：JWT过期时间
    // 说明：设置JWT令牌的有效时间（秒）
    pub jwt_refresh_token: usize, // 用途：JWT刷新令牌过期时间
    // 说明：设置刷新令牌的有效时间（秒）
    pub cache: String, // 用途：缓存配置
    // 说明：指定缓存的类型和配置
    pub storage: String, // 用途：存储配置
    // 说明：指定文件存储的类型和配置
    pub login_fail_retry: u64, // 用途：登录失败重试次数
    // 说明：控制登录失败后允许重试的最大次数
    pub login_fail_retry_wait_sec: u64, // 用途：登录失败重试等待时间
    // 说明：设置登录失败后需要等待的时间（秒）
    pub trash_recycle_days: u64, // 用途：回收站回收天数
    // 说明：设置回收站中数据的自动清理时间（天）
    pub datetime_format: String, // 用途：日期时间格式
    // 说明：统一应用程序中日期时间的格式化方式
    pub errors: HashMap<String, String>, // 用途：错误码映射
    // 说明：存储错误码与错误信息的对应关系
    #[serde(default)]
    pub error_infos: HashMap<String, String>, // 用途：错误信息反向映射
                                              // 说明：存储错误信息与错误码的对应关系，用于反向查找
}

/// 用途：ApplicationConfig的默认实现
/// 说明：提供默认的配置加载方式
impl Default for ApplicationConfig {
    fn default() -> Self {
        // 用途：打开配置文件
        // 说明：从application.json5文件中加载配置
        let mut f = File::open("application.json5").expect("not find 'application.json5'");
        // 用途：创建空字符串用于存储配置数据
        // 说明：准备存储从文件中读取的配置内容
        let mut cfg_data = "".to_string();
        // 用途：从文件中读取配置数据到字符串
        // 说明：将文件内容转换为字符串以便解析
        f.read_to_string(&mut cfg_data)
            .expect("read 'application.json5' fail");
        // 用途：解析配置数据
        // 说明：将JSON5格式的配置转换为ApplicationConfig结构体
        let mut result: ApplicationConfig =
            json5::from_str(&cfg_data).expect("load config file fail");
        // 用途：初始化配置信息
        // 说明：处理配置中的错误信息映射
        result.init_infos();
        // 用途：返回初始化后的配置
        // 说明：提供完整的配置对象给调用者
        result
    }
}

/// 用途：ApplicationConfig的方法实现
/// 说明：提供配置的各种操作方法
impl ApplicationConfig {
    /// 用途：根据错误码获取错误信息
    /// 说明：用于统一处理错误信息的获取
    pub fn get_error_info(&self, code: &str) -> String {
        // 用途：查找错误码对应的错误信息
        // 说明：根据传入的错误码返回对应的错误描述
        match self.errors.get(code) {
            // 用途：如果找不到指定错误码，查找默认错误码
            // 说明：确保总能返回一个错误信息，避免程序崩溃
            None => match self.errors.get("-1") {
                // 用途：如果默认错误码也找不到，返回通用错误信息
                // 说明：作为最后的 fallback，确保程序不会因错误处理而崩溃
                None => "unknown error".to_string(),
                // 用途：返回默认错误信息
                // 说明：当找不到指定错误码时使用默认错误信息
                Some(v) => v.to_string(),
            },
            // 用途：返回找到的错误信息
            // 说明：返回与错误码对应的具体错误描述
            Some(v) => v.as_str().to_string(),
        }
    }

    /// 用途：初始化错误信息映射
    /// 说明：构建错误信息到错误码的反向映射，用于快速查找
    pub fn init_infos(&mut self) {
        // 用途：创建新的错误信息映射
        // 说明：清空原有映射，准备重新构建
        self.error_infos = HashMap::new();
        // 用途：遍历错误码映射
        // 说明：为每个错误码构建反向映射
        for (k, error) in &self.errors {
            // 用途：复制错误信息字符串
            // 说明：避免直接修改原数据
            let mut error = error.to_string();
            // 用途：检查错误信息是否包含逗号
            // 说明：错误信息可能包含额外的描述，只取逗号前的部分作为关键字
            if error.contains(",") {
                // 用途：截取逗号前的错误信息
                // 说明：只使用错误信息的核心部分作为关键字
                error = error[0..error.find(",").unwrap_or_default()].to_string();
            }
            // 用途：构建错误信息到错误码的映射
            // 说明：用于根据错误信息快速查找对应的错误码
            self.error_infos.insert(error, k.to_string());
        }
    }

    /// 用途：检查是否为调试模式
    /// 说明：根据编译模式返回对应的运行模式
    pub fn debug(&self) -> bool {
        // 用途：检查编译断言
        // 说明：使用Rust内置的调试断言来判断当前运行模式
        cfg!(debug_assertions)
    }
}

/// 用途：错误信息宏
/// 说明：提供便捷的错误信息获取方式
#[macro_export]
macro_rules! error_info {
    // 用途：宏定义
    // 说明：当调用error_info!宏时，自动调用CONTEXT.config.get_error_info方法
    ($code: expr) => {
        $crate::context::CONTEXT.config.get_error_info($code)
    };
}
