//! 公共模型视图对象模块。
//! 定义 AI Hub 对外暴露的模型信息与 OpenAI 兼容模型列表响应结构。

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 内部公共模型信息结构。
pub struct PublicModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub model_code: String,
    pub model_type: String,
    pub input_price: f64,
    pub output_price: f64,
    pub currency: String,
    pub max_tokens_per_request: Option<i32>,
    pub max_requests_per_minute: Option<i32>,
}

/// 对外暴露的公共模型信息视图对象。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicModelInfoVO {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub model_code: String,
    pub model_type: String,
    pub input_price: f64,
    pub output_price: f64,
    pub currency: String,
    pub max_tokens_per_request: Option<i32>,
    pub max_requests_per_minute: Option<i32>,
}

/// 公共模型列表响应。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicModelsResponse {
    pub object: String,
    pub data: Vec<PublicModelInfoVO>,
    pub total: usize,
}

/// OpenAI 标准模型项。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OpenAIStandardModel {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub owned_by: String,
}

/// OpenAI 标准模型列表响应。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OpenAIStandardModelsResponse {
    pub object: String,
    pub data: Vec<OpenAIStandardModel>,
}
