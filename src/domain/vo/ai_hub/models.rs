use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicModelsResponse {
    pub object: String,
    pub data: Vec<PublicModelInfoVO>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OpenAIStandardModel {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub owned_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OpenAIStandardModelsResponse {
    pub object: String,
    pub data: Vec<OpenAIStandardModel>,
}
