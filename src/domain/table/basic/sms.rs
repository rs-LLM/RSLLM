// 用途：导入serde的序列化和反序列化特性
// 说明：支持Sms结构体的JSON序列化和反序列化，方便在网络中传输
use serde::{Deserialize, Serialize};
// 用途：导入HashMap集合类型
// 说明：用于存储短信发送的参数，键值对形式灵活方便
use std::collections::HashMap;

// 用途：派生序列化和反序列化特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输
#[derive(Serialize, Deserialize)]
// 用途：短信结构体定义
// 说明：封装短信发送所需的账号和参数信息
pub struct Sms {
    // 用途：短信发送账号
    // 说明：标识短信发送者的身份，用于认证和计费
    pub account: String,
    // 用途：短信发送参数
    // 说明：存储短信模板变量和对应值，支持动态生成短信内容
    pub args: HashMap<String, String>,
}
