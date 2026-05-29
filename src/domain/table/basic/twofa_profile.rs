use rbatis::crud;
use rbatis::rbdc::DateTime;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TwoFaProfile {
    pub id: Option<String>,
    pub user_id: Option<String>,
    pub secret: Option<String>,
    pub enabled: Option<i32>,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}

crud!(TwoFaProfile {}, "twofa_profile");
