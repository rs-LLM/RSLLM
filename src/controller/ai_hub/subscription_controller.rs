use axum::{Json, extract::State};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::sync::Arc;

use crate::context::ServiceContext;
use crate::domain::dto::ai_hub::{
    ConfirmSubscriptionPaymentCallbackRequest, ConfirmSubscriptionPaymentCallbackResponse,
    CreateSubscriptionPaymentOrderRequest, CreateTransactionDTO, DeductDTO,
    PurchaseSubscriptionRequest, PurchaseSubscriptionResponse, SelfSubscriptionResponse,
    SubscriptionPaymentOrder, SubscriptionPaymentOrdersResponse, SubscriptionPaymentProviderItem,
    SubscriptionPaymentProvidersResponse, SubscriptionPlanItem, SubscriptionPlansResponse,
    UpdateSubscriptionPlansRequest, UserSubscriptionStatus,
};
use crate::domain::table::basic::SysUser;
use crate::domain::table::key_value_config::KeyValueConfig;
use crate::domain::vo::response::ApiResponse;
use crate::error::{ApplicationError, ApplicationResult};
use crate::middleware::auth_axum::JwtAuth;
use crate::service::ai_hub::{BalanceService, TransactionService};

const SUBSCRIPTION_PLANS_KEY: &str = "system.subscription_plans";
const SUBSCRIPTION_PAYMENT_CALLBACK_TOKEN_KEY: &str = "system.subscription_payment_callback_token";
const SUBSCRIPTION_PAYMENT_STRIPE_CALLBACK_TOKEN_KEY: &str =
    "system.subscription_payment_stripe_callback_token";
const SUBSCRIPTION_PAYMENT_CREEM_CALLBACK_TOKEN_KEY: &str =
    "system.subscription_payment_creem_callback_token";
const SUBSCRIPTION_PAYMENT_EPAY_CALLBACK_TOKEN_KEY: &str =
    "system.subscription_payment_epay_callback_token";
const SUBSCRIPTION_PAYMENT_STRIPE_CHECKOUT_BASE_URL_KEY: &str =
    "system.subscription_payment_stripe_checkout_base_url";
const SUBSCRIPTION_PAYMENT_CREEM_CHECKOUT_BASE_URL_KEY: &str =
    "system.subscription_payment_creem_checkout_base_url";
const SUBSCRIPTION_PAYMENT_EPAY_CHECKOUT_BASE_URL_KEY: &str =
    "system.subscription_payment_epay_checkout_base_url";
const SUBSCRIPTION_PAYMENT_ALLOW_COMBINED_KEY: &str = "system.subscription_payment_allow_combined";
const SUBSCRIPTION_PAYMENT_CALLBACK_TOLERANCE_SECONDS: i64 = 300;

type HmacSha256 = Hmac<Sha256>;

fn user_subscription_key(user_id: &str) -> String {
    format!("user.subscription.{}", user_id)
}

fn payment_order_key(order_id: &str) -> String {
    format!("subscription.payment.order.{}", order_id)
}

fn user_payment_orders_key(user_id: &str) -> String {
    format!("subscription.payment.user_orders.{}", user_id)
}

fn payment_callback_event_key(event_id: &str) -> String {
    format!("subscription.payment.callback.event.{}", event_id)
}

fn build_fallback_callback_event_id(
    provider: &str,
    order_id: &str,
    normalized_status: &str,
    external_txn_id: Option<&str>,
) -> String {
    format!(
        "{}:{}:{}:{}",
        provider.trim().to_lowercase(),
        order_id,
        normalized_status,
        external_txn_id.unwrap_or("-")
    )
}

fn normalize_provider(provider: Option<String>) -> String {
    provider
        .unwrap_or_else(|| "mock".to_string())
        .trim()
        .to_lowercase()
}

fn normalize_callback_status(status: &str) -> ApplicationResult<String> {
    let normalized = status.trim().to_lowercase();
    let mapped = match normalized.as_str() {
        "confirmed" | "paid" | "success" | "succeeded" => "confirmed",
        "pending" | "processing" => "pending",
        "failed" | "fail" | "error" => "failed",
        "canceled" | "cancelled" => "canceled",
        _ => {
            return Err(ApplicationError::BusinessError {
                message: format!("不支持的支付回调状态: {}", status),
                code: Some("SUBSCRIPTION_PAYMENT_CALLBACK_STATUS_INVALID".to_string()),
                context: Some(status.to_string()),
            });
        }
    };
    Ok(mapped.to_string())
}

fn can_transition_payment_status(from: &str, to: &str) -> bool {
    let from = from.trim().to_lowercase();
    let to = to.trim().to_lowercase();
    if from == to {
        return true;
    }

    match from.as_str() {
        "pending" | "processing" => matches!(to.as_str(), "confirmed" | "failed" | "canceled"),
        "failed" | "canceled" => matches!(to.as_str(), "confirmed"),
        "confirmed" => false,
        _ => false,
    }
}

fn build_callback_signing_payload(
    order_id: &str,
    status: &str,
    external_txn_id: Option<&str>,
    event_id: &str,
    timestamp: i64,
) -> String {
    format!(
        "{}:{}:{}:{}:{}",
        order_id,
        status,
        external_txn_id.unwrap_or(""),
        event_id,
        timestamp
    )
}

fn bytes_to_lower_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{:02x}", byte));
    }
    output
}

fn parse_stripe_signature_parts(signature: &str) -> Option<(i64, String)> {
    let mut timestamp: Option<i64> = None;
    let mut v1: Option<String> = None;

    for part in signature.split(',') {
        let mut iter = part.trim().splitn(2, '=');
        let key = iter.next()?.trim();
        let value = iter.next()?.trim();
        if key == "t" {
            timestamp = value.parse::<i64>().ok();
        } else if key == "v1" && !value.is_empty() {
            v1 = Some(value.to_lowercase());
        }
    }

    match (timestamp, v1) {
        (Some(ts), Some(sig)) => Some((ts, sig)),
        _ => None,
    }
}

fn verify_with_standard_hmac(
    secret: &str,
    req: &ConfirmSubscriptionPaymentCallbackRequest,
    normalized_status: &str,
    event_id: &str,
    timestamp: i64,
    signature: &str,
) -> ApplicationResult<()> {
    let payload = build_callback_signing_payload(
        &req.order_id,
        normalized_status,
        req.external_txn_id.as_deref(),
        event_id,
        timestamp,
    );

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).map_err(|e| {
        ApplicationError::BusinessError {
            message: format!("支付回调签名初始化失败: {}", e),
            code: Some("SUBSCRIPTION_PAYMENT_CALLBACK_SIGNER_INVALID".to_string()),
            context: Some(req.order_id.clone()),
        }
    })?;
    mac.update(payload.as_bytes());

    let signature_bytes =
        STANDARD
            .decode(signature)
            .map_err(|_| ApplicationError::BusinessError {
                message: "支付回调签名格式无效".to_string(),
                code: Some("SUBSCRIPTION_PAYMENT_CALLBACK_SIGNATURE_INVALID".to_string()),
                context: Some(req.order_id.clone()),
            })?;

    mac.verify_slice(&signature_bytes)
        .map_err(|_| ApplicationError::BusinessError {
            message: "支付回调签名校验失败".to_string(),
            code: Some("SUBSCRIPTION_PAYMENT_CALLBACK_FORBIDDEN".to_string()),
            context: Some(req.order_id.clone()),
        })
}

fn verify_with_stripe_like_signature(
    secret: &str,
    req: &ConfirmSubscriptionPaymentCallbackRequest,
    normalized_status: &str,
    event_id: &str,
    signature_header: &str,
) -> ApplicationResult<()> {
    let (timestamp, provided_v1) =
        parse_stripe_signature_parts(signature_header).ok_or_else(|| {
            ApplicationError::BusinessError {
                message: "Stripe 回调签名格式无效".to_string(),
                code: Some("SUBSCRIPTION_PAYMENT_CALLBACK_SIGNATURE_INVALID".to_string()),
                context: Some(req.order_id.clone()),
            }
        })?;

    let now = Utc::now().timestamp();
    if (now - timestamp).abs() > SUBSCRIPTION_PAYMENT_CALLBACK_TOLERANCE_SECONDS {
        return Err(ApplicationError::BusinessError {
            message: "支付回调时间戳已过期".to_string(),
            code: Some("SUBSCRIPTION_PAYMENT_CALLBACK_EXPIRED".to_string()),
            context: Some(req.order_id.clone()),
        });
    }

    let payload = build_callback_signing_payload(
        &req.order_id,
        normalized_status,
        req.external_txn_id.as_deref(),
        event_id,
        timestamp,
    );

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).map_err(|e| {
        ApplicationError::BusinessError {
            message: format!("支付回调签名初始化失败: {}", e),
            code: Some("SUBSCRIPTION_PAYMENT_CALLBACK_SIGNER_INVALID".to_string()),
            context: Some(req.order_id.clone()),
        }
    })?;
    mac.update(payload.as_bytes());

    let expected_hex = bytes_to_lower_hex(&mac.finalize().into_bytes());
    if expected_hex != provided_v1 {
        return Err(ApplicationError::BusinessError {
            message: "支付回调签名校验失败".to_string(),
            code: Some("SUBSCRIPTION_PAYMENT_CALLBACK_FORBIDDEN".to_string()),
            context: Some(req.order_id.clone()),
        });
    }

    Ok(())
}

fn verify_callback_signature(
    secret: &str,
    provider: &str,
    req: &ConfirmSubscriptionPaymentCallbackRequest,
    normalized_status: &str,
    event_id: &str,
) -> ApplicationResult<()> {
    if secret.trim().is_empty() {
        return Ok(());
    }

    if event_id.trim().is_empty() {
        return Err(ApplicationError::BusinessError {
            message: "支付回调缺少 event_id".to_string(),
            code: Some("SUBSCRIPTION_PAYMENT_CALLBACK_INVALID_EVENT".to_string()),
            context: Some(req.order_id.clone()),
        });
    }

    if let Some(signature) = req.signature.as_deref() {
        if provider == "stripe" {
            return verify_with_stripe_like_signature(
                secret,
                req,
                normalized_status,
                event_id,
                signature,
            );
        }

        let timestamp = req
            .timestamp
            .ok_or_else(|| ApplicationError::BusinessError {
                message: "支付回调缺少 timestamp".to_string(),
                code: Some("SUBSCRIPTION_PAYMENT_CALLBACK_INVALID_TIMESTAMP".to_string()),
                context: Some(req.order_id.clone()),
            })?;

        let now = Utc::now().timestamp();
        if (now - timestamp).abs() > SUBSCRIPTION_PAYMENT_CALLBACK_TOLERANCE_SECONDS {
            return Err(ApplicationError::BusinessError {
                message: "支付回调时间戳已过期".to_string(),
                code: Some("SUBSCRIPTION_PAYMENT_CALLBACK_EXPIRED".to_string()),
                context: Some(req.order_id.clone()),
            });
        }

        return verify_with_standard_hmac(
            secret,
            req,
            normalized_status,
            event_id,
            timestamp,
            signature,
        );
    }

    let provided = req.callback_token.clone().unwrap_or_default();
    if provided != secret {
        return Err(ApplicationError::BusinessError {
            message: "支付回调令牌无效".to_string(),
            code: Some("SUBSCRIPTION_PAYMENT_CALLBACK_FORBIDDEN".to_string()),
            context: Some(req.order_id.clone()),
        });
    }

    Ok(())
}

fn build_subscription_status(
    user_id: &str,
    plan: &SubscriptionPlanItem,
    transaction_id: &str,
) -> UserSubscriptionStatus {
    let now = Utc::now();
    let cycle = plan.billing_cycle.to_lowercase();
    let expires_at = match cycle.as_str() {
        "yearly" | "annual" => now + Duration::days(365),
        "weekly" => now + Duration::days(7),
        _ => now + Duration::days(30),
    };
    let next_quota_reset_at = match cycle.as_str() {
        "daily" => Some((now + Duration::days(1)).to_rfc3339()),
        "weekly" => Some((now + Duration::days(7)).to_rfc3339()),
        "monthly" | "yearly" | "annual" => Some((now + Duration::days(30)).to_rfc3339()),
        _ => None,
    };

    UserSubscriptionStatus {
        user_id: user_id.to_string(),
        plan_id: plan.id.clone(),
        plan_name: plan.name.clone(),
        billing_cycle: plan.billing_cycle.clone(),
        currency: plan.currency.clone(),
        amount: plan.price,
        status: "active".to_string(),
        started_at: now.to_rfc3339(),
        expires_at: expires_at.to_rfc3339(),
        transaction_id: transaction_id.to_string(),
        quota_total: plan.quota_total.unwrap_or((plan.price * 100.0).max(0.0)),
        quota_used: 0.0,
        quota_reset_cycle: plan.billing_cycle.clone(),
        next_quota_reset_at,
    }
}

fn infer_user_level_from_plan(plan: &SubscriptionPlanItem) -> String {
    if let Some(level) = plan.user_level.as_deref() {
        let normalized = level.trim().to_uppercase();
        if normalized.starts_with('L') && normalized.len() >= 2 {
            return normalized;
        }
    }

    let plan_markers = [plan.id.as_str(), plan.name.as_str()]
        .into_iter()
        .map(|s| s.to_uppercase())
        .collect::<Vec<_>>();
    for marker in plan_markers {
        for candidate in ["L5", "L4", "L3", "L2", "L1"] {
            if marker.contains(candidate) {
                return candidate.to_string();
            }
        }
    }

    if plan.price >= 200.0 {
        "L5".to_string()
    } else if plan.price >= 100.0 {
        "L4".to_string()
    } else if plan.price >= 50.0 {
        "L3".to_string()
    } else {
        "L2".to_string()
    }
}

async fn sync_user_level_with_subscription(
    context: &Arc<ServiceContext>,
    user_id: &str,
    plan: &SubscriptionPlanItem,
) -> ApplicationResult<()> {
    let level_code = infer_user_level_from_plan(plan);
    set_user_level_code(context, user_id, &level_code).await
}

async fn set_user_level_code_with_conn(
    conn: &impl rbatis::executor::Executor,
    user_id: &str,
    level_code: &str,
) -> ApplicationResult<()> {
    let users = SysUser::select_by_map(conn, rbs::value! { "id": user_id }).await?;
    let user = users
        .first()
        .cloned()
        .ok_or_else(|| ApplicationError::NotFound {
            message: format!("用户不存在: {}", user_id),
            resource: Some("user".to_string()),
            id: Some(user_id.to_string()),
        })?;

    SysUser::update_by_map(
        conn,
        &SysUser {
            id: user.id,
            account: user.account,
            password: user.password,
            name: user.name,
            email: user.email,
            avatar: user.avatar,
            login_check: user.login_check,
            state: user.state,
            create_date: user.create_date,
            balance: user.balance,
            user_level: Some(level_code.to_string()),
        },
        rbs::value! { "id": user_id },
    )
    .await?;

    Ok(())
}

async fn set_user_level_code(
    context: &Arc<ServiceContext>,
    user_id: &str,
    level_code: &str,
) -> ApplicationResult<()> {
    let conn = context
        .rb
        .acquire()
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("数据库连接失败: {}", e),
            operation: Some("acquire connection".to_string()),
            table: None,
        })?;

    set_user_level_code_with_conn(&conn, user_id, level_code).await?;
    context
        .rate_limit_service
        .invalidate_user_level_cache(user_id)
        .await;
    Ok(())
}

fn is_subscription_expired(expires_at: &str) -> bool {
    DateTime::parse_from_rfc3339(expires_at)
        .map(|expired_at| expired_at <= Utc::now())
        .unwrap_or(false)
}

fn next_quota_reset_at(cycle: &str, from: DateTime<Utc>) -> Option<String> {
    let next = match cycle.trim().to_lowercase().as_str() {
        "daily" => Some(from + Duration::days(1)),
        "weekly" => Some(from + Duration::days(7)),
        "monthly" => Some(from + Duration::days(30)),
        _ => None,
    }?;
    Some(next.to_rfc3339())
}

fn parse_subscription_kv_row(row: &rbs::Value) -> Option<(String, String)> {
    let row_json = serde_json::to_value(row).ok()?;
    let key = row_json.get("key")?.as_str()?.to_string();
    let value = row_json.get("value")?.as_str()?.to_string();
    Some((key, value))
}

pub async fn run_subscription_maintenance(context: &Arc<ServiceContext>) -> ApplicationResult<()> {
    let conn = context
        .rb
        .acquire()
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("数据库连接失败: {}", e),
            operation: Some("acquire connection".to_string()),
            table: None,
        })?;

    let rows = conn
        .query(
            "SELECT key, value FROM key_value_config WHERE key LIKE ?",
            vec![rbs::value!("user.subscription.%")],
        )
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("查询订阅状态失败: {}", e),
            operation: Some("query subscription kv rows".to_string()),
            table: Some("key_value_config".to_string()),
        })?;

    let mut processed = 0usize;
    if let Some(array) = rows.as_array() {
        for row in array {
            let Some((key, raw_value)) = parse_subscription_kv_row(row) else {
                continue;
            };

            let user_id = key.trim_start_matches("user.subscription.").to_string();
            if user_id.is_empty() {
                continue;
            }

            let mut status = match serde_json::from_str::<UserSubscriptionStatus>(&raw_value) {
                Ok(value) => value,
                Err(_) => continue,
            };

            let mut changed = false;
            let now = Utc::now();
            if let Some(reset_at) = status.next_quota_reset_at.as_deref()
                && let Ok(reset_time) = DateTime::parse_from_rfc3339(reset_at)
                && reset_time.with_timezone(&Utc) <= now
            {
                status.quota_used = 0.0;
                status.next_quota_reset_at = next_quota_reset_at(&status.quota_reset_cycle, now);
                changed = true;
            }

            let normalized = status.status.trim().to_lowercase();
            if matches!(normalized.as_str(), "active" | "trial" | "confirmed")
                && is_subscription_expired(&status.expires_at)
            {
                status.status = "expired".to_string();
                changed = true;
                set_user_level_code_with_conn(&conn, &user_id, "L1").await?;
            }

            if changed {
                persist_subscription_status(context, &user_id, &status).await?;
                processed += 1;
            }
        }
    }

    if processed > 0 {
        log::info!(
            "[subscription-maintenance] processed {} subscription records",
            processed
        );
    }

    Ok(())
}

async fn persist_subscription_status(
    context: &Arc<ServiceContext>,
    user_id: &str,
    status: &UserSubscriptionStatus,
) -> ApplicationResult<()> {
    let payload = serde_json::to_string(status).map_err(|e| ApplicationError::BusinessError {
        message: format!("序列化订阅状态失败: {}", e),
        code: Some("SUBSCRIPTION_STATUS_SERIALIZE_FAILED".to_string()),
        context: Some(user_id.to_string()),
    })?;

    let mut conn = context
        .rb
        .acquire()
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("数据库连接失败: {}", e),
            operation: Some("acquire connection".to_string()),
            table: None,
        })?;

    KeyValueConfig::set_value(
        &mut conn,
        &user_subscription_key(user_id),
        &payload,
        Some("用户当前订阅状态"),
    )
    .await
    .map_err(|e| ApplicationError::DatabaseError {
        message: format!("保存订阅状态失败: {}", e),
        operation: Some("save user subscription".to_string()),
        table: Some("key_value_config".to_string()),
    })?;

    context
        .balance_service
        .invalidate_user_balance_cache(user_id)
        .await;

    Ok(())
}

async fn read_user_subscription_status(
    context: &Arc<ServiceContext>,
    user_id: &str,
) -> ApplicationResult<Option<UserSubscriptionStatus>> {
    let mut conn = context
        .rb
        .acquire()
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("数据库连接失败: {}", e),
            operation: Some("acquire connection".to_string()),
            table: None,
        })?;

    let raw = KeyValueConfig::get_value(&mut conn, &user_subscription_key(user_id), "")
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("读取订阅状态失败: {}", e),
            operation: Some("read self subscription".to_string()),
            table: Some("key_value_config".to_string()),
        })?;

    if raw.trim().is_empty() {
        return Ok(None);
    }

    let mut status = match serde_json::from_str::<UserSubscriptionStatus>(&raw).ok() {
        Some(value) => value,
        None => return Ok(None),
    };

    let normalized = status.status.trim().to_lowercase();
    if matches!(normalized.as_str(), "active" | "trial" | "confirmed")
        && is_subscription_expired(&status.expires_at)
    {
        status.status = "expired".to_string();
        persist_subscription_status(context, user_id, &status).await?;
        set_user_level_code_with_conn(&conn, user_id, "L1").await?;
    }

    Ok(Some(status))
}

async fn store_payment_order(
    context: &Arc<ServiceContext>,
    order: &SubscriptionPaymentOrder,
) -> ApplicationResult<()> {
    let mut conn = context
        .rb
        .acquire()
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("数据库连接失败: {}", e),
            operation: Some("acquire connection".to_string()),
            table: None,
        })?;

    let order_payload =
        serde_json::to_string(order).map_err(|e| ApplicationError::BusinessError {
            message: format!("序列化支付订单失败: {}", e),
            code: Some("SUBSCRIPTION_PAYMENT_ORDER_SERIALIZE_FAILED".to_string()),
            context: Some(order.order_id.clone()),
        })?;

    KeyValueConfig::set_value(
        &mut conn,
        &payment_order_key(&order.order_id),
        &order_payload,
        Some("订阅支付订单"),
    )
    .await
    .map_err(|e| ApplicationError::DatabaseError {
        message: format!("保存支付订单失败: {}", e),
        operation: Some("save payment order".to_string()),
        table: Some("key_value_config".to_string()),
    })?;

    let user_orders_key = user_payment_orders_key(&order.user_id);
    let existing = KeyValueConfig::get_value(&mut conn, &user_orders_key, "[]")
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("读取用户订单索引失败: {}", e),
            operation: Some("read user payment index".to_string()),
            table: Some("key_value_config".to_string()),
        })?;

    let mut order_ids: Vec<String> = serde_json::from_str(&existing).unwrap_or_default();
    if !order_ids.iter().any(|id| id == &order.order_id) {
        order_ids.push(order.order_id.clone());
    }

    let ids_payload =
        serde_json::to_string(&order_ids).map_err(|e| ApplicationError::BusinessError {
            message: format!("序列化用户订单索引失败: {}", e),
            code: Some("SUBSCRIPTION_PAYMENT_INDEX_SERIALIZE_FAILED".to_string()),
            context: Some(order.user_id.clone()),
        })?;

    KeyValueConfig::set_value(
        &mut conn,
        &user_orders_key,
        &ids_payload,
        Some("用户订阅支付订单索引"),
    )
    .await
    .map_err(|e| ApplicationError::DatabaseError {
        message: format!("保存用户订单索引失败: {}", e),
        operation: Some("save user payment index".to_string()),
        table: Some("key_value_config".to_string()),
    })?;

    Ok(())
}

async fn read_payment_order(
    context: &Arc<ServiceContext>,
    order_id: &str,
) -> ApplicationResult<SubscriptionPaymentOrder> {
    let mut conn = context
        .rb
        .acquire()
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("数据库连接失败: {}", e),
            operation: Some("acquire connection".to_string()),
            table: None,
        })?;

    let raw = KeyValueConfig::get_value(&mut conn, &payment_order_key(order_id), "")
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("读取支付订单失败: {}", e),
            operation: Some("read payment order".to_string()),
            table: Some("key_value_config".to_string()),
        })?;

    if raw.trim().is_empty() {
        return Err(ApplicationError::BusinessError {
            message: "支付订单不存在".to_string(),
            code: Some("SUBSCRIPTION_PAYMENT_ORDER_NOT_FOUND".to_string()),
            context: Some(order_id.to_string()),
        });
    }

    serde_json::from_str::<SubscriptionPaymentOrder>(&raw).map_err(|e| {
        ApplicationError::BusinessError {
            message: format!("支付订单数据损坏: {}", e),
            code: Some("SUBSCRIPTION_PAYMENT_ORDER_INVALID".to_string()),
            context: Some(order_id.to_string()),
        }
    })
}

async fn is_callback_event_processed(
    context: &Arc<ServiceContext>,
    event_id: &str,
) -> ApplicationResult<bool> {
    let mut conn = context
        .rb
        .acquire()
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("数据库连接失败: {}", e),
            operation: Some("acquire connection".to_string()),
            table: None,
        })?;

    let raw = KeyValueConfig::get_value(&mut conn, &payment_callback_event_key(event_id), "")
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("读取回调事件幂等记录失败: {}", e),
            operation: Some("read callback event".to_string()),
            table: Some("key_value_config".to_string()),
        })?;

    Ok(!raw.trim().is_empty())
}

async fn mark_callback_event_processed(
    context: &Arc<ServiceContext>,
    event_id: &str,
    order_id: &str,
) -> ApplicationResult<()> {
    let mut conn = context
        .rb
        .acquire()
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("数据库连接失败: {}", e),
            operation: Some("acquire connection".to_string()),
            table: None,
        })?;

    let payload = serde_json::json!({
        "order_id": order_id,
        "processed_at": Utc::now().to_rfc3339(),
    })
    .to_string();

    KeyValueConfig::set_value(
        &mut conn,
        &payment_callback_event_key(event_id),
        &payload,
        Some("订阅支付回调幂等事件"),
    )
    .await
    .map_err(|e| ApplicationError::DatabaseError {
        message: format!("保存回调事件幂等记录失败: {}", e),
        operation: Some("save callback event".to_string()),
        table: Some("key_value_config".to_string()),
    })?;

    Ok(())
}

async fn resolve_payment_url(
    context: &Arc<ServiceContext>,
    provider: &str,
    order_id: &str,
) -> ApplicationResult<String> {
    let mut conn = context
        .rb
        .acquire()
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("数据库连接失败: {}", e),
            operation: Some("acquire connection".to_string()),
            table: None,
        })?;

    match provider {
        "stripe" => {
            let base = KeyValueConfig::get_value(
                &mut conn,
                SUBSCRIPTION_PAYMENT_STRIPE_CHECKOUT_BASE_URL_KEY,
                "",
            )
            .await
            .map_err(|e| ApplicationError::DatabaseError {
                message: format!("读取 Stripe Checkout 地址失败: {}", e),
                operation: Some("read stripe checkout base".to_string()),
                table: Some("key_value_config".to_string()),
            })?;

            if base.trim().is_empty() {
                return Err(ApplicationError::BusinessError {
                    message: "Stripe 支付未配置".to_string(),
                    code: Some("SUBSCRIPTION_PAYMENT_PROVIDER_NOT_CONFIGURED".to_string()),
                    context: Some("stripe".to_string()),
                });
            }

            Ok(format!(
                "{}/{}?client_reference_id={}",
                base.trim_end_matches('/'),
                order_id,
                order_id
            ))
        }
        "creem" => {
            let base = KeyValueConfig::get_value(
                &mut conn,
                SUBSCRIPTION_PAYMENT_CREEM_CHECKOUT_BASE_URL_KEY,
                "",
            )
            .await
            .map_err(|e| ApplicationError::DatabaseError {
                message: format!("读取 Creem Checkout 地址失败: {}", e),
                operation: Some("read creem checkout base".to_string()),
                table: Some("key_value_config".to_string()),
            })?;

            if base.trim().is_empty() {
                return Err(ApplicationError::BusinessError {
                    message: "Creem 支付未配置".to_string(),
                    code: Some("SUBSCRIPTION_PAYMENT_PROVIDER_NOT_CONFIGURED".to_string()),
                    context: Some("creem".to_string()),
                });
            }

            Ok(format!(
                "{}/{}?merchant_reference={}",
                base.trim_end_matches('/'),
                order_id,
                order_id
            ))
        }
        "epay" => {
            let base = KeyValueConfig::get_value(
                &mut conn,
                SUBSCRIPTION_PAYMENT_EPAY_CHECKOUT_BASE_URL_KEY,
                "",
            )
            .await
            .map_err(|e| ApplicationError::DatabaseError {
                message: format!("读取 ePay Checkout 地址失败: {}", e),
                operation: Some("read epay checkout base".to_string()),
                table: Some("key_value_config".to_string()),
            })?;

            if base.trim().is_empty() {
                return Err(ApplicationError::BusinessError {
                    message: "Epay 支付未配置".to_string(),
                    code: Some("SUBSCRIPTION_PAYMENT_PROVIDER_NOT_CONFIGURED".to_string()),
                    context: Some("epay".to_string()),
                });
            }

            Ok(format!(
                "{}/{}?out_trade_no={}",
                base.trim_end_matches('/'),
                order_id,
                order_id
            ))
        }
        "mock" => Ok(format!(
            "/payment/mock/checkout?order_id={}&provider={}",
            order_id, provider
        )),
        _ => Ok(format!(
            "/payment/mock/checkout?order_id={}&provider={}",
            order_id, provider
        )),
    }
}

async fn resolve_callback_secret(
    context: &Arc<ServiceContext>,
    provider: &str,
) -> ApplicationResult<String> {
    let mut conn = context
        .rb
        .acquire()
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("数据库连接失败: {}", e),
            operation: Some("acquire connection".to_string()),
            table: None,
        })?;

    let provider_secret_key = match provider {
        "stripe" => Some(SUBSCRIPTION_PAYMENT_STRIPE_CALLBACK_TOKEN_KEY),
        "creem" => Some(SUBSCRIPTION_PAYMENT_CREEM_CALLBACK_TOKEN_KEY),
        "epay" => Some(SUBSCRIPTION_PAYMENT_EPAY_CALLBACK_TOKEN_KEY),
        _ => None,
    };

    let provider_secret = if let Some(key) = provider_secret_key {
        KeyValueConfig::get_value(&mut conn, key, "")
            .await
            .map_err(|e| ApplicationError::DatabaseError {
                message: format!("读取支付回调密钥失败: {}", e),
                operation: Some("read provider callback token".to_string()),
                table: Some("key_value_config".to_string()),
            })?
    } else {
        String::new()
    };

    if !provider_secret.trim().is_empty() {
        return Ok(provider_secret);
    }

    KeyValueConfig::get_value(&mut conn, SUBSCRIPTION_PAYMENT_CALLBACK_TOKEN_KEY, "")
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("读取回调令牌失败: {}", e),
            operation: Some("read callback token".to_string()),
            table: Some("key_value_config".to_string()),
        })
}

async fn list_available_payment_providers(
    context: &Arc<ServiceContext>,
) -> ApplicationResult<SubscriptionPaymentProvidersResponse> {
    let mut conn = context
        .rb
        .acquire()
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("数据库连接失败: {}", e),
            operation: Some("acquire connection".to_string()),
            table: None,
        })?;

    let stripe_base = KeyValueConfig::get_value(
        &mut conn,
        SUBSCRIPTION_PAYMENT_STRIPE_CHECKOUT_BASE_URL_KEY,
        "",
    )
    .await
    .unwrap_or_default();
    let creem_base = KeyValueConfig::get_value(
        &mut conn,
        SUBSCRIPTION_PAYMENT_CREEM_CHECKOUT_BASE_URL_KEY,
        "",
    )
    .await
    .unwrap_or_default();
    let epay_base = KeyValueConfig::get_value(
        &mut conn,
        SUBSCRIPTION_PAYMENT_EPAY_CHECKOUT_BASE_URL_KEY,
        "",
    )
    .await
    .unwrap_or_default();

    let mut providers = vec![
        SubscriptionPaymentProviderItem {
            provider: "wallet".to_string(),
            display_name: "余额支付".to_string(),
            enabled: true,
        },
        SubscriptionPaymentProviderItem {
            provider: "stripe".to_string(),
            display_name: "Stripe".to_string(),
            enabled: !stripe_base.trim().is_empty(),
        },
        SubscriptionPaymentProviderItem {
            provider: "creem".to_string(),
            display_name: "Creem".to_string(),
            enabled: !creem_base.trim().is_empty(),
        },
        SubscriptionPaymentProviderItem {
            provider: "epay".to_string(),
            display_name: "Epay".to_string(),
            enabled: !epay_base.trim().is_empty(),
        },
        SubscriptionPaymentProviderItem {
            provider: "mock".to_string(),
            display_name: "Mock".to_string(),
            enabled: true,
        },
    ];

    if providers
        .iter()
        .filter(|item| item.provider != "mock")
        .all(|item| !item.enabled)
    {
        providers.push(SubscriptionPaymentProviderItem {
            provider: "mock".to_string(),
            display_name: "Mock".to_string(),
            enabled: true,
        });
    }

    let default_provider = providers
        .iter()
        .find(|item| item.enabled)
        .map(|item| item.provider.clone())
        .unwrap_or_else(|| "mock".to_string());

    Ok(SubscriptionPaymentProvidersResponse {
        providers,
        default_provider,
    })
}

fn default_subscription_plans() -> Vec<SubscriptionPlanItem> {
    vec![
        SubscriptionPlanItem {
            id: "starter".to_string(),
            name: "Starter".to_string(),
            description: "适合个人与小团队试用".to_string(),
            price: 9.9,
            billing_cycle: "monthly".to_string(),
            currency: "USD".to_string(),
            enabled: true,
            user_level: Some("L2".to_string()),
            quota_total: Some(1000.0),
        },
        SubscriptionPlanItem {
            id: "pro".to_string(),
            name: "Pro".to_string(),
            description: "适合生产环境与稳定业务".to_string(),
            price: 29.9,
            billing_cycle: "monthly".to_string(),
            currency: "USD".to_string(),
            enabled: true,
            user_level: Some("L3".to_string()),
            quota_total: Some(5000.0),
        },
        SubscriptionPlanItem {
            id: "team".to_string(),
            name: "Team".to_string(),
            description: "适合多成员协作与统一治理".to_string(),
            price: 99.0,
            billing_cycle: "monthly".to_string(),
            currency: "USD".to_string(),
            enabled: true,
            user_level: Some("L4".to_string()),
            quota_total: Some(20000.0),
        },
    ]
}

fn normalize_billing_cycle(raw: &str) -> String {
    raw.trim().to_lowercase()
}

fn is_supported_billing_cycle(raw: &str) -> bool {
    matches!(raw, "daily" | "weekly" | "monthly" | "yearly" | "annual")
}

fn normalize_subscription_plan_item(mut plan: SubscriptionPlanItem) -> SubscriptionPlanItem {
    plan.id = plan.id.trim().to_string();
    plan.name = plan.name.trim().to_string();
    plan.description = plan.description.trim().to_string();
    plan.billing_cycle = normalize_billing_cycle(&plan.billing_cycle);
    plan.currency = plan.currency.trim().to_uppercase();
    plan.user_level = plan
        .user_level
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    plan.quota_total = plan.quota_total.filter(|v| v.is_finite());
    plan
}

async fn load_all_subscription_plans(
    context: &Arc<ServiceContext>,
) -> ApplicationResult<Vec<SubscriptionPlanItem>> {
    let mut conn = context
        .rb
        .acquire()
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("数据库连接失败: {}", e),
            operation: Some("acquire connection".to_string()),
            table: None,
        })?;

    let default_json = serde_json::to_string(&default_subscription_plans()).unwrap_or_else(|_| {
        "[{\"id\":\"starter\",\"name\":\"Starter\",\"description\":\"适合个人与小团队试用\",\"price\":9.9,\"billing_cycle\":\"monthly\",\"currency\":\"USD\",\"enabled\":true}]".to_string()
    });

    let raw = KeyValueConfig::get_value(&mut conn, SUBSCRIPTION_PLANS_KEY, &default_json)
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("读取订阅方案失败: {}", e),
            operation: Some("read subscription plans".to_string()),
            table: Some("key_value_config".to_string()),
        })?;

    let mut plans: Vec<SubscriptionPlanItem> = serde_json::from_str(&raw)
        .unwrap_or_else(|_| default_subscription_plans())
        .into_iter()
        .map(normalize_subscription_plan_item)
        .collect();

    if plans.is_empty() {
        plans = default_subscription_plans();
    }

    Ok(plans)
}

async fn load_enabled_subscription_plans(
    context: &Arc<ServiceContext>,
) -> ApplicationResult<Vec<SubscriptionPlanItem>> {
    let plans = load_all_subscription_plans(context).await?;
    Ok(plans.into_iter().filter(|p| p.enabled).collect())
}

fn validate_subscription_plans(plans: &[SubscriptionPlanItem]) -> ApplicationResult<()> {
    if plans.is_empty() {
        return Err(ApplicationError::BusinessError {
            message: "订阅方案不能为空".to_string(),
            code: Some("SUBSCRIPTION_PLANS_EMPTY".to_string()),
            context: None,
        });
    }

    let mut seen_ids = std::collections::HashSet::<String>::new();

    for plan in plans {
        let id = plan.id.trim();
        let name = plan.name.trim();
        let currency = plan.currency.trim();
        let billing_cycle = normalize_billing_cycle(&plan.billing_cycle);

        if id.is_empty() || name.is_empty() {
            return Err(ApplicationError::BusinessError {
                message: "订阅方案的 id 和 name 不能为空".to_string(),
                code: Some("SUBSCRIPTION_PLAN_INVALID".to_string()),
                context: Some(plan.id.clone()),
            });
        }
        if !seen_ids.insert(id.to_string()) {
            return Err(ApplicationError::BusinessError {
                message: "订阅方案 id 不能重复".to_string(),
                code: Some("SUBSCRIPTION_PLAN_DUPLICATE_ID".to_string()),
                context: Some(id.to_string()),
            });
        }
        if currency.is_empty() {
            return Err(ApplicationError::BusinessError {
                message: "订阅方案币种不能为空".to_string(),
                code: Some("SUBSCRIPTION_PLAN_INVALID_CURRENCY".to_string()),
                context: Some(id.to_string()),
            });
        }
        if billing_cycle.is_empty() || !is_supported_billing_cycle(billing_cycle.as_str()) {
            return Err(ApplicationError::BusinessError {
                message: "订阅方案计费周期非法".to_string(),
                code: Some("SUBSCRIPTION_PLAN_INVALID_BILLING_CYCLE".to_string()),
                context: Some(format!("{}:{}", id, plan.billing_cycle)),
            });
        }
        if plan.price < 0.0 {
            return Err(ApplicationError::BusinessError {
                message: "订阅价格不能为负数".to_string(),
                code: Some("SUBSCRIPTION_PLAN_INVALID_PRICE".to_string()),
                context: Some(plan.id.clone()),
            });
        }
        if !plan.price.is_finite() {
            return Err(ApplicationError::BusinessError {
                message: "订阅价格非法".to_string(),
                code: Some("SUBSCRIPTION_PLAN_INVALID_PRICE".to_string()),
                context: Some(id.to_string()),
            });
        }
        if let Some(quota_total) = plan.quota_total {
            if !quota_total.is_finite() || quota_total < 0.0 {
                return Err(ApplicationError::BusinessError {
                    message: "订阅额度非法".to_string(),
                    code: Some("SUBSCRIPTION_PLAN_INVALID_QUOTA_TOTAL".to_string()),
                    context: Some(id.to_string()),
                });
            }
        }
    }

    Ok(())
}

pub async fn update_admin_subscription_plans(
    State(context): State<Arc<ServiceContext>>,
    _jwt_auth: JwtAuth,
    Json(req): Json<UpdateSubscriptionPlansRequest>,
) -> ApplicationResult<Json<ApiResponse<serde_json::Value>>> {
    let normalized_plans: Vec<SubscriptionPlanItem> = req
        .plans
        .into_iter()
        .map(normalize_subscription_plan_item)
        .collect();
    validate_subscription_plans(&normalized_plans)?;

    let payload =
        serde_json::to_string(&normalized_plans).map_err(|e| ApplicationError::BusinessError {
            message: format!("序列化订阅方案失败: {}", e),
            code: Some("SUBSCRIPTION_PLANS_SERIALIZE_FAILED".to_string()),
            context: None,
        })?;

    let mut conn = context
        .rb
        .acquire()
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("数据库连接失败: {}", e),
            operation: Some("acquire connection".to_string()),
            table: None,
        })?;

    KeyValueConfig::set_value(
        &mut conn,
        SUBSCRIPTION_PLANS_KEY,
        &payload,
        Some("订阅方案配置(JSON)"),
    )
    .await
    .map_err(|e| ApplicationError::DatabaseError {
        message: format!("保存订阅方案失败: {}", e),
        operation: Some("save subscription plans".to_string()),
        table: Some("key_value_config".to_string()),
    })?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "updated": normalized_plans.len()
    }))))
}

pub async fn get_admin_subscription_plans(
    State(context): State<Arc<ServiceContext>>,
    _jwt_auth: JwtAuth,
) -> ApplicationResult<Json<ApiResponse<UpdateSubscriptionPlansRequest>>> {
    let plans = load_all_subscription_plans(&context).await?;
    Ok(Json(ApiResponse::success(UpdateSubscriptionPlansRequest {
        plans,
    })))
}

pub async fn list_public_subscription_plans(
    State(context): State<Arc<ServiceContext>>,
) -> ApplicationResult<Json<ApiResponse<SubscriptionPlansResponse>>> {
    let plans = load_enabled_subscription_plans(&context).await?;
    Ok(Json(ApiResponse::success(SubscriptionPlansResponse {
        plans,
    })))
}

pub async fn list_subscription_plans(
    State(context): State<Arc<ServiceContext>>,
    _jwt_auth: JwtAuth,
) -> ApplicationResult<Json<ApiResponse<SubscriptionPlansResponse>>> {
    let plans = load_enabled_subscription_plans(&context).await?;
    Ok(Json(ApiResponse::success(SubscriptionPlansResponse {
        plans,
    })))
}

pub async fn list_subscription_payment_providers(
    State(context): State<Arc<ServiceContext>>,
    _jwt_auth: JwtAuth,
) -> ApplicationResult<Json<ApiResponse<SubscriptionPaymentProvidersResponse>>> {
    let response = list_available_payment_providers(&context).await?;
    Ok(Json(ApiResponse::success(response)))
}

pub async fn list_public_subscription_payment_providers(
    State(context): State<Arc<ServiceContext>>,
) -> ApplicationResult<Json<ApiResponse<SubscriptionPaymentProvidersResponse>>> {
    let response = list_available_payment_providers(&context).await?;
    Ok(Json(ApiResponse::success(response)))
}

pub async fn get_self_subscription(
    State(context): State<Arc<ServiceContext>>,
    jwt_auth: JwtAuth,
) -> ApplicationResult<Json<ApiResponse<SelfSubscriptionResponse>>> {
    let subscription = read_user_subscription_status(&context, &jwt_auth.id).await?;

    Ok(Json(ApiResponse::success(SelfSubscriptionResponse {
        subscription,
    })))
}

pub async fn create_subscription_payment_order(
    State(context): State<Arc<ServiceContext>>,
    jwt_auth: JwtAuth,
    Json(req): Json<CreateSubscriptionPaymentOrderRequest>,
) -> ApplicationResult<Json<ApiResponse<SubscriptionPaymentOrder>>> {
    let plans = load_enabled_subscription_plans(&context).await?;
    let plan = plans
        .iter()
        .find(|item| item.id == req.plan_id)
        .ok_or_else(|| ApplicationError::BusinessError {
            message: "订阅方案不存在".to_string(),
            code: Some("SUBSCRIPTION_PLAN_NOT_FOUND".to_string()),
            context: Some(req.plan_id.clone()),
        })?;

    let provider = normalize_provider(req.provider);

    // provider=wallet 表示纯余额购买（不创建第三方订单）
    if provider == "wallet" {
        let balance_service = BalanceService::new();
        let transaction_id = balance_service
            .deduct(DeductDTO {
                user_id: jwt_auth.id.clone(),
                amount: plan.price,
                reason: format!("订阅购买(余额):{}", plan.name),
            })
            .await?;

        let subscription_status = build_subscription_status(&jwt_auth.id, plan, &transaction_id);
        persist_subscription_status(&context, &jwt_auth.id, &subscription_status).await?;
        sync_user_level_with_subscription(&context, &jwt_auth.id, plan).await?;

        let now = Utc::now().to_rfc3339();
        let order = SubscriptionPaymentOrder {
            order_id: format!("subpay_{}", ulid::Ulid::new()),
            user_id: jwt_auth.id.clone(),
            plan_id: plan.id.clone(),
            plan_name: plan.name.clone(),
            provider,
            amount: plan.price,
            currency: plan.currency.clone(),
            status: "confirmed".to_string(),
            pay_url: String::new(),
            created_at: now.clone(),
            updated_at: now.clone(),
            confirmed_at: Some(now),
            external_txn_id: Some(transaction_id),
        };

        store_payment_order(&context, &order).await?;
        return Ok(Json(ApiResponse::success(order)));
    }

    if !matches!(provider.as_str(), "stripe" | "creem" | "epay" | "mock") {
        return Err(ApplicationError::BusinessError {
            message: "不支持的支付通道".to_string(),
            code: Some("SUBSCRIPTION_PAYMENT_PROVIDER_INVALID".to_string()),
            context: Some(provider),
        });
    }

    // 余额优先：先扣余额，余额不足时按开关允许补差创建第三方订单
    let wallet_first = req.wallet_first.unwrap_or(true);
    let mut remaining_amount = plan.price;
    let mut wallet_deduct_transaction_id: Option<String> = None;

    if wallet_first {
        let balance_service = BalanceService::new();
        let balance_vo = balance_service.get_balance(&jwt_auth.id).await?;
        let available_balance = balance_vo.balance.max(0.0);

        let desired_wallet_amount = req.wallet_amount.unwrap_or(available_balance).max(0.0);
        let wallet_amount = desired_wallet_amount
            .min(available_balance)
            .min(remaining_amount);

        if wallet_amount > 0.0 {
            let transaction_id = balance_service
                .deduct(DeductDTO {
                    user_id: jwt_auth.id.clone(),
                    amount: wallet_amount,
                    reason: format!("订阅购买(余额优先):{}", plan.name),
                })
                .await?;
            wallet_deduct_transaction_id = Some(transaction_id);
            remaining_amount = (remaining_amount - wallet_amount).max(0.0);
        }

        if remaining_amount <= 0.0 {
            let transaction_id = wallet_deduct_transaction_id.unwrap_or_else(|| {
                // 理论上不会走到这里（remaining_amount==0 且 wallet_amount==0），但给个兜底
                format!("subwallet_{}", ulid::Ulid::new())
            });

            let subscription_status =
                build_subscription_status(&jwt_auth.id, plan, &transaction_id);
            persist_subscription_status(&context, &jwt_auth.id, &subscription_status).await?;
            sync_user_level_with_subscription(&context, &jwt_auth.id, plan).await?;

            let now = Utc::now().to_rfc3339();
            let order = SubscriptionPaymentOrder {
                order_id: format!("subpay_{}", ulid::Ulid::new()),
                user_id: jwt_auth.id.clone(),
                plan_id: plan.id.clone(),
                plan_name: plan.name.clone(),
                provider: "wallet".to_string(),
                amount: plan.price,
                currency: plan.currency.clone(),
                status: "confirmed".to_string(),
                pay_url: String::new(),
                created_at: now.clone(),
                updated_at: now.clone(),
                confirmed_at: Some(now),
                external_txn_id: Some(transaction_id),
            };

            store_payment_order(&context, &order).await?;
            return Ok(Json(ApiResponse::success(order)));
        }

        // 需要补差：检查后端开关
        let mut conn = context
            .rb
            .acquire()
            .await
            .map_err(|e| ApplicationError::DatabaseError {
                message: format!("数据库连接失败: {}", e),
                operation: Some("acquire connection".to_string()),
                table: None,
            })?;
        let allow_combined =
            KeyValueConfig::get_value(&mut conn, SUBSCRIPTION_PAYMENT_ALLOW_COMBINED_KEY, "false")
                .await
                .unwrap_or_else(|_| "false".to_string());

        if allow_combined.trim() != "true" {
            return Err(ApplicationError::BusinessError {
                message: "余额不足，且系统未开启组合支付".to_string(),
                code: Some("SUBSCRIPTION_PAYMENT_COMBINED_DISABLED".to_string()),
                context: Some(jwt_auth.id.clone()),
            });
        }
    }

    let now = Utc::now().to_rfc3339();
    let order_id = format!("subpay_{}", ulid::Ulid::new());
    let pay_url = resolve_payment_url(&context, &provider, &order_id).await?;

    let mut order = SubscriptionPaymentOrder {
        order_id,
        user_id: jwt_auth.id.clone(),
        plan_id: plan.id.clone(),
        plan_name: plan.name.clone(),
        provider: provider.clone(),
        amount: remaining_amount,
        currency: plan.currency.clone(),
        status: "pending".to_string(),
        pay_url,
        created_at: now.clone(),
        updated_at: now,
        confirmed_at: None,
        external_txn_id: wallet_deduct_transaction_id,
    };

    if provider == "mock" {
        let confirmed_at = Utc::now().to_rfc3339();
        order.status = "confirmed".to_string();
        order.updated_at = confirmed_at.clone();
        order.confirmed_at = Some(confirmed_at.clone());
        order.external_txn_id = Some(order.order_id.clone());

        let subscription = build_subscription_status(&order.user_id, plan, &order.order_id);
        persist_subscription_status(&context, &order.user_id, &subscription).await?;
        sync_user_level_with_subscription(&context, &order.user_id, plan).await?;
    }

    store_payment_order(&context, &order).await?;
    Ok(Json(ApiResponse::success(order)))
}

pub async fn list_self_subscription_payment_orders(
    State(context): State<Arc<ServiceContext>>,
    jwt_auth: JwtAuth,
) -> ApplicationResult<Json<ApiResponse<SubscriptionPaymentOrdersResponse>>> {
    let mut conn = context
        .rb
        .acquire()
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("数据库连接失败: {}", e),
            operation: Some("acquire connection".to_string()),
            table: None,
        })?;

    let index_key = user_payment_orders_key(&jwt_auth.id);
    let ids_raw = KeyValueConfig::get_value(&mut conn, &index_key, "[]")
        .await
        .map_err(|e| ApplicationError::DatabaseError {
            message: format!("读取支付订单索引失败: {}", e),
            operation: Some("read payment order index".to_string()),
            table: Some("key_value_config".to_string()),
        })?;

    let order_ids: Vec<String> = serde_json::from_str(&ids_raw).unwrap_or_default();
    let mut orders: Vec<SubscriptionPaymentOrder> = Vec::new();
    for order_id in order_ids {
        if let Ok(order) = read_payment_order(&context, &order_id).await {
            orders.push(order);
        }
    }
    orders.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(Json(ApiResponse::success(
        SubscriptionPaymentOrdersResponse { orders },
    )))
}

pub async fn confirm_subscription_payment_callback(
    State(context): State<Arc<ServiceContext>>,
    Json(req): Json<ConfirmSubscriptionPaymentCallbackRequest>,
) -> ApplicationResult<Json<ApiResponse<ConfirmSubscriptionPaymentCallbackResponse>>> {
    let normalized = normalize_callback_status(&req.status)?;
    let mut order = read_payment_order(&context, &req.order_id).await?;
    let callback_provider = normalize_provider(req.provider.clone());
    if callback_provider != "mock" && callback_provider != order.provider {
        return Err(ApplicationError::BusinessError {
            message: "支付回调通道与订单不一致".to_string(),
            code: Some("SUBSCRIPTION_PAYMENT_PROVIDER_MISMATCH".to_string()),
            context: Some(order.order_id.clone()),
        });
    }

    let provider_for_event = if callback_provider == "mock" {
        order.provider.clone()
    } else {
        callback_provider
    };

    let event_id = req.event_id.clone().unwrap_or_else(|| {
        build_fallback_callback_event_id(
            &provider_for_event,
            &req.order_id,
            &normalized,
            req.external_txn_id.as_deref(),
        )
    });

    let callback_token = resolve_callback_secret(&context, &order.provider).await?;
    if callback_token.trim().is_empty() && order.provider != "mock" {
        return Err(ApplicationError::BusinessError {
            message: "支付回调密钥未配置".to_string(),
            code: Some("SUBSCRIPTION_PAYMENT_CALLBACK_SECRET_NOT_CONFIGURED".to_string()),
            context: Some(order.provider.clone()),
        });
    }

    verify_callback_signature(
        &callback_token,
        &order.provider,
        &req,
        &normalized,
        &event_id,
    )?;

    if is_callback_event_processed(&context, &event_id).await? {
        let subscription = read_user_subscription_status(&context, &order.user_id).await?;
        return Ok(Json(ApiResponse::success(
            ConfirmSubscriptionPaymentCallbackResponse {
                order,
                subscription,
            },
        )));
    }

    if !can_transition_payment_status(&order.status, &normalized) {
        mark_callback_event_processed(&context, &event_id, &order.order_id).await?;
        let subscription = read_user_subscription_status(&context, &order.user_id).await?;
        return Ok(Json(ApiResponse::success(
            ConfirmSubscriptionPaymentCallbackResponse {
                order,
                subscription,
            },
        )));
    }

    if normalized != "confirmed" {
        order.status = normalized;
        order.updated_at = Utc::now().to_rfc3339();
        store_payment_order(&context, &order).await?;
        mark_callback_event_processed(&context, &event_id, &order.order_id).await?;
        return Ok(Json(ApiResponse::success(
            ConfirmSubscriptionPaymentCallbackResponse {
                order,
                subscription: None,
            },
        )));
    }

    if order.status != "confirmed" {
        let plans = load_all_subscription_plans(&context).await?;
        let plan = plans
            .iter()
            .find(|item| item.id == order.plan_id)
            .ok_or_else(|| ApplicationError::BusinessError {
                message: "订阅方案不存在".to_string(),
                code: Some("SUBSCRIPTION_PLAN_NOT_FOUND".to_string()),
                context: Some(order.plan_id.clone()),
            })?;
        let now = Utc::now().to_rfc3339();
        order.status = "confirmed".to_string();
        order.updated_at = now.clone();
        order.confirmed_at = Some(now);
        order.external_txn_id = req.external_txn_id.clone();

        let subscription = build_subscription_status(
            &order.user_id,
            plan,
            order
                .external_txn_id
                .as_deref()
                .unwrap_or(order.order_id.as_str()),
        );
        persist_subscription_status(&context, &order.user_id, &subscription).await?;
        sync_user_level_with_subscription(&context, &order.user_id, plan).await?;
        store_payment_order(&context, &order).await?;
        mark_callback_event_processed(&context, &event_id, &order.order_id).await?;

        return Ok(Json(ApiResponse::success(
            ConfirmSubscriptionPaymentCallbackResponse {
                order,
                subscription: Some(subscription),
            },
        )));
    }

    if let (Some(existing), Some(incoming)) = (
        order.external_txn_id.as_deref(),
        req.external_txn_id.as_deref(),
    ) {
        if existing != incoming {
            return Err(ApplicationError::BusinessError {
                message: "支付订单已确认且外部交易号不一致".to_string(),
                code: Some("SUBSCRIPTION_PAYMENT_ORDER_CONFLICT".to_string()),
                context: Some(order.order_id.clone()),
            });
        }
    }

    mark_callback_event_processed(&context, &event_id, &order.order_id).await?;

    let subscription = read_user_subscription_status(&context, &order.user_id).await?;

    Ok(Json(ApiResponse::success(
        ConfirmSubscriptionPaymentCallbackResponse {
            order,
            subscription,
        },
    )))
}

pub async fn purchase_subscription(
    State(context): State<Arc<ServiceContext>>,
    jwt_auth: JwtAuth,
    Json(req): Json<PurchaseSubscriptionRequest>,
) -> ApplicationResult<Json<ApiResponse<PurchaseSubscriptionResponse>>> {
    let plans = load_enabled_subscription_plans(&context).await?;
    let plan = plans
        .iter()
        .find(|item| item.id == req.plan_id)
        .ok_or_else(|| ApplicationError::BusinessError {
            message: "订阅方案不存在".to_string(),
            code: Some("SUBSCRIPTION_PLAN_NOT_FOUND".to_string()),
            context: Some(req.plan_id.clone()),
        })?;

    let transaction_service = TransactionService {};
    let transaction_id = transaction_service
        .create_transaction(
            CreateTransactionDTO {
                user_id: jwt_auth.id.clone(),
                transaction_type: "deduct".to_string(),
                amount: plan.price,
                reason: format!("订阅购买:{}", plan.name),
            },
            Some(jwt_auth.id.clone()),
        )
        .await?;

    let balance_service = BalanceService::new();
    let balance_vo = balance_service.get_balance(&jwt_auth.id).await?;

    let subscription_status = build_subscription_status(&jwt_auth.id, plan, &transaction_id);
    persist_subscription_status(&context, &jwt_auth.id, &subscription_status).await?;
    sync_user_level_with_subscription(&context, &jwt_auth.id, plan).await?;

    Ok(Json(ApiResponse::success(PurchaseSubscriptionResponse {
        transaction_id,
        plan_id: plan.id.clone(),
        amount: plan.price,
        currency: plan.currency.clone(),
        balance_after: balance_vo.balance,
        subscription: subscription_status,
    })))
}
