// 用途：导入ULID生成器
// 说明：用于生成唯一ID
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
// 用途：导入用户表
// 说明：用于查询用户信息
use crate::domain::table::basic::SysUser;
// 用途：导入交易DTO
// 说明：用于接收请求数据
use crate::domain::dto::ai_hub::{
    CreateTransactionDTO, DeductDTO, QueryTransactionDTO, RechargeDTO, SetBalanceDTO,
};
// 用途：导入交易VO
// 说明：用于返回响应数据
use crate::domain::vo::ai_hub::TransactionVO;
// 用途：导入数据库连接池
// 说明：用于获取数据库连接
use crate::pool;

/// 交易管理服务
///
/// 负责用户余额交易记录的管理，包括充值、扣减、设置余额等操作
#[derive(Clone)]
pub struct TransactionService {}

impl TransactionService {
    /// 创建交易记录
    pub async fn create_transaction(
        &self,
        dto: CreateTransactionDTO,
        operator_id: Option<String>,
    ) -> ApplicationResult<String> {
        let id = Ulid::new().to_string();
        let now = DateTime::now();

        let tx = pool!().acquire_begin().await?;

        let users = SysUser::select_by_map(&tx, rbs::value! { "id": &dto.user_id }).await?;
        if users.is_empty() {
            tx.rollback().await?;
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
                    tx.rollback().await?;
                    return Err(ApplicationError::ValidationError {
                        message: "Insufficient balance".to_string(),
                        field: Some("balance".to_string()),
                        value: Some(balance_before.to_string()),
                    });
                }
                balance_before - dto.amount
            }
            "set" => dto.amount,
            _ => {
                tx.rollback().await?;
                return Err(ApplicationError::ValidationError {
                    message: "Invalid transaction type".to_string(),
                    field: Some("transaction_type".to_string()),
                    value: Some(dto.transaction_type.clone()),
                });
            }
        };

        user.balance = Some(balance_after);
        SysUser::update_by_map(&tx, &user, rbs::value! { "id": &dto.user_id }).await?;

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

        Transaction::insert(&tx, &transaction).await?;

        tx.commit().await?;

        Ok(id)
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
    ) -> ApplicationResult<Vec<TransactionVO>> {
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

        transactions = transactions
            .into_iter()
            .skip(offset as usize)
            .take(page_size as usize)
            .collect();

        let mut result = Vec::new();
        for transaction in transactions {
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
                None
            };

            let transaction_type_name = match transaction.type_.as_str() {
                "recharge" => "充值".to_string(),
                "deduct" => "扣减".to_string(),
                "set" => "设置".to_string(),
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

        Ok(result)
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
}
