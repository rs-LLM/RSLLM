// 用途：导入序列化支持
// 说明：用于响应结构的JSON转换和数据传输
use serde::Serialize;

// 用途：模型列表响应结构体
// 说明：用于返回可用模型的列表信息
#[derive(Serialize)]
pub struct ModelListResponse {
    // 用途：对象类型
    // 说明：标识响应对象的类型，固定为"list"
    pub object: String,
    // 用途：模型数据列表
    // 说明：包含所有可用模型的基本信息
    pub data: Vec<ModelInfoResponse>,
}

// 用途：模型信息响应结构体
// 说明：用于表示单个模型的详细信息
#[derive(Serialize)]
pub struct ModelInfoResponse {
    // 用途：模型ID
    // 说明：模型的唯一标识符，如"gpt-4"、"claude-3-sonnet"等
    pub id: String,
    // 用途：对象类型
    // 说明：标识对象的类型，固定为"model"
    pub object: String,
    // 用途：拥有者
    // 说明：模型的拥有者或开发组织名称
    pub owned_by: String,
}
