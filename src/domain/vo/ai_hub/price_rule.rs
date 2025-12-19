// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：价格规则VO结构体
// 说明：用于API响应的价格规则数据
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AiHubPriceRuleVO {
    // 用途：规则ID
    pub id: Option<String>,
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
    // 用途：创建时间
    pub created_at: Option<String>,
}

// 用途：价格计算结果VO结构体
// 说明：用于展示价格计算的结果
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct PriceCalculationVO {
    // 用途：基础价格（分/1000 tokens）
    pub base_price: f64,
    // 用途：应用的规则列表
    pub applied_rules: Vec<AppliedRuleVO>,
    // 用途：最终价格（分/1000 tokens）
    pub final_price: f64,
    // 用途：折扣金额（分）
    pub discount_amount: f64,
    // 用途：附加费用（分）
    pub additional_amount: f64,
    // 用途：总费用（分）
    pub total_amount: f64,
}

// 用途：应用的规则VO结构体
// 说明：用于展示应用到价格计算的规则详情
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct AppliedRuleVO {
    // 用途：规则ID
    pub rule_id: String,
    // 用途：规则名称
    pub rule_name: String,
    // 用途：折扣率
    pub discount_rate: Option<f64>,
    // 用途：附加费率
    pub additional_rate: Option<f64>,
    // 用途：优先级
    pub priority: i32,
    // 用途：规则影响金额（分）
    pub impact_amount: f64,
}

// 用途：价格规则概览VO结构体
// 说明：用于展示价格规则的整体概览
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct PriceRuleOverviewVO {
    // 用途：活跃规则数量
    pub active_rules: i32,
    // 用途：即将生效的规则数量
    pub pending_rules: i32,
    // 用途：已过期的规则数量
    pub expired_rules: i32,
    // 用途：规则列表
    pub rules: Vec<AiHubPriceRuleVO>,
}