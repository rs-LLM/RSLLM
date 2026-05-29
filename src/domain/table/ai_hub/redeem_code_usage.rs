use rbatis::crud;
use rbatis::rbdc::DateTime;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RedeemCodeUsage {
    pub id: Option<String>,
    pub code_id: String,
    pub user_id: String,
    pub username: String,
    pub created_at: Option<DateTime>,
}

crud!(RedeemCodeUsage {});
