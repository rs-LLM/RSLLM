// 用途：导入日期时间类型
// 说明：用于记录配额的创建、更新和周期时间
use rbatis::rbdc::DateTime;
// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据持久化
use serde::{Serialize, Deserialize};
// 用途：导入rbatis的CRUD宏
// 说明：用于自动生成增删改查操作
use rbatis::{crud};

// 用途：用户配额表结构体（扩展版）
// 说明：管理用户的配额和余额，支持周期性配额和多级警告
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiHubUserQuota {
    // 用途：配额ID
    // 说明：配额记录的唯一标识符
    pub id: Option<String>,
    // 用途：用户ID
    // 说明：配额所属用户的唯一标识符
    pub user_id: String,
    // 用途：总配额
    // 说明：用户拥有的总配额额度
    pub total_quota: f64,
    // 用途：已用配额
    // 说明：用户已经使用的配额额度
    pub used_quota: f64,
    // 用途：剩余配额
    // 说明：用户剩余的配额额度
    pub remaining_quota: f64,
    // 用途：配额周期
    // 说明：配额的周期类型，如"daily"、"weekly"、"monthly"
    pub quota_period: Option<String>,
    // 用途：周期开始时间
    // 说明：当前配额周期的开始日期
    pub period_start: Option<DateTime>,
    // 用途：周期结束时间
    // 说明：当前配额周期的结束日期
    pub period_end: Option<DateTime>,
    // 用途：状态
    // 说明：配额状态，如"active"、"paused"、"exhausted"
    pub status: Option<String>,
    // 用途：警告阈值（百分比）
    // 说明：触发警告的配额使用百分比，默认80%
    pub warning_threshold: Option<f64>,
    // 用途：临界阈值（百分比）
    // 说明：触发临界警告的配额使用百分比，默认95%
    pub critical_threshold: Option<f64>,
    // 用途：创建时间
    // 说明：记录的创建时间
    pub created_at: Option<DateTime>,
    // 用途：更新时间
    // 说明：记录的最后更新时间
    pub updated_at: Option<DateTime>,
    // 用途：最后使用时间
    // 说明：配额最后一次被使用的时间
    pub last_used_at: Option<DateTime>,
}

// 用途：生成AiHubUserQuota的CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(AiHubUserQuota {});