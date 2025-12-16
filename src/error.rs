//! 用途：错误和结果类型定义
//! 说明：提供应用程序统一的错误处理机制

// 用途：导入标准错误 trait
// 说明：用于实现自定义错误类型
use std::error::Error as StdError;

// 用途：导入格式化相关的 trait
// 说明：用于实现错误的显示和调试功能
use std::fmt::{self, Debug, Display};

// 用途：导入IO模块
// 说明：用于处理IO错误的转换
use std::io;

// 用途：导入serde的反序列化相关类型
// 说明：用于实现错误类型的反序列化
use serde::de::Visitor;

// 用途：导入serde的序列化相关类型
// 说明：用于实现错误类型的序列化
use serde::ser::{Serialize, Serializer};

// 用途：导入serde的核心 trait
// 说明：用于实现错误类型的序列化和反序列化
use serde::{Deserialize, Deserializer};

/// 用途：定义应用程序的结果类型别名
/// 说明：简化错误处理，统一使用自定义Error类型
pub type Result<T> = std::result::Result<T, Error>;

/// 用途：通用错误枚举
/// 说明：表示应用程序中所有可能的失败方式
#[derive(Debug)]
#[non_exhaustive] // 用途：标记为非详尽枚举
                  // 说明：允许未来扩展错误类型而不破坏现有代码
pub enum Error {
    /// 用途：默认错误类型
    /// 说明：用于包装字符串形式的错误信息
    E(String),
}

/// 用途：实现Display trait
/// 说明：允许错误类型以友好的方式显示给用户
impl Display for Error {
    // 用途：格式化错误信息
    // 说明：将错误转换为字符串形式
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 用途：匹配错误类型
        // 说明：根据不同的错误类型生成相应的字符串表示
        match self {
            // 用途：处理默认错误类型
            // 说明：将内部字符串作为错误信息输出
            Error::E(error) => write!(f, "{}", error),
        }
    }
}

/// 用途：实现标准错误 trait
/// 说明：允许自定义错误类型与标准库错误处理兼容
impl StdError for Error {}

/// 用途：实现从IO错误到自定义错误的转换
/// 说明：方便处理IO操作产生的错误
impl From<io::Error> for Error {
    #[inline] // 用途：内联提示
             // 说明：减少函数调用开销，提高性能
    fn from(err: io::Error) -> Self {
        // 用途：将IO错误转换为字符串，再转换为自定义错误
        // 说明：统一错误处理，方便上层调用
        Error::from(err.to_string())
    }
}

/// 用途：实现从字符串切片到自定义错误的转换
/// 说明：方便直接使用字符串创建错误
impl From<&str> for Error {
    fn from(arg: &str) -> Self {
        // 用途：将字符串切片转换为自定义错误
        // 说明：允许直接使用字符串字面量创建错误
        return Error::E(arg.to_string());
    }
}

/// 用途：实现从String到自定义错误的转换
/// 说明：方便使用String创建错误
impl From<std::string::String> for Error {
    fn from(arg: String) -> Self {
        // 用途：将String转换为自定义错误
        // 说明：允许直接使用String对象创建错误
        return Error::E(arg);
    }
}

/// 用途：实现从通用错误类型到自定义错误的转换
/// 说明：方便处理各种实现了StdError的错误类型
impl From<&dyn std::error::Error> for Error {
    fn from(arg: &dyn std::error::Error) -> Self {
        // 用途：将通用错误转换为字符串，再转换为自定义错误
        // 说明：统一处理各种类型的错误
        return Error::E(arg.to_string());
    }
}

/// 用途：实现从自定义错误到IO错误的转换
/// 说明：方便在需要IO错误的上下文中使用自定义错误
impl From<Error> for std::io::Error {
    fn from(arg: Error) -> Self {
        // 用途：将自定义错误转换为IO错误
        // 说明：允许在需要IO错误的函数中返回自定义错误
        arg.into()
    }
}

/// 用途：实现从rbatis错误到自定义错误的转换
/// 说明：方便处理数据库操作产生的错误
impl From<rbatis::Error> for Error {
    fn from(arg: rbatis::Error) -> Self {
        // 用途：将rbatis错误转换为自定义错误
        // 说明：统一处理数据库操作错误
        Error::E(arg.to_string())
    }
}

/// 用途：实现Clone trait
/// 说明：允许错误类型被克隆，方便在异步和多线程环境中使用
impl Clone for Error {
    /// 用途：克隆错误对象
    /// 说明：创建一个新的错误对象，包含相同的错误信息
    fn clone(&self) -> Self {
        // 用途：将错误转换为字符串，再创建新的错误对象
        // 说明：实现简单且安全的克隆
        Error::from(self.to_string())
    }

    /// 用途：从源错误克隆到当前错误
    /// 说明：允许将一个错误的内容复制到另一个错误
    fn clone_from(&mut self, source: &Self) {
        // 用途：将源错误转换为字符串，再赋值给当前错误
        // 说明：实现简单且安全的克隆赋值
        *self = Self::from(source.to_string());
    }
}

// 用途：序列化实现注释
// 说明：说明这是 #[derive(Serialize)] 会生成的代码
impl Serialize for Error {
    /// 用途：序列化错误对象
    /// 说明：允许错误类型被序列化为JSON等格式
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 用途：将错误转换为字符串并序列化
        // 说明：错误信息最适合以字符串形式序列化
        serializer.serialize_str(self.to_string().as_str())
    }
}

/// 用途：错误访问者结构体
/// 说明：用于反序列化错误对象
struct ErrorVisitor;

/// 用途：实现Visitor trait
/// 说明：用于自定义错误类型的反序列化
impl<'de> Visitor<'de> for ErrorVisitor {
    /// 用途：定义访问者返回的类型
    /// 说明：反序列化过程中需要临时存储字符串
    type Value = String;

    /// 用途：期望的输入格式
    /// 说明：告知反序列化器期望接收字符串类型
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        // 用途：输出期望的格式
        // 说明：帮助调试反序列化错误
        formatter.write_str("a string")
    }

    /// 用途：处理字符串值
    /// 说明：当反序列化器遇到字符串时调用
    fn visit_string<E>(self, v: String) -> std::result::Result<Self::Value, E>
    where
        E: std::error::Error,
    {
        // 用途：返回接收到的字符串
        // 说明：将字符串作为反序列化的结果
        Ok(v)
    }

    /// 用途：处理字符串切片
    /// 说明：当反序列化器遇到字符串切片时调用
    fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
    where
        E: std::error::Error,
    {
        // 用途：将字符串切片转换为String并返回
        // 说明：统一处理字符串和字符串切片
        Ok(v.to_string())
    }
}

/// 用途：实现Deserialize trait
/// 说明：允许错误类型从JSON等格式反序列化
impl<'de> Deserialize<'de> for Error {
    /// 用途：反序列化错误对象
    /// 说明：将字符串形式的错误转换为自定义错误类型
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // 用途：使用访问者反序列化字符串
        // 说明：将JSON字符串转换为Error类型
        let r = deserializer.deserialize_string(ErrorVisitor)?;
        // 用途：创建自定义错误对象
        // 说明：将反序列化得到的字符串转换为Error类型
        return Ok(Error::from(r));
    }
}
