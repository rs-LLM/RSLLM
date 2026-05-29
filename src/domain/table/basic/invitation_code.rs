use rbatis::crud;
use rbatis::rbdc::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvitationCode {
    pub id: Option<String>,
    pub code: String,
    pub status: String,
    #[serde(rename = "maxUses")]
    pub max_uses: i64,
    #[serde(rename = "usedCount")]
    pub used_count: i64,
    #[serde(rename = "userLevel")]
    pub user_level: Option<String>,
    #[serde(rename = "operatorId")]
    pub operator_id: Option<String>,
    pub remark: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<DateTime>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<DateTime>,
}

crud!(InvitationCode {}, "invitation_code");
