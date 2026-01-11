// 用途：导入全局上下文
// 说明：用于访问应用程序配置
use crate::context::CONTEXT;
// 用途：导入fast_log的配置结构体
// 说明：用于配置日志系统
use fast_log::config::Config;
// 用途：导入fast_log的日志大小常量
// 说明：用于解析日志文件大小配置
use fast_log::consts::LogSize;
// 用途：导入fast_log的文件分割插件相关类型
// 说明：用于配置日志文件的滚动、保留和压缩策略
use fast_log::plugin::file_split::{DateType, KeepType, Packer, Rolling, RollingType};
// 用途：导入rbatis的DateTime类型
// 说明：用于处理日期时间相关的日志配置
use rbatis::rbdc::DateTime;
// 用途：导入标准库的Duration类型
// 说明：用于配置日志保留时间
use std::time::Duration;

/// 用途：初始化日志系统
/// 说明：配置并启动fast_log日志系统
pub fn init_log() {
    // 用途：创建日志配置
    // 说明：初始化fast_log的配置对象
    let mut cfg = Config::new()
        // 用途：设置日志通道长度
        // 说明：控制日志处理的缓冲大小，避免日志丢失
        .chan_len(CONTEXT.config.log_chan_len)
        // 用途：设置日志级别
        // 说明：根据配置文件指定的日志级别过滤日志输出
        .level(parse_log_level(&CONTEXT.config.log_level))
        // 用途：配置文件分割策略
        // 说明：设置日志文件的存储目录、滚动方式、保留策略和压缩方式
        .file_split(
            &CONTEXT.config.log_dir, // 日志存储目录
            // 用途：创建滚动策略
            // 说明：根据配置解析日志滚动方式
            Rolling::new(parse_rolling_type(CONTEXT.config.log_rolling.as_str())),
            // 用途：解析日志保留策略
            // 说明：设置日志文件的保留方式
            parse_keep_type(&CONTEXT.config.log_keep_type),
            // 用途：解析日志压缩方式
            // 说明：设置日志文件的压缩策略
            parse_packer(&CONTEXT.config.log_pack_compress),
        );
    // 用途：检查是否为调试模式
    // 说明：调试模式下同时输出日志到控制台
    if CONTEXT.config.debug() {
        // 用途：添加控制台输出
        // 说明：调试模式下方便开发者查看日志
        cfg = cfg.console();
    }
    // 用途：初始化日志系统
    // 说明：启动fast_log日志处理
    let _ = fast_log::init(cfg);
    // 用途：检查是否为发布模式
    // 说明：发布模式下提示日志输出配置
    if !CONTEXT.config.debug() {
        // 用途：输出发布模式提示
        // 说明：告知用户当前运行模式下的日志配置
        println!("[rsllm] release_mode is up! [file_log] open,[console_log] disabled!");
    }
}

/// 用途：解析日志滚动类型
/// 说明：将配置文件中的滚动策略字符串转换为fast_log的RollingType枚举
fn parse_rolling_type(log_rolling: &str) -> RollingType {
    // 用途：转换为小写字符串
    // 说明：统一大小写，便于后续比较
    let lower = log_rolling.to_lowercase();
    // 用途：声明滚动类型变量
    // 说明：用于存储解析后的滚动类型
    let rolling_type;
    // 用途：检查是否以"B"结尾（表示按大小滚动）
    // 说明：大小滚动格式如"100MB"、"1GB"等
    if log_rolling.ends_with("B") {
        // 用途：按大小滚动
        // 说明：根据配置的文件大小进行日志滚动
        rolling_type = RollingType::BySize(parse_log_size(&CONTEXT.config.log_rolling));
    // 用途：检查是否按时间滚动
    // 说明：时间滚动格式如"minute"、"hour"、"day"等
    } else if lower.as_str().ends_with("minute")
        || lower.as_str().ends_with("hour")
        || lower.as_str().ends_with("day")
    {
        // 用途：匹配具体的时间单位
        // 说明：根据不同的时间单位选择对应的滚动类型
        match lower.as_str() {
            // 用途：按分钟滚动
            // 说明：每分钟创建一个新的日志文件
            "minute" => {
                rolling_type = RollingType::ByDate(DateType::Minute);
            }
            // 用途：按小时滚动
            // 说明：每小时创建一个新的日志文件
            "hour" => {
                rolling_type = RollingType::ByDate(DateType::Hour);
            }
            // 用途：按天滚动
            // 说明：每天创建一个新的日志文件
            "day" => {
                rolling_type = RollingType::ByDate(DateType::Day);
            }
            // 用途：处理带数字的时间滚动（如"30minute"、"6hour"）
            // 说明：支持自定义时间间隔的滚动策略
            _ => {
                // 用途：检查是否为分钟间隔
                // 说明：如"30minute"表示每30分钟滚动一次
                if lower.ends_with("minute") {
                    // 用途：提取数字部分
                    // 说明：获取具体的时间间隔数值
                    let value: u64 = lower
                        .trim_end_matches("minute")
                        .parse()
                        .expect("parse number fail");
                    // 用途：按自定义分钟间隔滚动
                    // 说明：根据指定的分钟数创建滚动策略
                    rolling_type = RollingType::ByDuration((
                        DateTime::now().0,               // 当前时间
                        Duration::from_secs(value * 60), // 转换为秒
                    ));
                // 用途：检查是否为小时间隔
                // 说明：如"6hour"表示每6小时滚动一次
                } else if lower.ends_with("hour") {
                    // 用途：提取数字部分
                    // 说明：获取具体的时间间隔数值
                    let value: u64 = lower
                        .trim_end_matches("hour")
                        .parse()
                        .expect("parse number fail");
                    // 用途：按自定义小时间隔滚动
                    // 说明：根据指定的小时数创建滚动策略
                    rolling_type = RollingType::ByDuration((
                        DateTime::now().0,                    // 当前时间
                        Duration::from_secs(value * 60 * 60), // 转换为秒
                    ));
                // 用途：检查是否为天间隔
                // 说明：如"7day"表示每7天滚动一次
                } else if lower.ends_with("day") {
                    // 用途：提取数字部分
                    // 说明：获取具体的时间间隔数值
                    let value: u64 = lower
                        .trim_end_matches("day")
                        .parse()
                        .expect("parse number fail");
                    // 用途：按自定义天间隔滚动
                    // 说明：根据指定的天数创建滚动策略
                    rolling_type = RollingType::ByDuration((
                        DateTime::now().0,                         // 当前时间
                        Duration::from_secs(value * 24 * 60 * 60), // 转换为秒
                    ));
                // 用途：处理未知滚动类型
                // 说明：避免程序继续执行错误的滚动策略
                } else {
                    panic!("unknown log_rolling '{}'", log_rolling);
                }
            }
        }
    // 用途：处理未知滚动类型
    // 说明：避免程序继续执行错误的滚动策略
    } else {
        panic!("unknown log_rolling '{}'", log_rolling);
    }
    // 用途：返回解析后的滚动类型
    // 说明：将解析结果用于日志配置
    rolling_type
}

/// 用途：解析日志压缩方式
/// 说明：将配置文件中的压缩策略字符串转换为fast_log的Packer实现
fn parse_packer(_packer: &str) -> Box<dyn Packer> {
    Box::new(fast_log::plugin::packer::LogPacker {})
}

/// 用途：解析日志文件大小
/// 说明：将配置文件中的大小字符串转换为fast_log的LogSize枚举
fn parse_log_size(arg: &str) -> LogSize {
    // 用途：匹配大小单位
    // 说明：根据不同的单位转换为对应的LogSize枚举
    match arg {
        // 用途：匹配MB单位
        // 说明：如"100MB"表示100兆字节
        arg if arg.ends_with("MB") => {
            // 用途：查找MB的位置
            // 说明：提取数字部分
            let end = arg.find("MB").unwrap_or_default();
            // 用途：提取数字字符串
            // 说明：转换为数值
            let num = arg[0..end].to_string();
            // 用途：创建MB大小的LogSize
            // 说明：返回对应的日志大小枚举
            LogSize::MB(num.parse::<usize>().unwrap_or_default())
        }
        // 用途：匹配KB单位
        // 说明：如"1024KB"表示1024千字节
        arg if arg.ends_with("KB") => {
            // 用途：查找KB的位置
            // 说明：提取数字部分
            let end = arg.find("KB").unwrap_or_default();
            // 用途：提取数字字符串
            // 说明：转换为数值
            let num = arg[0..end].to_string();
            // 用途：创建KB大小的LogSize
            // 说明：返回对应的日志大小枚举
            LogSize::KB(num.parse::<usize>().unwrap_or_default())
        }
        // 用途：匹配GB单位
        // 说明：如"1GB"表示1吉字节
        arg if arg.ends_with("GB") => {
            // 用途：查找GB的位置
            // 说明：提取数字部分
            let end = arg.find("GB").unwrap_or_default();
            // 用途：提取数字字符串
            // 说明：转换为数值
            let num = arg[0..end].to_string();
            // 用途：创建GB大小的LogSize
            // 说明：返回对应的日志大小枚举
            LogSize::GB(num.parse::<usize>().unwrap_or_default())
        }
        // 用途：默认日志大小
        // 说明：当无法解析大小时使用默认值100MB
        _ => LogSize::MB(100),
    }
}

/// 用途：解析日志保留策略
/// 说明：将配置文件中的保留策略字符串转换为fast_log的KeepType枚举
fn parse_keep_type(arg: &str) -> KeepType {
    // 用途：匹配保留策略格式
    // 说明：根据不同的格式创建对应的KeepType枚举
    match arg {
        // 用途：匹配KeepNum格式
        // 说明：如"KeepNum(10)"表示保留最近10个日志文件
        arg if arg.starts_with("KeepNum(") => {
            // 用途：查找右括号位置
            // 说明：提取括号内的数字
            let end = arg.find(")").unwrap_or_default();
            // 用途：提取数字字符串
            // 说明：转换为数值
            let num = arg["KeepNum(".len()..end].to_string();
            // 用途：创建按数量保留的KeepType
            // 说明：返回对应的日志保留策略
            KeepType::KeepNum(num.parse::<i64>().unwrap_or_default())
        }
        // 用途：匹配KeepTime格式
        // 说明：如"KeepTime(86400)"表示保留最近24小时的日志文件
        arg if arg.starts_with("KeepTime(") => {
            // 用途：查找右括号位置
            // 说明：提取括号内的数字
            let end = arg.find(")").unwrap_or_default();
            // 用途：提取数字字符串
            // 说明：转换为数值
            let num = arg["KeepTime(".len()..end].to_string();
            // 用途：创建按时间保留的KeepType
            // 说明：返回对应的日志保留策略
            KeepType::KeepTime(Duration::from_secs(num.parse::<u64>().unwrap_or_default()))
        }
        // 用途：匹配ALL格式
        // 说明："ALL"表示保留所有日志文件
        arg if arg.to_uppercase().as_str() == "ALL" => KeepType::All,
        // 用途：处理未知保留策略
        // 说明：避免程序继续执行错误的保留策略
        _ => {
            panic!("unknown keep_type '{}'", arg)
        }
    }
}

/// 用途：解析日志级别
/// 说明：将配置文件中的日志级别字符串转换为log::LevelFilter枚举
fn parse_log_level(arg: &str) -> log::LevelFilter {
    // 用途：匹配日志级别
    // 说明：根据不同的级别字符串返回对应的LevelFilter枚举
    match arg {
        // 用途：关闭日志
        // 说明："off"表示不输出任何日志
        "off" => log::LevelFilter::Off,
        // 用途：警告级别
        // 说明："warn"表示只输出警告及以上级别的日志
        "warn" => log::LevelFilter::Warn,
        // 用途：错误级别
        // 说明："error"表示只输出错误及以上级别的日志
        "error" => log::LevelFilter::Error,
        // 用途：跟踪级别
        // 说明："trace"表示输出所有级别的日志，包括最详细的跟踪信息
        "trace" => log::LevelFilter::Trace,
        // 用途：信息级别
        // 说明："info"表示输出信息、警告、错误级别的日志
        "info" => log::LevelFilter::Info,
        // 用途：调试级别
        // 说明："debug"表示输出调试、信息、警告、错误级别的日志
        "debug" => log::LevelFilter::Debug,
        // 用途：默认日志级别
        // 说明：当无法解析级别时使用默认的Info级别
        _ => log::LevelFilter::Info,
    }
}
