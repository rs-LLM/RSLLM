// 用途：导入日期时间类型
// 说明：用于记录周期开始和结束时间
use rbatis::rbdc::DateTime;
// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据持久化
use serde::{Serialize, Deserialize};
// 用途：导入rbatis的CRUD宏
// 说明：用于自动生成增删改查操作
use rbatis::{crud};

// 用途：用户配额表结构体
// 说明：管理用户的配额和余额，支持不同类型的配额周期
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiHubUserQuota {
    // 用途：配额ID
    // 说明：用户配额的唯一标识符
    pub id: Option<String>,
    // 用途：用户ID
    // 说明：配额所属用户的唯一标识符
    pub user_id: String,
    // 用途：配额类型
    // 说明：配额的周期类型，如"monthly"、"yearly"、"prepaid"等
    pub quota_type: String,
    // 用途：总额度（分）
    // 说明：配额的总金额，单位为分
    pub total_quota: f64,
    // 用途：已用额度（分）
    // 说明：已使用的配额金额，单位为分
    pub used_quota: f64,
    // 用途：剩余额度（分）
    // 说明：剩余的配额金额，单位为分
    pub remaining_quota: f64,
    // 用途：周期开始时间
    // 说明：配额周期的开始时间
    pub cycle_start: Option<DateTime>,
    // 用途：周期结束时间
    // 说明：配额周期的结束时间
    pub cycle_end: Option<DateTime>,
    // 用途：状态
    // 说明：配额的状态，如"active"、"paused"、"expired"等
    pub status: String,
    // 用途：警告阈值（百分比）
    // 说明：触发警告的阈值百分比，如80表示80%时警告
    pub warning_threshold: Option<f64>,
    // 用途：创建时间
    // 说明：记录的创建时间
    pub created_at: Option<DateTime>,
    // 用途：更新时间
    // 说明：记录的最后更新时间
    pub updated_at: Option<DateTime>,
}

// 用途：生成AiHubUserQuota的CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(AiHubUserQuota {});