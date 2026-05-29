use rbatis::crud;
use rbatis::rbdc::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RedeemCode {
    pub id: Option<String>,
    pub code: String,
    pub amount: f64,
    pub status: String,
    #[serde(rename = "maxUses")]
    pub max_uses: i64,
    #[serde(rename = "usedCount")]
    pub used_count: i64,
    #[serde(rename = "usedBy")]
    pub used_by: Option<String>,
    #[serde(rename = "usedAt")]
    pub used_at: Option<DateTime>,
    #[serde(rename = "operatorId")]
    pub operator_id: Option<String>,
    pub remark: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<DateTime>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<DateTime>,
}

crud!(RedeemCode {}, "redeem_code");
