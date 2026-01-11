// 用途：导入serde的序列化和反序列化特性
// 说明：支持LoginCheck枚举的JSON序列化和反序列化
use serde::{Deserializer, Serializer};
// 用途：导入格式化相关特性
// 说明：支持LoginCheck枚举的调试和显示格式化
use std::fmt::{Debug, Display, Formatter};
// 用途：登录检查枚举
// 说明：定义不同的登录验证方式，用于控制用户登录行为
#[derive(Clone, Default)]
pub enum LoginCheck {
    // 用途：无需检查
    // 说明：允许用户直接登录，无需额外验证
    #[default]
    NoCheck,
    // 用途：密码检查
    // 说明：仅需要密码验证即可登录
    PasswordCheck,
    // 用途：密码+图片验证码检查
    // 说明：需要密码和图片验证码双重验证，防止自动化攻击
    PasswordImgCodeCheck,
    // 用途：手机验证码检查
    // 说明：通过手机验证码进行登录验证，提高安全性
    PhoneCodeCheck,
}

// 用途：为LoginCheck实现Default trait
// 说明：提供默认值，方便在未指定时使用

// 用途：实现LoginCheck到&str的转换
// 说明：方便将枚举转换为字符串，用于存储和传输
impl From<LoginCheck> for &str {
    // 用途：转换方法
    // 说明：将不同的登录检查类型映射为对应的字符串
    fn from(arg: LoginCheck) -> Self {
        match arg {
            LoginCheck::NoCheck => "",
            LoginCheck::PasswordCheck => "PasswordCheck",
            LoginCheck::PasswordImgCodeCheck => "PasswordImgCodeCheck",
            LoginCheck::PhoneCodeCheck => "PhoneCodeCheck",
        }
    }
}

// 用途：实现&str到LoginCheck的转换
// 说明：方便将字符串转换为枚举，用于从存储或传输中恢复
impl From<&str> for LoginCheck {
    // 用途：转换方法
    // 说明：将字符串映射为对应的登录检查类型，支持容错处理
    fn from(arg: &str) -> Self {
        match arg {
            "" => LoginCheck::NoCheck,
            "NoCheck" => LoginCheck::NoCheck,
            "PasswordCheck" => LoginCheck::PasswordCheck,
            "PasswordImgCodeCheck" => LoginCheck::PasswordImgCodeCheck,
            "PhoneCodeCheck" => LoginCheck::PhoneCodeCheck,
            _ => LoginCheck::NoCheck,
        }
    }
}

// 用途：为LoginCheck实现Debug trait
// 说明：支持调试打印，便于开发和测试
impl Debug for LoginCheck {
    // 用途：调试格式化方法
    // 说明：将枚举转换为字符串进行调试输出
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(<&str>::from(self.clone()))
    }
}

// 用途：为LoginCheck实现Display trait
// 说明：支持直接打印，便于日志记录和用户界面显示
impl Display for LoginCheck {
    // 用途：显示格式化方法
    // 说明：将枚举转换为字符串进行显示输出
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(<&str>::from(self.clone()))
    }
}

// 用途：为LoginCheck实现Serialize trait
// 说明：支持JSON序列化，方便在网络中传输
impl serde::Serialize for LoginCheck {
    // 用途：序列化方法
    // 说明：将枚举转换为字符串后进行序列化
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

// 用途：为LoginCheck实现Deserialize trait
// 说明：支持JSON反序列化，方便从网络中接收数据
impl<'de> serde::Deserialize<'de> for LoginCheck {
    // 用途：反序列化方法
    // 说明：将字符串反序列化为对应的枚举类型
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = String::deserialize(deserializer)?;
        Ok(LoginCheck::from(v.as_str()))
    }
}
