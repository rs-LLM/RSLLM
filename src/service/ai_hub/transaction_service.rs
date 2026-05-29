// 用途：导入ULID生成器
// 说明：用于生成唯一ID
use chrono::{DateTime as ChronoDateTime, Duration, NaiveDateTime, Utc};
use ulid::Ulid;
// 用途：导入日期时间类型
// 说明：用于记录时间戳
use rbatis::rbdc::DateTime;
// 用途：导入应用错误类型
// 说明：用于错误处理
use crate::error::ApplicationError;
// 用途：导入应用结果类型
// 说明：用于统一返回结果
use crate::error::ApplicationResult;
// 用途：导入交易表
// 说明：用于数据库操作
use crate::domain::table::ai_hub::Transaction;
use crate::domain::table::basic::KeyValueConfig;
// 用途：导入用户表
// 说明：用于查询用户信息
use crate::domain::table::basic::SysUser;
// 用途：导入交易DTO
// 说明：用于接收请求数据
use crate::domain::dto::ai_hub::{
    CreateTransactionDTO, DeductDTO, QueryTransactionDTO, RechargeDTO, SetBalanceDTO,
    UserSubscriptionStatus,
};
// 用途：导入交易VO
// 说明：用于返回响应数据
use crate::domain::vo::ai_hub::{TransactionCycleSummaryVO, TransactionVO};
// 用途：导入数据库连接池
// 说明：用于获取数据库连接
use crate::pool;

/// 交易管理服务
///
/// 负责用户余额交易记录的管理，包括充值、扣减、设置余额等操作
#[derive(Clone)]
pub struct TransactionService {}

impl TransactionService {
    fn parse_to_utc(input: &str) -> Option<ChronoDateTime<Utc>> {
        if let Ok(dt) = ChronoDateTime::parse_from_rfc3339(input) {
            return Some(dt.with_timezone(&Utc));
        }
        if let Ok(dt) = NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M:%S") {
            return Some(ChronoDateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
        }
        None
    }

    /// 创建交易记录（带事务）
    pub async fn create_transaction_with_tx(
        &self,
        tx: &mut dyn rbatis::executor::Executor,
        dto: CreateTransactionDTO,
        operator_id: Option<String>,
    ) -> ApplicationResult<(String, f64)> {
        let id = Ulid::new().to_string();
        let now = DateTime::now();

        let users = SysUser::select_by_map(tx, rbs::value! { "id": &dto.user_id }).await?;
        if users.is_empty() {
            return Err(ApplicationError::NotFound {
                message: "User not found".to_string(),
                resource: Some("user".to_string()),
                id: Some(dto.user_id.clone()),
            });
        }

        let mut user = users[0].clone();
        let balance_before = user.balance.unwrap_or(0.0);
        let balance_after = match dto.transaction_type.as_str() {
            "recharge" => balance_before + dto.amount,
            "deduct" => {
                if balance_before < dto.amount {
                    return Err(ApplicationError::ValidationError {
                        message: "Insufficient balance".to_string(),
                        field: Some("balance".to_string()),
                        value: Some(balance_before.to_string()),
                    });
                }
                balance_before - dto.amount
            }
            "set" => dto.amount,
            "redeem" => balance_before + dto.amount,
            _ => {
                return Err(ApplicationError::ValidationError {
                    message: "Invalid transaction type".to_string(),
                    field: Some("transaction_type".to_string()),
                    value: Some(dto.transaction_type.clone()),
                });
            }
        };

        user.balance = Some(balance_after);
        SysUser::update_by_map(tx, &user, rbs::value! { "id": &dto.user_id }).await?;

        let transaction = Transaction {
            id: Some(id.clone()),
            user_id: dto.user_id.clone(),
            type_: dto.transaction_type.clone(),
            amount: dto.amount,
            balance_before,
            balance_after,
            operator_id,
            reason: dto.reason,
            created_at: Some(now),
        };

        Transaction::insert(tx, &transaction).await?;

        Ok((id, balance_after))
    }

    /// 创建交易记录
    pub async fn create_transaction(
        &self,
        dto: CreateTransactionDTO,
        operator_id: Option<String>,
    ) -> ApplicationResult<String> {
        let mut tx = pool!().acquire_begin().await?;
        let result = self
            .create_transaction_with_tx(&mut tx, dto, operator_id)
            .await;

        match result {
            Ok((id, _balance_after)) => {
                tx.commit().await?;
                Ok(id)
            }
            Err(e) => {
                tx.rollback().await?;
                Err(e)
            }
        }
    }

    /// 充值
    pub async fn recharge(
        &self,
        dto: RechargeDTO,
        operator_id: Option<String>,
    ) -> ApplicationResult<String> {
        let create_dto = CreateTransactionDTO {
            user_id: dto.user_id,
            transaction_type: "recharge".to_string(),
            amount: dto.amount,
            reason: dto.reason,
        };

        self.create_transaction(create_dto, operator_id).await
    }

    /// 扣减
    pub async fn deduct(&self, dto: DeductDTO) -> ApplicationResult<String> {
        let create_dto = CreateTransactionDTO {
            user_id: dto.user_id,
            transaction_type: "deduct".to_string(),
            amount: dto.amount,
            reason: dto.reason,
        };

        self.create_transaction(create_dto, None).await
    }

    /// 设置余额
    pub async fn set_balance(
        &self,
        dto: SetBalanceDTO,
        operator_id: Option<String>,
    ) -> ApplicationResult<String> {
        let create_dto = CreateTransactionDTO {
            user_id: dto.user_id,
            transaction_type: "set".to_string(),
            amount: dto.amount,
            reason: dto.reason,
        };

        self.create_transaction(create_dto, operator_id).await
    }

    /// 查询交易记录列表
    pub async fn list_transaction(
        &self,
        dto: QueryTransactionDTO,
        current_user_id: String,
        has_manage_balance: bool,
    ) -> ApplicationResult<(Vec<TransactionVO>, usize)> {
        let mut conditions = rbs::value!({});

        if !has_manage_balance {
            conditions["userId"] = rbs::value!(current_user_id);
        } else if let Some(user_id) = dto.user_id {
            conditions["userId"] = rbs::value!(user_id);
        }

        if let Some(transaction_type) = dto.transaction_type {
            conditions["type"] = rbs::value!(transaction_type);
        }

        let page = dto.page.unwrap_or(1);
        let page_size = dto.page_size.unwrap_or(20);
        let offset = (page - 1) * page_size;

        let mut transactions = Transaction::select_by_map(pool!(), conditions.clone()).await?;
        let total = transactions.len();

        transactions.sort_by(|a, b| {
            let a_time = a.created_at.clone().unwrap_or(DateTime::now());
            let b_time = b.created_at.clone().unwrap_or(DateTime::now());
            b_time.cmp(&a_time)
        });

        let paginated_transactions = transactions
            .into_iter()
            .skip(offset as usize)
            .take(page_size as usize)
            .collect::<Vec<_>>();

        let mut result = Vec::new();
        for transaction in paginated_transactions {
            let username = if let Ok(users) =
                SysUser::select_by_map(pool!(), rbs::value! { "id": &transaction.user_id }).await
            {
                if !users.is_empty() {
                    users[0].account.clone()
                } else {
                    None
                }
            } else {
                None
            };

            let operator_name = if let Some(operator_id) = &transaction.operator_id {
                if let Ok(users) =
                    SysUser::select_by_map(pool!(), rbs::value! { "id": operator_id }).await
                {
                    if !users.is_empty() {
                        users[0].account.clone()
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                if transaction.type_ == "redeem" || transaction.reason.contains("兑换码") {
                    Some("兑换码服务".to_string())
                } else if transaction.reason.starts_with("AI服务调用") {
                    Some("AI服务".to_string())
                } else {
                    None
                }
            };

            let transaction_type_name = match transaction.type_.as_str() {
                "recharge" => "充值".to_string(),
                "deduct" => "扣减".to_string(),
                "set" => "设置".to_string(),
                "redeem" => "兑换".to_string(),
                _ => transaction.type_.clone(),
            };

            result.push(TransactionVO {
                id: transaction.id.clone(),
                user_id: transaction.user_id.clone(),
                username,
                transaction_type: transaction.type_.clone(),
                transaction_type_name,
                amount: transaction.amount,
                balance_before: transaction.balance_before,
                balance_after: transaction.balance_after,
                operator_id: transaction.operator_id.clone(),
                operator_name,
                reason: transaction.reason.clone(),
                created_at: transaction.created_at.as_ref().map(|dt| dt.to_string()),
            });
        }

        Ok((result, total))
    }

    /// 查询用户余额
    pub async fn get_balance(&self, user_id: &str) -> ApplicationResult<f64> {
        let users = SysUser::select_by_map(pool!(), rbs::value! { "id": user_id }).await?;

        if users.is_empty() {
            return Err(ApplicationError::NotFound {
                message: "User not found".to_string(),
                resource: Some("user".to_string()),
                id: Some(user_id.to_string()),
            });
        }

        Ok(users[0].balance.unwrap_or(0.0))
    }

    pub async fn summarize_cycle_deduct(
        &self,
        current_user_id: String,
    ) -> ApplicationResult<TransactionCycleSummaryVO> {
        let user_id = current_user_id;

        let mut conn = pool!().acquire().await?;
        let subscription_key = format!("user.subscription.{user_id}");
        let subscription_status =
            match KeyValueConfig::get_value(&mut conn, &subscription_key, "").await {
                Ok(raw) => serde_json::from_str::<UserSubscriptionStatus>(&raw).ok(),
                Err(_) => None,
            };

        let cycle_end = subscription_status
            .as_ref()
            .and_then(|status| status.next_quota_reset_at.clone());
        let end_time = cycle_end
            .as_deref()
            .and_then(TransactionService::parse_to_utc);
        let start_time = match (subscription_status.as_ref(), end_time.as_ref()) {
            (Some(status), Some(end)) => {
                let end = end.to_owned();
                let start = match status.quota_reset_cycle.as_str() {
                    "daily" => end - Duration::days(1),
                    "weekly" => end - Duration::weeks(1),
                    "monthly" => end - Duration::days(30),
                    "yearly" => end - Duration::days(365),
                    _ => end - Duration::days(30),
                };
                Some(start)
            }
            _ => None,
        };
        let cycle_start = start_time
            .as_ref()
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string());

        let mut conditions = rbs::value!({});
        conditions["userId"] = rbs::value!(user_id);
        conditions["type"] = rbs::value!("deduct");

        let transactions = Transaction::select_by_map(pool!(), conditions).await?;

        let mut deduct_count = 0usize;
        let mut wallet_deduct_amount = 0.0f64;
        let mut subscription_deduct_amount = 0.0f64;

        for transaction in transactions {
            let transaction_time = transaction
                .created_at
                .as_ref()
                .and_then(|time| TransactionService::parse_to_utc(&time.to_string()));

            if let Some(start) = start_time.as_ref() {
                match transaction_time.as_ref() {
                    Some(time) if time < start => continue,
                    None => continue,
                    _ => {}
                }
            }

            if let Some(end) = end_time.as_ref() {
                match transaction_time.as_ref() {
                    Some(time) if time >= end => continue,
                    None => continue,
                    _ => {}
                }
            }

            deduct_count += 1;
            if transaction.reason.contains("(订阅额度)") {
                subscription_deduct_amount += transaction.amount;
            } else {
                wallet_deduct_amount += transaction.amount;
            }
        }

        let total_deduct_amount = wallet_deduct_amount + subscription_deduct_amount;
        let wallet_deduct_ratio = if total_deduct_amount > 0.0 {
            (wallet_deduct_amount / total_deduct_amount) * 100.0
        } else {
            0.0
        };

        Ok(TransactionCycleSummaryVO {
            cycle_start,
            cycle_end,
            deduct_count,
            wallet_deduct_amount,
            subscription_deduct_amount,
            total_deduct_amount,
            wallet_deduct_ratio,
        })
    }
}
