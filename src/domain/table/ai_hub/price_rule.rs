// 用途：导入日期时间类型
// 说明：用于记录规则的生效和失效时间
use rbatis::rbdc::DateTime;
// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据持久化
use serde::{Serialize, Deserialize};
// 用途：导入rbatis的CRUD宏
// 说明：用于自动生成增删改查操作
use rbatis::{crud};

// 用途：价格规则表结构体
// 说明：管理动态价格规则，支持基于用户等级和用量区间的定价策略
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiHubPriceRule {
    // 用途：规则ID
    // 说明：价格规则的唯一标识符
    pub id: Option<String>,
    // 用途：规则名称
    // 说明：价格规则的显示名称
    pub rule_name: String,
    // 用途：适用条件
    // 说明：规则适用的条件，JSON格式存储，如用户等级、用量区间等
    pub conditions: Option<serde_json::Value>,
    // 用途：折扣率
    // 说明：折扣百分比，如0.8表示8折
    pub discount_rate: Option<f64>,
    // 用途：附加费率
    // 说明：额外的费率，如0.1表示额外增加10%费用
    pub additional_rate: Option<f64>,
    // 用途：优先级
    // 说明：规则的优先级，数值越小优先级越高
    pub priority: i32,
    // 用途：生效时间
    // 说明：规则开始生效的时间
    pub effective_start: Option<DateTime>,
    // 用途：失效时间
    // 说明：规则失效的时间
    pub effective_end: Option<DateTime>,
    // 用途：状态
    // 说明：规则的状态，如"active"、"inactive"等
    pub status: String,
    // 用途：描述
    // 说明：规则的详细描述
    pub description: Option<String>,
    // 用途：创建时间
    // 说明：记录的创建时间
    pub created_at: Option<DateTime>,
    // 用途：更新时间
    // 说明：记录的最后更新时间
    pub updated_at: Option<DateTime>,
}

// 用途：生成AiHubPriceRule的CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(AiHubPriceRule {});