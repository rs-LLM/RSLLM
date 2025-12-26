// 用途：导入serde的序列化和反序列化特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输数据
use serde::{Deserialize, Serialize};

// 用途：派生序列化、反序列化、克隆和调试特性
// 说明：支持JSON序列化和反序列化，方便在网络中传输；支持克隆和调试，便于开发和测试
#[derive(Serialize, Deserialize, Clone, Debug)]
// 用途：刷新令牌请求数据传输对象
// 说明：封装刷新访问令牌所需的刷新令牌信息
pub struct RefreshTokenDTO {
    // 用途：刷新令牌
    // 说明：用于获取新的访问令牌
    pub refresh_token: String,
}
