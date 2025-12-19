// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：创建价格规则DTO
// 说明：用于创建新的价格规则
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct CreatePriceRuleDTO {
    // 用途：规则名称
    pub rule_name: String,
    // 用途：适用条件
    pub conditions: Option<serde_json::Value>,
    // 用途：折扣率
    pub discount_rate: Option<f64>,
    // 用途：附加费率
    pub additional_rate: Option<f64>,
    // 用途：优先级
    pub priority: i32,
    // 用途：生效时间
    pub effective_start: Option<String>,
    // 用途：失效时间
    pub effective_end: Option<String>,
    // 用途：状态
    pub status: String,
    // 用途：描述
    pub description: Option<String>,
}

// 用途：更新价格规则DTO
// 说明：用于更新现有的价格规则
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct UpdatePriceRuleDTO {
    // 用途：规则名称
    pub rule_name: Option<String>,
    // 用途：适用条件
    pub conditions: Option<serde_json::Value>,
    // 用途：折扣率
    pub discount_rate: Option<f64>,
    // 用途：附加费率
    pub additional_rate: Option<f64>,
    // 用途：优先级
    pub priority: Option<i32>,
    // 用途：生效时间
    pub effective_start: Option<String>,
    // 用途：失效时间
    pub effective_end: Option<String>,
    // 用途：状态
    pub status: Option<String>,
    // 用途：描述
    pub description: Option<String>,
}

// 用途：价格规则查询DTO
// 说明：用于查询价格规则列表
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct PriceRuleQueryDTO {
    // 用途：规则名称
    pub rule_name: Option<String>,
    // 用途：状态
    pub status: Option<String>,
    // 用途：优先级
    pub priority: Option<i32>,
    // 用途：当前时间是否生效
    pub active_only: Option<bool>,
    // 用途：页码
    pub page: Option<i64>,
    // 用途：每页数量
    pub page_size: Option<i64>,
}

// 用途：价格计算DTO
// 说明：用于计算特定请求的价格
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct PriceCalculationDTO {
    // 用途：基础价格（分/1000 tokens）
    pub base_price: f64,
    // 用途：输入token数量
    pub input_tokens: i64,
    // 用途：输出token数量
    pub output_tokens: i64,
    // 用途：用户等级
    pub user_level: Option<String>,
    // 用途：总用量（分）
    pub total_usage: Option<f64>,
    // 用途：是否应用规则
    pub apply_rules: Option<bool>,
}

// 用途：批量价格规则DTO
// 说明：用于批量创建或更新价格规则
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct BatchPriceRuleDTO {
    // 用途：价格规则列表
    pub rules: Vec<CreatePriceRuleDTO>,
    // 用途：是否覆盖同名规则
    pub overwrite: Option<bool>,
}