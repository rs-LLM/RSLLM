use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TwoFaSetupResponse {
    pub challenge_id: String,
    pub secret: String,
    pub otpauth_url: String,
    pub backup_codes: Vec<String>,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TwoFaStatusResponse {
    pub enabled: bool,
    pub has_backup_codes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TwoFaEnableRequest {
    pub challenge_id: String,
    pub otp_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TwoFaDisableRequest {
    pub otp_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TwoFaBackupCodesRequest {
    pub otp_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TwoFaBackupCodesResponse {
    pub backup_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TwoFaVerifyLoginRequest {
    pub challenge_id: String,
    pub otp_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TwoFaAdminStatsResponse {
    pub total_users: i64,
    pub enabled_users: i64,
    pub enabled_rate: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NeedTwoFaResponse {
    pub challenge_id: String,
    pub expires_in: u64,
}
