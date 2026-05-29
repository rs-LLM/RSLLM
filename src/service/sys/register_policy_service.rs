use rbatis::RBatis;
use rbatis::rbdc::DateTime;

use crate::domain::table::invitation_code::InvitationCode;
use crate::domain::table::key_value_config::KeyValueConfig;
use crate::domain::table::registration_review::RegistrationReview;
use crate::error::{ApplicationError, ApplicationResult};
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use ulid::Ulid;

#[derive(Clone, Debug)]
pub struct RegisterPolicy {
    pub allow_register: bool,
    pub register_email_verify_enabled: bool,
    pub register_captcha_enabled: bool,
    pub invite_code_required: bool,
    pub register_review_enabled: bool,
    pub invite_code_bypass_review: bool,
    pub register_review_reject_notify_enabled: bool,
}

pub struct RegisterPolicyService;

impl RegisterPolicyService {
    async fn get_bool(rb: &RBatis, key: &str, default_val: &str) -> bool {
        let mut conn = match rb.acquire().await {
            Ok(c) => c,
            Err(_) => return !default_val.eq_ignore_ascii_case("false"),
        };
        let val = KeyValueConfig::get_value(&mut conn, key, default_val)
            .await
            .unwrap_or_else(|_| default_val.to_string());
        !val.eq_ignore_ascii_case("false")
    }

    pub async fn get_policy(rb: &RBatis) -> RegisterPolicy {
        RegisterPolicy {
            allow_register: Self::get_bool(rb, "system.allow_register", "true").await,
            register_email_verify_enabled: Self::get_bool(
                rb,
                "system.register_email_verify_enabled",
                "true",
            )
            .await,
            register_captcha_enabled: Self::get_bool(
                rb,
                "system.register_captcha_enabled",
                "false",
            )
            .await,
            invite_code_required: Self::get_bool(rb, "system.invite_code_required", "false").await,
            register_review_enabled: Self::get_bool(rb, "system.register_review_enabled", "false")
                .await,
            invite_code_bypass_review: Self::get_bool(
                rb,
                "system.invite_code_bypass_review",
                "false",
            )
            .await,
            register_review_reject_notify_enabled: Self::get_bool(
                rb,
                "system.register_review_reject_notify_enabled",
                "true",
            )
            .await,
        }
    }

    pub async fn find_invite_code(rb: &RBatis, code: &str) -> ApplicationResult<InvitationCode> {
        let input = code.trim().to_uppercase();
        if input.is_empty() {
            return Err(ApplicationError::ValidationError {
                message: "邀请码不能为空".to_string(),
                field: Some("invite_code".to_string()),
                value: None,
            });
        }
        let list = InvitationCode::select_by_map(rb, rbs::value! {"code": input.clone()}).await?;
        let invite = list
            .into_iter()
            .next()
            .ok_or_else(|| ApplicationError::NotFound {
                message: "邀请码不存在".to_string(),
                resource: Some("invitation_code".to_string()),
                id: Some(input.clone()),
            })?;
        if invite.status != "active" {
            return Err(ApplicationError::ValidationError {
                message: "邀请码不可用".to_string(),
                field: Some("invite_code".to_string()),
                value: Some(invite.status.clone()),
            });
        }
        if invite.used_count >= invite.max_uses {
            return Err(ApplicationError::ValidationError {
                message: "邀请码已达到使用上限".to_string(),
                field: Some("invite_code".to_string()),
                value: Some(input),
            });
        }
        Ok(invite)
    }

    pub async fn consume_invite_code(
        rb: &RBatis,
        code: &str,
        operator_id: Option<String>,
    ) -> ApplicationResult<()> {
        let input = code.trim().to_uppercase();
        if input.is_empty() {
            return Err(ApplicationError::ValidationError {
                message: "邀请码不能为空".to_string(),
                field: Some("invite_code".to_string()),
                value: None,
            });
        }

        let tx = rb.acquire_begin().await?;
        let now = DateTime::now();

        let affected = tx
            .exec(
                "UPDATE invitation_code SET usedCount = usedCount + 1, status = CASE WHEN usedCount + 1 >= maxUses THEN 'used' ELSE status END, operatorId = ?, updatedAt = ? WHERE code = ? AND status = 'active' AND usedCount < maxUses",
                vec![
                    rbs::Value::from(operator_id.clone().unwrap_or_default()),
                    rbs::Value::from(now),
                    rbs::Value::from(input.clone()),
                ],
            )
            .await?;

        if affected.rows_affected == 0 {
            tx.rollback().await?;
            return Err(ApplicationError::BusinessError {
                message: "邀请码已用尽或不可用".to_string(),
                code: Some("INVITE_CODE_EXHAUSTED".to_string()),
                context: Some("register_policy.consume_invite_code".to_string()),
            });
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn create_review(
        rb: &RBatis,
        name: String,
        email: String,
        password: String,
        agree_terms: bool,
        invite_code: Option<String>,
        user_level: Option<String>,
        apply_reason: Option<String>,
    ) -> ApplicationResult<RegistrationReview> {
        let existing = RegistrationReview::select_by_map(
            rb,
            rbs::value! {"email": email.clone(), "status": "pending"},
        )
        .await?;
        if !existing.is_empty() {
            return Err(ApplicationError::ValidationError {
                message: "该邮箱已有待审核申请，请耐心等待".to_string(),
                field: Some("email".to_string()),
                value: Some(email),
            });
        }

        let now = DateTime::now();
        let item = RegistrationReview {
            id: Some(Ulid::new().to_string()),
            name,
            email,
            password,
            agree_terms,
            invite_code,
            user_level,
            apply_reason,
            status: "pending".to_string(),
            review_note: None,
            notify_on_reject: true,
            reviewed_by: None,
            reviewed_at: None,
            created_at: Some(now.clone()),
            updated_at: Some(now),
        };
        RegistrationReview::insert(rb, &item).await?;
        Ok(item)
    }

    pub async fn send_mail(
        rb: &RBatis,
        to_email: &str,
        subject: &str,
        body: &str,
    ) -> Result<(), String> {
        let mut conn = rb
            .acquire()
            .await
            .map_err(|e| format!("数据库连接失败: {}", e))?;
        let smtp_host = KeyValueConfig::get_value(&mut conn, "system.smtp_host", "")
            .await
            .unwrap_or_default();
        let smtp_port = KeyValueConfig::get_value(&mut conn, "system.smtp_port", "465")
            .await
            .unwrap_or_else(|_| "465".to_string());
        let smtp_username = KeyValueConfig::get_value(&mut conn, "system.smtp_username", "")
            .await
            .unwrap_or_default();
        let smtp_password = KeyValueConfig::get_value(&mut conn, "system.smtp_password", "")
            .await
            .unwrap_or_default();
        let smtp_from = KeyValueConfig::get_value(&mut conn, "system.smtp_from", "")
            .await
            .unwrap_or_default();
        let smtp_from_name = KeyValueConfig::get_value(&mut conn, "system.smtp_from_name", "RSLLM")
            .await
            .unwrap_or_else(|_| "RSLLM".to_string());
        let smtp_starttls = KeyValueConfig::get_value(&mut conn, "system.smtp_starttls", "true")
            .await
            .unwrap_or_else(|_| "true".to_string());

        if smtp_host.is_empty()
            || smtp_username.is_empty()
            || smtp_password.is_empty()
            || smtp_from.is_empty()
        {
            return Err("系统未配置完整发信服务".to_string());
        }

        let from_address = smtp_from
            .parse()
            .map_err(|_| "发件邮箱配置无效".to_string())?;
        let to_address = to_email
            .trim()
            .parse()
            .map_err(|_| "收件邮箱格式无效".to_string())?;

        let message = Message::builder()
            .from(Mailbox::new(Some(smtp_from_name), from_address))
            .to(to_address)
            .subject(subject)
            .body(body.to_string())
            .map_err(|e| format!("邮件构造失败: {}", e))?;

        let port = smtp_port.parse::<u16>().unwrap_or(465);
        let credentials = Credentials::new(smtp_username, smtp_password);
        let starttls = !smtp_starttls.eq_ignore_ascii_case("false");
        let mailer = if starttls {
            let builder = AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_host)
                .map_err(|e| format!("SMTP配置错误: {}", e))?;
            builder.port(port).credentials(credentials).build()
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&smtp_host)
                .port(port)
                .credentials(credentials)
                .build()
        };

        mailer
            .send(message)
            .await
            .map_err(|e| format!("邮件发送失败: {}", e))?;
        Ok(())
    }
}
