use std::collections::{HashMap, HashSet};

use rand::Rng;
use rbatis::RBatis;
use rbatis::rbdc::DateTime;
use ulid::Ulid;

use crate::domain::dto::ai_hub::redeem_code::{
    GenerateRedeemCodesDTO, QueryRedeemCodesDTO, RedeemCodeExchangeDTO, UpdateRedeemCodeStatusDTO,
};
use crate::domain::dto::ai_hub::transaction::CreateTransactionDTO;
use crate::domain::table::ai_hub::redeem_code::RedeemCode;
use crate::domain::table::ai_hub::redeem_code_usage::RedeemCodeUsage;
use crate::domain::table::basic::sys_user::SysUser;
use crate::error::{ApplicationError, ApplicationResult};
use crate::service::ai_hub::TransactionService;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RedeemCodeListResult {
    pub total: usize,
    pub items: Vec<RedeemCodeListItem>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RedeemCodeListItem {
    #[serde(flatten)]
    pub code: RedeemCode,
    pub used_usernames: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RedeemResult {
    pub amount: f64,
    #[serde(rename = "balanceAfter")]
    pub balance_after: f64,
}

pub struct RedeemCodeService;

impl RedeemCodeService {
    fn generate_code() -> String {
        const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        let mut rng = rand::rng();
        let mut code = String::with_capacity(12);
        for _ in 0..12 {
            let idx = rng.random_range(0..CHARS.len());
            code.push(CHARS[idx] as char);
        }
        code
    }

    pub async fn generate_codes(
        rb: &RBatis,
        dto: GenerateRedeemCodesDTO,
        operator_id: Option<String>,
    ) -> ApplicationResult<Vec<RedeemCode>> {
        if dto.amount <= 0.0 {
            return Err(ApplicationError::ValidationError {
                message: "兑换码金额必须大于0".to_string(),
                field: Some("amount".to_string()),
                value: Some(dto.amount.to_string()),
            });
        }

        let quantity = dto.quantity.unwrap_or(1).clamp(1, 200) as usize;
        let max_uses = dto.max_uses.unwrap_or(1).max(1);
        let now = DateTime::now();

        let mut created = Vec::with_capacity(quantity);
        let mut generated_codes = HashSet::new();

        while created.len() < quantity {
            let code = Self::generate_code();
            if generated_codes.contains(&code) {
                continue;
            }

            let exists =
                RedeemCode::select_by_map(rb, rbs::value! { "code": code.clone() }).await?;
            if !exists.is_empty() {
                continue;
            }

            generated_codes.insert(code.clone());
            let code_item = RedeemCode {
                id: Some(Ulid::new().to_string()),
                code,
                amount: dto.amount,
                status: "active".to_string(),
                max_uses,
                used_count: 0,
                used_by: None,
                used_at: None,
                operator_id: operator_id.clone(),
                remark: dto.remark.clone(),
                created_at: Some(now.clone()),
                updated_at: Some(now.clone()),
            };
            RedeemCode::insert(rb, &code_item).await?;
            created.push(code_item);
        }

        Ok(created)
    }

    pub async fn list_codes(
        rb: &RBatis,
        dto: QueryRedeemCodesDTO,
    ) -> ApplicationResult<RedeemCodeListResult> {
        let all = RedeemCode::select_all(rb).await?;
        let code_filter = dto.code.unwrap_or_default().trim().to_string();
        let status_filter = dto.status.unwrap_or_default().trim().to_lowercase();

        let mut items: Vec<RedeemCode> = all
            .into_iter()
            .filter(|item| {
                let mut ok = true;
                if !code_filter.is_empty() {
                    ok = ok
                        && item
                            .code
                            .to_uppercase()
                            .contains(&code_filter.to_uppercase());
                }
                if !status_filter.is_empty() {
                    ok = ok && item.status.to_lowercase() == status_filter;
                }
                ok
            })
            .collect();

        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let page = dto.page.unwrap_or(1).max(1) as usize;
        let page_size = dto.page_size.unwrap_or(20).clamp(1, 200) as usize;
        let start = (page - 1) * page_size;
        let end = std::cmp::min(start + page_size, items.len());

        let page_codes = if start >= items.len() {
            Vec::new()
        } else {
            items[start..end].to_vec()
        };

        let usages = RedeemCodeUsage::select_all(rb).await?;
        let mut username_map: HashMap<String, Vec<String>> = HashMap::new();
        for usage in usages {
            username_map
                .entry(usage.code_id)
                .or_default()
                .push(usage.username);
        }

        let paged_items = page_codes
            .into_iter()
            .map(|code| {
                let mut used_usernames = code
                    .id
                    .as_ref()
                    .and_then(|id| username_map.get(id))
                    .cloned()
                    .unwrap_or_default();
                used_usernames.sort();
                used_usernames.dedup();
                RedeemCodeListItem {
                    code,
                    used_usernames,
                }
            })
            .collect();

        Ok(RedeemCodeListResult {
            total: items.len(),
            items: paged_items,
        })
    }

    pub async fn update_status(
        rb: &RBatis,
        dto: UpdateRedeemCodeStatusDTO,
        operator_id: Option<String>,
    ) -> ApplicationResult<RedeemCode> {
        let status = dto.status.trim().to_lowercase();
        if status != "active" && status != "disabled" && status != "used" {
            return Err(ApplicationError::ValidationError {
                message: "兑换码状态仅支持 active/disabled/used".to_string(),
                field: Some("status".to_string()),
                value: Some(dto.status),
            });
        }

        let mut list = RedeemCode::select_by_map(rb, rbs::value! { "id": dto.id.clone() }).await?;
        let mut code = list
            .drain(..)
            .next()
            .ok_or_else(|| ApplicationError::NotFound {
                message: "兑换码不存在".to_string(),
                resource: Some("redeem_code".to_string()),
                id: Some(dto.id.clone()),
            })?;

        code.status = status;
        code.remark = dto.remark;
        code.operator_id = operator_id;
        code.updated_at = Some(DateTime::now());

        RedeemCode::update_by_map(rb, &code, rbs::value! { "id": dto.id }).await?;
        Ok(code)
    }

    pub async fn redeem(
        rb: &RBatis,
        dto: RedeemCodeExchangeDTO,
        user_id: String,
    ) -> ApplicationResult<RedeemResult> {
        let input_code = dto.code.trim().to_uppercase();
        if input_code.is_empty() {
            return Err(ApplicationError::ValidationError {
                message: "请输入兑换码".to_string(),
                field: Some("code".to_string()),
                value: None,
            });
        }

        let mut tx = rb.acquire_begin().await?;

        // 1) 查询兑换码
        let mut list =
            RedeemCode::select_by_map(&tx, rbs::value! { "code": input_code.clone() }).await?;
        let code = list
            .drain(..)
            .next()
            .ok_or_else(|| ApplicationError::BusinessError {
                message: "兑换码不存在".to_string(),
                code: Some("REDEEM_CODE_NOT_FOUND".to_string()),
                context: Some("redeem_code.redeem".to_string()),
            })?;

        let code_id = code
            .id
            .clone()
            .ok_or_else(|| ApplicationError::BusinessError {
                message: "兑换码ID缺失".to_string(),
                code: Some("REDEEM_CODE_ID_MISSING".to_string()),
                context: Some("redeem_code.redeem".to_string()),
            })?;

        // 幂等优先：同一用户同一码重复兑换，稳定返回 ALREADY_USED
        // 注意：即便兑换码已被标记为 used/disabled，只要该用户已使用过，也应返回 ALREADY_USED
        let existing_usage = RedeemCodeUsage::select_by_map(
            &tx,
            rbs::value! { "code_id": code_id.clone(), "user_id": user_id.clone() },
        )
        .await?;
        if !existing_usage.is_empty() {
            tx.rollback().await?;
            return Err(ApplicationError::BusinessError {
                message: "该兑换码你已使用过".to_string(),
                code: Some("REDEEM_CODE_ALREADY_USED".to_string()),
                context: Some("redeem_code.redeem".to_string()),
            });
        }

        if code.status != "active" {
            tx.rollback().await?;
            return Err(ApplicationError::BusinessError {
                message: "兑换码不可用".to_string(),
                code: Some("REDEEM_CODE_NOT_AVAILABLE".to_string()),
                context: Some("redeem_code.redeem".to_string()),
            });
        }

        // 2) 先插入使用记录（唯一索引兜底幂等）
        // 目的：确保同一用户同一码的重复兑换，稳定返回 ALREADY_USED（而不是先消耗名额/命中用尽）
        let now = DateTime::now();
        let username = SysUser::select_by_map(&tx, rbs::value! { "id": user_id.clone() })
            .await?
            .into_iter()
            .next()
            .map(|user| {
                if let Some(account) = user.account.filter(|v| !v.trim().is_empty()) {
                    account
                } else if let Some(name) = user.name.filter(|n| !n.trim().is_empty()) {
                    name
                } else if let Some(email) = user.email.filter(|e| !e.trim().is_empty()) {
                    email
                } else {
                    user_id.clone()
                }
            })
            .unwrap_or_else(|| user_id.clone());

        let usage = RedeemCodeUsage {
            id: Some(Ulid::new().to_string()),
            code_id: code_id.clone(),
            user_id: user_id.clone(),
            username,
            created_at: Some(now.clone()),
        };

        if let Err(error) = RedeemCodeUsage::insert(&tx, &usage).await {
            let lower_message = error.to_string().to_lowercase();
            if lower_message.contains("unique") || lower_message.contains("duplicate") {
                tx.rollback().await?;
                return Err(ApplicationError::BusinessError {
                    message: "该兑换码你已使用过".to_string(),
                    code: Some("REDEEM_CODE_ALREADY_USED".to_string()),
                    context: Some("redeem_code.redeem".to_string()),
                });
            }
            tx.rollback().await?;
            return Err(error.into());
        }

        // 3) 原子占用名额（并发安全）
        let affected = tx
            .exec(
                "UPDATE redeem_code SET usedCount = usedCount + 1, updatedAt = ? WHERE id = ? AND status = 'active' AND usedCount < maxUses",
                vec![rbs::Value::from(now.clone()), rbs::Value::from(code_id.clone())],
            )
            .await?;

        // rbatis 不同驱动返回值不一，这里用 rows_affected 兜底
        let rows = affected.rows_affected;
        if rows == 0 {
            tx.rollback().await?;
            return Err(ApplicationError::BusinessError {
                message: "兑换码已用尽".to_string(),
                code: Some("REDEEM_CODE_EXHAUSTED".to_string()),
                context: Some("redeem_code.redeem".to_string()),
            });
        }

        // 4) 交易与余额变更（同一事务）
        let transaction_service = TransactionService {};
        let (_transaction_id, balance_after) = transaction_service
            .create_transaction_with_tx(
                &mut tx,
                CreateTransactionDTO {
                    user_id: user_id.clone(),
                    transaction_type: "redeem".to_string(),
                    amount: code.amount,
                    reason: format!("兑换码兑换: code={}", code.code),
                },
                None,
            )
            .await?;

        // 5) 如达到上限，顺便置为 used
        let _ = tx
            .exec(
                "UPDATE redeem_code SET status = 'used', updatedAt = ? WHERE id = ? AND usedCount >= maxUses",
                vec![rbs::Value::from(DateTime::now()), rbs::Value::from(code_id.clone())],
            )
            .await;

        tx.commit().await?;

        Ok(RedeemResult {
            amount: code.amount,
            balance_after,
        })
    }
}
