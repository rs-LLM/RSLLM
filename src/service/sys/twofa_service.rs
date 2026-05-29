use crate::context::{CONTEXT, ServiceContext};
use crate::domain::dto::basic::{
    SignInDTO, TwoFaAdminStatsResponse, TwoFaBackupCodesRequest, TwoFaBackupCodesResponse,
    TwoFaDisableRequest, TwoFaEnableRequest, TwoFaSetupResponse, TwoFaStatusResponse,
    TwoFaVerifyLoginRequest,
};
use crate::domain::table::{
    sys_user::SysUser, twofa_backup_code::TwoFaBackupCode,
    twofa_login_challenge::TwoFaLoginChallenge, twofa_profile::TwoFaProfile,
};
use crate::domain::vo::SignInVO;
use crate::error::Error;
use crate::{error_info, pool};
use data_encoding::{BASE32, BASE32_NOPAD};
use hmac::{Hmac, Mac};
use rand::RngCore;
use rbatis::rbdc::DateTime;
use rbs::value;
use sha1::Sha1;
use sha2::Sha256;
use std::sync::Arc;
use std::time::Duration;
use ulid::Ulid;

const TWOFA_SETUP_TTL_SECONDS: u64 = 600;
const TWOFA_LOGIN_CHALLENGE_TTL_SECONDS: i64 = 600;
const TWOFA_SETUP_KEY_PREFIX: &str = "twofa:setup:";
const TWOFA_ISSUER: &str = "RSLLM";
const TOTP_SECRET_BYTES_LEN: usize = 20;
const TOTP_TIME_STEP_SECONDS: i64 = 30;
const TOTP_ALLOWED_DRIFT_STEPS: i64 = 1;
const BACKUP_CODE_PEPPER: &str = "rsllm-twofa-backup-v1";
const BACKUP_CODE_HASH_COST: u32 = 120_000;

#[derive(Clone)]
pub struct TwoFaService;

impl TwoFaService {
    pub async fn get_status(&self, user_id: &str) -> Result<TwoFaStatusResponse, Error> {
        let profile = TwoFaProfile::select_by_map(pool!(), value! {"user_id": user_id})
            .await?
            .into_iter()
            .next();

        let enabled = profile.as_ref().and_then(|v| v.enabled).unwrap_or(0) == 1;

        let backup_codes = TwoFaBackupCode::select_by_map(pool!(), value! {"user_id": user_id})
            .await?
            .into_iter()
            .filter(|v| v.used_at.is_none())
            .count();

        Ok(TwoFaStatusResponse {
            enabled,
            has_backup_codes: backup_codes > 0,
        })
    }

    pub async fn get_admin_stats(&self) -> Result<TwoFaAdminStatsResponse, Error> {
        #[derive(Debug, Clone, serde::Deserialize)]
        struct CountRow {
            cnt: i64,
        }

        let conn = pool!();

        let total_rows: Vec<CountRow> = conn
            .query_decode("SELECT COUNT(1) AS cnt FROM sys_user", vec![])
            .await?;
        let total_users = total_rows.first().map(|v| v.cnt).unwrap_or(0);

        let enabled_rows: Vec<CountRow> = conn
            .query_decode(
                "SELECT COUNT(1) AS cnt FROM twofa_profile WHERE enabled = 1",
                vec![],
            )
            .await?;
        let enabled_users = enabled_rows.first().map(|v| v.cnt).unwrap_or(0);

        let enabled_rate = if total_users > 0 {
            format!(
                "{:.1}%",
                (enabled_users as f64 / total_users as f64) * 100.0
            )
        } else {
            "0.0%".to_string()
        };

        Ok(TwoFaAdminStatsResponse {
            total_users,
            enabled_users,
            enabled_rate,
        })
    }

    pub async fn setup(
        &self,
        context: &Arc<ServiceContext>,
        user_id: &str,
    ) -> Result<TwoFaSetupResponse, Error> {
        let user = SysUser::select_by_map(pool!(), value! {"id": user_id})
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| Error::from(error_info!("user_not_exists")))?;

        let account = user.account.clone().unwrap_or_else(|| user_id.to_string());

        let secret = generate_base32_secret(TOTP_SECRET_BYTES_LEN);
        let setup_id = Ulid::new().to_string();
        let backup_codes = generate_backup_codes();

        let setup_payload = SetupPayload {
            user_id: user_id.to_string(),
            secret: secret.clone(),
            backup_codes: backup_codes.clone(),
        };

        let payload_json = serde_json::to_string(&setup_payload)
            .map_err(|e| Error::from(format!("序列化2FA setup失败: {}", e)))?;

        context
            .cache_service
            .set_string_ex(
                &format!("{}{}", TWOFA_SETUP_KEY_PREFIX, setup_id),
                &payload_json,
                Some(Duration::from_secs(TWOFA_SETUP_TTL_SECONDS)),
            )
            .await?;

        let otpauth_url = format!(
            "otpauth://totp/{}:{}?secret={}&issuer={}&digits=6&period=30",
            TWOFA_ISSUER, account, secret, TWOFA_ISSUER
        );

        Ok(TwoFaSetupResponse {
            challenge_id: setup_id,
            secret,
            otpauth_url,
            backup_codes,
            expires_in: TWOFA_SETUP_TTL_SECONDS,
        })
    }

    pub async fn enable(
        &self,
        context: &Arc<ServiceContext>,
        user_id: &str,
        req: &TwoFaEnableRequest,
    ) -> Result<(), Error> {
        if req.challenge_id.trim().is_empty() || req.otp_code.trim().is_empty() {
            return Err(Error::from(error_info!("empty")));
        }

        let key = format!("{}{}", TWOFA_SETUP_KEY_PREFIX, req.challenge_id.trim());
        let payload_raw = context.cache_service.get_string(&key).await?;
        if payload_raw.trim().is_empty() {
            return Err(Error::from("2FA setup 已过期，请重新发起"));
        }

        let payload: SetupPayload = serde_json::from_str(&payload_raw)
            .map_err(|e| Error::from(format!("解析2FA setup失败: {}", e)))?;

        if payload.user_id != user_id {
            return Err(Error::from(error_info!("access_denied")));
        }

        if !verify_totp(&payload.secret, &req.otp_code) {
            return Err(Error::from(error_info!("vcode_error")));
        }

        let conn = pool!();

        let mut profile = TwoFaProfile::select_by_map(conn, value! {"user_id": user_id})
            .await?
            .into_iter()
            .next()
            .unwrap_or(TwoFaProfile {
                id: Some(Ulid::new().to_string()),
                user_id: Some(user_id.to_string()),
                secret: None,
                enabled: Some(0),
                created_at: Some(DateTime::now()),
                updated_at: Some(DateTime::now()),
            });

        profile.secret = Some(payload.secret.clone());
        profile.enabled = Some(1);
        profile.updated_at = Some(DateTime::now());

        if TwoFaProfile::select_by_map(conn, value! {"user_id": user_id})
            .await?
            .is_empty()
        {
            let _ = TwoFaProfile::insert(conn, &profile).await?;
        } else {
            let _ =
                TwoFaProfile::update_by_map(conn, &profile, value! {"user_id": user_id}).await?;
        }

        let _ = TwoFaBackupCode::delete_by_map(conn, value! {"user_id": user_id}).await?;

        for code in payload.backup_codes {
            let code_hash = hash_backup_code(user_id, &code);
            let row = TwoFaBackupCode {
                id: Some(Ulid::new().to_string()),
                user_id: Some(user_id.to_string()),
                code_hash: Some(code_hash),
                used_at: None,
                created_at: Some(DateTime::now()),
            };
            let _ = TwoFaBackupCode::insert(conn, &row).await?;
        }

        let _ = context.cache_service.del(&key).await;
        Ok(())
    }

    pub async fn disable(&self, user_id: &str, req: &TwoFaDisableRequest) -> Result<(), Error> {
        if req.otp_code.trim().is_empty() {
            return Err(Error::from(error_info!("vcode_error")));
        }

        let conn = pool!();
        let profile = TwoFaProfile::select_by_map(conn, value! {"user_id": user_id})
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| Error::from("2FA 未启用"))?;

        if profile.enabled.unwrap_or(0) != 1 {
            return Err(Error::from("2FA 未启用"));
        }

        let secret = profile.secret.clone().unwrap_or_default();
        if secret.is_empty() {
            return Err(Error::from("2FA 配置异常"));
        }

        if !verify_totp(&secret, &req.otp_code)
            && !self.consume_backup_code(user_id, &req.otp_code).await?
        {
            return Err(Error::from(error_info!("vcode_error")));
        }

        let mut update = profile;
        update.enabled = Some(0);
        update.secret = None;
        update.updated_at = Some(DateTime::now());
        let _ = TwoFaProfile::update_by_map(conn, &update, value! {"user_id": user_id}).await?;

        let _ = TwoFaBackupCode::delete_by_map(conn, value! {"user_id": user_id}).await?;
        let _ = TwoFaLoginChallenge::delete_by_map(conn, value! {"user_id": user_id}).await?;
        Ok(())
    }

    pub async fn regenerate_backup_codes(
        &self,
        user_id: &str,
        req: &TwoFaBackupCodesRequest,
    ) -> Result<TwoFaBackupCodesResponse, Error> {
        if req.otp_code.trim().is_empty() {
            return Err(Error::from(error_info!("vcode_error")));
        }

        let conn = pool!();
        let profile = TwoFaProfile::select_by_map(conn, value! {"user_id": user_id})
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| Error::from("2FA 未启用"))?;

        if profile.enabled.unwrap_or(0) != 1 {
            return Err(Error::from("2FA 未启用"));
        }

        let secret = profile.secret.clone().unwrap_or_default();
        if secret.is_empty() {
            return Err(Error::from("2FA 配置异常"));
        }

        if !verify_totp(&secret, &req.otp_code)
            && !self.consume_backup_code(user_id, &req.otp_code).await?
        {
            return Err(Error::from(error_info!("vcode_error")));
        }

        let backup_codes = generate_backup_codes();
        let _ = TwoFaBackupCode::delete_by_map(conn, value! {"user_id": user_id}).await?;

        for code in &backup_codes {
            let code_hash = hash_backup_code(user_id, code);
            let row = TwoFaBackupCode {
                id: Some(Ulid::new().to_string()),
                user_id: Some(user_id.to_string()),
                code_hash: Some(code_hash),
                used_at: None,
                created_at: Some(DateTime::now()),
            };
            let _ = TwoFaBackupCode::insert(conn, &row).await?;
        }

        Ok(TwoFaBackupCodesResponse { backup_codes })
    }

    pub async fn sign_in_or_create_challenge(
        &self,
        arg: &SignInDTO,
    ) -> Result<SignInResult, Error> {
        CONTEXT
            .sys_user_service
            .is_need_wait_login_ex(&arg.username)
            .await?;

        let user: Option<SysUser> =
            SysUser::select_by_map(pool!(), value! {"account": &arg.username})
                .await?
                .into_iter()
                .next();

        let user = user.ok_or_else(|| {
            Error::from(format!(
                "{}={}",
                error_info!("account_not_exists"),
                arg.username
            ))
        })?;

        if user.state == Some(0) {
            return Err(Error::from(error_info!("account_disabled")));
        }

        let user_id = user.id.clone().unwrap_or_default();
        let profile = TwoFaProfile::select_by_map(pool!(), value! {"user_id": &user_id})
            .await?
            .into_iter()
            .next();

        let twofa_enabled = profile.as_ref().and_then(|v| v.enabled).unwrap_or(0) == 1;

        let sign_in_vo = CONTEXT.sys_user_service.sign_in(arg).await?;

        if !twofa_enabled {
            return Ok(SignInResult::Success(sign_in_vo));
        }

        let challenge_id = Ulid::new().to_string();
        let expires_at = DateTime::from_timestamp(
            DateTime::now().unix_timestamp() + TWOFA_LOGIN_CHALLENGE_TTL_SECONDS,
        );

        let row = TwoFaLoginChallenge {
            id: Some(challenge_id.clone()),
            user_id: Some(user_id),
            account: Some(arg.username.clone()),
            expires_at: Some(expires_at),
            created_at: Some(DateTime::now()),
        };

        let _ =
            TwoFaLoginChallenge::delete_by_map(pool!(), value! {"account": &arg.username}).await;
        let _ = TwoFaLoginChallenge::insert(pool!(), &row).await?;

        Ok(SignInResult::NeedTwoFa {
            challenge_id,
            expires_in: TWOFA_LOGIN_CHALLENGE_TTL_SECONDS as u64,
        })
    }

    pub async fn verify_login(&self, req: &TwoFaVerifyLoginRequest) -> Result<SignInVO, Error> {
        if req.challenge_id.trim().is_empty() || req.otp_code.trim().is_empty() {
            return Err(Error::from(error_info!("empty")));
        }

        let challenge =
            TwoFaLoginChallenge::select_by_map(pool!(), value! {"id": req.challenge_id.trim()})
                .await?
                .into_iter()
                .next()
                .ok_or_else(|| Error::from("2FA challenge 不存在或已过期"))?;

        if challenge
            .expires_at
            .as_ref()
            .map(|v| v.unix_timestamp() <= DateTime::now().unix_timestamp())
            .unwrap_or(true)
        {
            let _ =
                TwoFaLoginChallenge::delete_by_map(pool!(), value! {"id": req.challenge_id.trim()})
                    .await;
            return Err(Error::from("2FA challenge 已过期"));
        }

        let user_id = challenge.user_id.clone().unwrap_or_default();
        let account = challenge.account.clone().unwrap_or_default();
        if user_id.is_empty() {
            return Err(Error::from("2FA challenge 数据异常"));
        }

        let profile = TwoFaProfile::select_by_map(pool!(), value! {"user_id": &user_id})
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| Error::from("2FA 未配置"))?;

        let secret = profile.secret.clone().unwrap_or_default();
        if secret.is_empty() {
            return Err(Error::from("2FA 配置异常"));
        }

        let mut verified = verify_totp(&secret, &req.otp_code);
        if !verified {
            verified = self.consume_backup_code(&user_id, &req.otp_code).await?;
        }

        if !verified {
            CONTEXT
                .sys_user_service
                .add_retry_login_limit_num(&account)
                .await?;
            return Err(Error::from(error_info!("vcode_error")));
        }

        let _ = TwoFaLoginChallenge::delete_by_map(pool!(), value! {"id": req.challenge_id.trim()})
            .await;

        let user = SysUser::select_by_map(pool!(), value! {"id": &user_id})
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| Error::from(error_info!("user_not_exists")))?;

        CONTEXT.sys_user_service.get_user_info(&user).await
    }

    pub async fn admin_reset_by_user_id(&self, user_id: &str) -> Result<(), Error> {
        if user_id.trim().is_empty() {
            return Err(Error::from(error_info!("id_empty")));
        }

        let _ = TwoFaProfile::delete_by_map(pool!(), value! {"user_id": user_id}).await;
        let _ = TwoFaBackupCode::delete_by_map(pool!(), value! {"user_id": user_id}).await;
        let _ = TwoFaLoginChallenge::delete_by_map(pool!(), value! {"user_id": user_id}).await;
        Ok(())
    }

    async fn consume_backup_code(&self, user_id: &str, input: &str) -> Result<bool, Error> {
        let code = normalize_backup_code(input);
        if code.len() != 8 || !code.chars().all(|c| c.is_ascii_digit()) {
            return Ok(false);
        }

        let code_hash = hash_backup_code(user_id, &code);
        let now = DateTime::now();
        let result = pool!()
            .exec(
                "UPDATE twofa_backup_code SET used_at = ? WHERE user_id = ? AND code_hash = ? AND used_at IS NULL",
                vec![rbs::value!(now), rbs::value!(user_id), rbs::value!(code_hash)],
            )
            .await?;

        Ok(result.rows_affected > 0)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SetupPayload {
    user_id: String,
    secret: String,
    backup_codes: Vec<String>,
}

pub enum SignInResult {
    Success(SignInVO),
    NeedTwoFa {
        challenge_id: String,
        expires_in: u64,
    },
}

fn generate_base32_secret(bytes_len: usize) -> String {
    let mut bytes = vec![0u8; bytes_len];
    rand::rng().fill_bytes(&mut bytes);
    BASE32_NOPAD.encode(&bytes)
}

fn generate_backup_codes() -> Vec<String> {
    use rand::Rng;
    let mut rng = rand::rng();
    (0..8)
        .map(|_| format!("{:08}", rng.random_range(0..100_000_000_u32)))
        .collect()
}

fn normalize_backup_code(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>()
}

fn hash_backup_code(user_id: &str, code: &str) -> String {
    let normalized_user_id = user_id.trim();
    let mut digest = [0u8; 32];
    pbkdf2_sha256(
        format!("{}:{}", BACKUP_CODE_PEPPER, code).as_bytes(),
        normalized_user_id.as_bytes(),
        BACKUP_CODE_HASH_COST,
        &mut digest,
    );
    format_hex_lower(&digest)
}

fn pbkdf2_sha256(password: &[u8], salt: &[u8], iterations: u32, out: &mut [u8]) {
    type HmacSha256 = Hmac<Sha256>;

    if out.is_empty() || iterations == 0 {
        return;
    }

    let hash_len = 32usize;
    let blocks = out.len().div_ceil(hash_len);

    for block_index in 1..=blocks {
        let mut salt_block = Vec::with_capacity(salt.len() + 4);
        salt_block.extend_from_slice(salt);
        salt_block.extend_from_slice(&(block_index as u32).to_be_bytes());

        let mut mac = match HmacSha256::new_from_slice(password) {
            Ok(v) => v,
            Err(_) => return,
        };
        mac.update(&salt_block);
        let mut u = mac.finalize().into_bytes().to_vec();
        let mut t = u.clone();

        for _ in 1..iterations {
            let mut mac = match HmacSha256::new_from_slice(password) {
                Ok(v) => v,
                Err(_) => return,
            };
            mac.update(&u);
            u = mac.finalize().into_bytes().to_vec();
            for (ti, ui) in t.iter_mut().zip(&u) {
                *ti ^= *ui;
            }
        }

        let start = (block_index - 1) * hash_len;
        let end = usize::min(start + hash_len, out.len());
        out[start..end].copy_from_slice(&t[..(end - start)]);
    }
}

fn format_hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

fn verify_totp(secret: &str, otp_code: &str) -> bool {
    let code = otp_code.trim();
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }

    let Some(secret_bytes) = decode_base32_secret(secret) else {
        return false;
    };

    let now_step = DateTime::now().unix_timestamp() / TOTP_TIME_STEP_SECONDS;
    let code_num = match code.parse::<u32>() {
        Ok(v) => v,
        Err(_) => return false,
    };

    for offset in -TOTP_ALLOWED_DRIFT_STEPS..=TOTP_ALLOWED_DRIFT_STEPS {
        let counter = now_step + offset;
        if counter < 0 {
            continue;
        }
        if generate_totp_code(&secret_bytes, counter as u64) == code_num {
            return true;
        }
    }

    false
}

fn decode_base32_secret(secret: &str) -> Option<Vec<u8>> {
    let normalized: String = secret
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect::<String>()
        .to_ascii_uppercase();

    if normalized.is_empty() {
        return None;
    }

    BASE32_NOPAD.decode(normalized.as_bytes()).ok().or_else(|| {
        let mut padded = normalized.clone();
        let rem = padded.len() % 8;
        if rem != 0 {
            padded.push_str(&"=".repeat(8 - rem));
        }
        BASE32.decode(padded.as_bytes()).ok()
    })
}

fn generate_totp_code(secret: &[u8], counter: u64) -> u32 {
    let mut mac = match Hmac::<Sha1>::new_from_slice(secret) {
        Ok(v) => v,
        Err(_) => return 0,
    };
    mac.update(&counter.to_be_bytes());
    let digest = mac.finalize().into_bytes();

    let offset = (digest[19] & 0x0f) as usize;
    let binary = ((u32::from(digest[offset]) & 0x7f) << 24)
        | (u32::from(digest[offset + 1]) << 16)
        | (u32::from(digest[offset + 2]) << 8)
        | u32::from(digest[offset + 3]);

    binary % 1_000_000
}
