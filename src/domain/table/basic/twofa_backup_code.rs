use rbatis::crud;
use rbatis::rbdc::DateTime;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TwoFaBackupCode {
    pub id: Option<String>,
    pub user_id: Option<String>,
    pub code_hash: Option<String>,
    pub used_at: Option<DateTime>,
    pub created_at: Option<DateTime>,
}

crud!(TwoFaBackupCode {}, "twofa_backup_code");
