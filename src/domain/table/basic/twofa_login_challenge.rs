use rbatis::crud;
use rbatis::rbdc::DateTime;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TwoFaLoginChallenge {
    pub id: Option<String>,
    pub user_id: Option<String>,
    pub account: Option<String>,
    pub expires_at: Option<DateTime>,
    pub created_at: Option<DateTime>,
}

crud!(TwoFaLoginChallenge {}, "twofa_login_challenge");
