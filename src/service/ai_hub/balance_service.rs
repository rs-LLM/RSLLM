// 用途：导入应用结果类型
// 说明：用于统一返回结果
use crate::error::ApplicationResult;
// 用途：导入用户表
// 说明：用于查询用户信息
use crate::domain::table::basic::SysUser;
// 用途：导入余额VO
// 说明：用于返回响应数据
use crate::domain::vo::ai_hub::BalanceVO;
// 用途：导入交易服务
// 说明：用于处理余额变更操作
use super::transaction_service::TransactionService;
// 用途：导入交易DTO
// 说明：用于创建交易记录
use crate::domain::dto::ai_hub::{RechargeDTO, DeductDTO, SetBalanceDTO};
// 用途：导入数据库连接池
// 说明：用于获取数据库连接
use crate::pool;

/// 余额管理服务
///
/// 负责用户余额的查询和管理操作
#[derive(Clone)]
pub struct BalanceService {
    transaction_service: TransactionService,
}

impl BalanceService {
    pub fn new() -> Self {
        Self {
            transaction_service: TransactionService {},
        }
    }

    /// 查询用户余额
    pub async fn get_balance(&self, user_id: &str) -> ApplicationResult<BalanceVO> {
        let users = SysUser::select_by_map(pool!(), rbs::value! { "id": user_id }).await?;
        
        if users.is_empty() {
            return Err(crate::error::ApplicationError::NotFound {
                message: "User not found".to_string(),
                resource: Some("user".to_string()),
                id: Some(user_id.to_string()),
            });
        }
        
        let user = &users[0];
        
        Ok(BalanceVO {
            user_id: user.id.clone().unwrap_or_default(),
            username: user.account.clone().unwrap_or_default(),
            balance: user.balance.unwrap_or(0.0),
        })
    }

    /// 充值
    pub async fn recharge(&self, dto: RechargeDTO, operator_id: Option<String>) -> ApplicationResult<String> {
        self.transaction_service.recharge(dto, operator_id).await
    }

    /// 扣减
    pub async fn deduct(&self, dto: DeductDTO) -> ApplicationResult<String> {
        self.transaction_service.deduct(dto).await
    }

    /// 设置余额
    pub async fn set_balance(&self, dto: SetBalanceDTO, operator_id: Option<String>) -> ApplicationResult<String> {
        self.transaction_service.set_balance(dto, operator_id).await
    }

    /// 检查余额是否足够
    pub async fn check_balance(&self, user_id: &str, amount: f64) -> ApplicationResult<bool> {
        let balance = self.get_balance(user_id).await?;
        Ok(balance.balance >= amount)
    }
}
