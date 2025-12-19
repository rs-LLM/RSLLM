//! 账单服务模块
//! 提供账单生成、支付和查询功能
use crate::domain::table::ai_hub::billing::AiHubBilling;
use crate::domain::table::ai_hub::usage_log::AiHubUsageLog;
use crate::domain::dto::billing::{UpdateBillingDTO, PayBillingDTO, BillingQueryDTO, BillingStatisticsQueryDTO};
use crate::domain::vo::billing::{AiHubBillingVO, BillingOverviewVO, BillingStatisticsVO};
use crate::error::Result;
use crate::pool;
use rbatis::rbdc::DateTime;
use std::str::FromStr;
use rand::Rng;

/// 账单服务
///
/// 负责账单生成、支付和查询
#[derive(Clone)]
pub struct BillService {}

impl BillService {
    /// 生成账单
    /// 
    /// 根据用户和周期统计用量并生成账单
    pub async fn generate_bill(&self, user_id: &str, billing_cycle: &str) -> Result<String> {
        // 查询账单周期内的用量记录
        let start_time = self.get_cycle_start_time(billing_cycle)?;
        let end_time = self.get_cycle_end_time(billing_cycle)?;

        // 使用select_by_map替代select_by_wrapper
        let usage_logs = AiHubUsageLog::select_by_map(
            pool!(),
            rbs::value! {
                "user_id": user_id,
                "status": "success",
                "created_at >=": start_time,
                "created_at <=": end_time
            }
        ).await?;

        if usage_logs.is_empty() {
            return Err(Error::from("该周期内无有效用量记录"));
        }

        // 检查是否已存在账单
        let existing = AiHubBilling::select_by_map(
            pool!(),
            rbs::value! {
                "user_id": user_id,
                "billing_cycle": billing_cycle
            }
        ).await?;

        if !existing.is_empty() {
            return Err(Error::from(format!(
                "账单已存在: user_id={}, cycle={}", 
                user_id, billing_cycle
            )));
        }

        // 统计费用
        let total_amount: f64 = usage_logs.iter().map(|l| l.total_cost.unwrap_or(0.0)).sum();
        let service_amount = total_amount; // 服务费用 = 总费用
        let tax_amount = 0.0; // 税费（可后续扩展）
        
        let total_requests = usage_logs.len() as i64;
        let total_tokens: i64 = usage_logs.iter().map(|l| l.total_tokens.unwrap_or(0)).sum();

        // 生成账单编号
        let mut rng = rand::thread_rng();
        let bill_number = format!(
            "BILL{}{}{:04}",
            billing_cycle.replace("-", ""),
            user_id.chars().take(8).collect::<String>(),
            rng.gen_range(0..10000)
        );

        let bill = AiHubBilling {
            id: Some(uuid::Uuid::new_v4().to_string()),
            bill_number: bill_number.clone(),
            user_id: user_id.to_string(),
            billing_cycle: billing_cycle.to_string(),
            total_amount,
            service_amount,
            tax_amount,
            total_requests,
            total_tokens,
            payment_status: "pending".to_string(),
            payment_time: None,
            bill_status: "issued".to_string(),
            remark: Some(format!("周期账单: {}", billing_cycle)),
            created_at: Some(DateTime::now()),
            updated_at: Some(DateTime::now()),
        };

        let id = bill.id.clone().ok_or_else(|| Error::from("Failed to generate bill ID"))?;
        AiHubBilling::insert(pool!(), &bill).await?;
        Ok(id)
    }

    /// 支付账单
    pub async fn pay_bill(&self, id: &str, _dto: PayBillingDTO) -> Result<()> {
        // 使用select_by_map替代select_by_id
        let mut bill = AiHubBilling::select_by_map(
            pool!(),
            rbs::value! { "id": id }
        ).await?
        .first()
        .cloned()
        .ok_or_else(|| Error::from("Bill not found"))?;

        if bill.payment_status == "paid" {
            return Err(Error::from("账单已支付"));
        }

        if bill.bill_status == "cancelled" {
            return Err(Error::from("账单已取消"));
        }

        // 更新支付状态
        bill.payment_status = "paid".to_string();
        bill.bill_status = "paid".to_string();
        bill.payment_time = Some(DateTime::now());
        bill.updated_at = Some(DateTime::now());

        // TODO: 这里可以集成实际的支付渠道
        // 例如：支付宝、微信支付、银行转账等
        // 支付成功后更新支付流水号

        // 使用update_by_map替代update_by_id
        AiHubBilling::update_by_map(
            pool!(),
            &bill,
            rbs::value! { "id": id }
        ).await?;
        Ok(())
    }

    /// 更新账单
    pub async fn update_bill(&self, id: &str, dto: UpdateBillingDTO) -> Result<()> {
        // 使用select_by_map替代select_by_id
        let mut bill = AiHubBilling::select_by_map(
            pool!(),
            rbs::value! { "id": id }
        ).await?
        .first()
        .cloned()
        .ok_or_else(|| Error::from("Bill not found"))?;

        if let Some(total_amount) = dto.total_amount {
            bill.total_amount = total_amount;
        }
        if let Some(service_amount) = dto.service_amount {
            bill.service_amount = service_amount;
        }
        if let Some(tax_amount) = dto.tax_amount {
            bill.tax_amount = tax_amount;
        }
        if let Some(total_requests) = dto.total_requests {
            bill.total_requests = total_requests;
        }
        if let Some(total_tokens) = dto.total_tokens {
            bill.total_tokens = total_tokens;
        }
        if let Some(payment_status) = dto.payment_status {
            bill.payment_status = payment_status;
        }
        if let Some(payment_time) = &dto.payment_time {
            bill.payment_time = Some(DateTime::from_str(payment_time).map_err(|e| Error::from(format!("Invalid payment_time: {}", e)))?);
        }
        if let Some(bill_status) = dto.bill_status {
            bill.bill_status = bill_status;
        }
        if let Some(remark) = dto.remark {
            bill.remark = Some(remark);
        }

        bill.updated_at = Some(DateTime::now());

        // 使用update_by_map替代update_by_id
        AiHubBilling::update_by_map(
            pool!(),
            &bill,
            rbs::value! { "id": id }
        ).await?;
        Ok(())
    }

    /// 取消账单
    pub async fn cancel_bill(&self, id: &str) -> Result<()> {
        // 使用select_by_map替代select_by_id
        let mut bill = AiHubBilling::select_by_map(
            pool!(),
            rbs::value! { "id": id }
        ).await?
        .first()
        .cloned()
        .ok_or_else(|| Error::from("Bill not found"))?;

        if bill.payment_status == "paid" {
            return Err(Error::from("已支付的账单无法取消"));
        }

        bill.bill_status = "cancelled".to_string();
        bill.updated_at = Some(DateTime::now());

        // 使用update_by_map替代update_by_id
        AiHubBilling::update_by_map(
            pool!(),
            &bill,
            rbs::value! { "id": id }
        ).await?;
        Ok(())
    }

    /// 获取账单详情
    pub async fn get_bill(&self, id: &str) -> Result<AiHubBillingVO> {
        // 使用select_by_map替代select_by_id
        let bill = AiHubBilling::select_by_map(
            pool!(),
            rbs::value! { "id": id }
        ).await?
        .first()
        .cloned()
        .ok_or_else(|| Error::from("Bill not found"))?;
        Ok(self.to_vo(bill))
    }

    /// 查询账单列表
    pub async fn list_bills(&self, query: BillingQueryDTO) -> Result<Vec<AiHubBillingVO>> {
        // 构建查询条件
        let mut map = rbs::value! {};

        if let Some(user_id) = query.user_id {
            map["user_id"] = rbs::Value::String(user_id);
        }
        if let Some(billing_cycle) = query.billing_cycle {
            map["billing_cycle"] = rbs::Value::String(billing_cycle);
        }
        if let Some(payment_status) = query.payment_status {
            map["payment_status"] = rbs::Value::String(payment_status);
        }
        if let Some(bill_status) = query.bill_status {
            map["bill_status"] = rbs::Value::String(bill_status);
        }

        // 使用select_by_map替代select_by_wrapper
        let bills = AiHubBilling::select_by_map(pool!(), map).await?;
        Ok(bills.into_iter().map(|b| self.to_vo(b)).collect())
    }

    /// 获取账单概览
    pub async fn get_overview(&self, user_id: &str) -> Result<BillingOverviewVO> {
        // 使用select_by_map替代select_by_wrapper
        let bills = AiHubBilling::select_by_map(
            pool!(),
            rbs::value! { "user_id": user_id }
        ).await?;

        let total_bills = bills.len() as i32;
        let pending_bills = bills.iter().filter(|b| b.payment_status == "pending").count() as i32;
        let paid_bills = bills.iter().filter(|b| b.payment_status == "paid").count() as i32;

        let total_amount: f64 = bills.iter().map(|b| b.total_amount).sum();
        let paid_amount: f64 = bills.iter()
            .filter(|b| b.payment_status == "paid")
            .map(|b| b.total_amount)
            .sum();
        let pending_amount = total_amount - paid_amount;

        let bill_vos: Vec<AiHubBillingVO> = bills.iter().map(|b| self.to_vo(b.clone())).collect();

        Ok(BillingOverviewVO {
            user_id: user_id.to_string(),
            total_bills,
            pending_bills,
            paid_bills,
            total_amount,
            paid_amount,
            pending_amount,
            bills: bill_vos,
        })
    }

    /// 账单统计
    pub async fn statistics(&self, query: BillingStatisticsQueryDTO) -> Result<BillingStatisticsVO> {
        // 保存user_id副本，避免移动后无法使用
        let user_id = query.user_id.clone();
        // 构建基础查询条件
        let mut map = rbs::value! {
            "user_id": query.user_id
        };

        if let Some(period) = &query.period {
            // 支持: daily, weekly, monthly, quarterly, yearly
            let (start_time, end_time) = self.get_period_range(period)?;
            map["created_at >="] = rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(start_time.to_string())));
            map["created_at <="] = rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(end_time.to_string())));
        } else if let Some(start) = &query.start_time {
            let start_dt = DateTime::from_str(start).map_err(|e| Error::from(format!("Invalid start_time: {}", e)))?;
            map["created_at >="] = rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(start_dt.to_string())));
            if let Some(end) = &query.end_time {
                let end_dt = DateTime::from_str(end).map_err(|e| Error::from(format!("Invalid end_time: {}", e)))?;
                map["created_at <="] = rbs::Value::Ext("DateTime", Box::new(rbs::Value::String(end_dt.to_string())));
            }
        }

        // 使用select_by_map替代select_by_wrapper
        let bills = AiHubBilling::select_by_map(pool!(), map).await?;

        if bills.is_empty() {
            return Ok(BillingStatisticsVO {
                user_id: user_id.clone(),
                period: query.period.unwrap_or_else(|| "custom".to_string()),
                total_amount: 0.0,
                average_amount: 0.0,
                total_requests: 0,
                total_tokens: 0,
                average_tokens: 0,
                bill_count: 0,
            });
        }

        let total_amount: f64 = bills.iter().map(|b| b.total_amount).sum();
        let total_requests: i64 = bills.iter().map(|b| b.total_requests).sum();
        let total_tokens: i64 = bills.iter().map(|b| b.total_tokens).sum();
        let bill_count = bills.len() as i32;

        let average_amount = total_amount / bill_count as f64;
        let average_tokens = if bill_count > 0 {
            total_tokens / bill_count as i64
        } else {
            0
        };

        Ok(BillingStatisticsVO {
            user_id,
            period: query.period.unwrap_or_else(|| "custom".to_string()),
            total_amount,
            average_amount,
            total_requests,
            total_tokens,
            average_tokens,
            bill_count,
        })
    }

    /// 获取周期开始时间
    fn get_cycle_start_time(&self, billing_cycle: &str) -> Result<DateTime> {
        // 格式: 2024-01, 2024-Q1, 2024
        if billing_cycle.contains("-") {
            let parts: Vec<&str> = billing_cycle.split('-').collect();
            if parts.len() == 2 {
                let year = parts[0];
                let month = parts[1];
                return DateTime::from_str(&format!("{}-{}-01 00:00:00", year, month))
                    .map_err(|e| Error::from(format!("Invalid billing cycle format: {}", e)));
            }
        } else if billing_cycle.contains("Q") {
            let parts: Vec<&str> = billing_cycle.split('Q').collect();
            if parts.len() == 2 {
                let year = parts[0];
                let quarter: i32 = parts[1].parse().map_err(|e| Error::from(format!("Invalid quarter: {}", e)))?;
                let month = (quarter - 1) * 3 + 1;
                return DateTime::from_str(&format!("{}-{:02}-01 00:00:00", year, month))
                    .map_err(|e| Error::from(format!("Invalid billing cycle format: {}", e)));
            }
        }
        
        // 默认按年处理
        DateTime::from_str(&format!("{}-01-01 00:00:00", billing_cycle))
            .map_err(|e| Error::from(format!("Invalid billing cycle format: {}", e)))
    }

    /// 获取周期结束时间
    fn get_cycle_end_time(&self, billing_cycle: &str) -> Result<DateTime> {
        let _start = self.get_cycle_start_time(billing_cycle)?;
        
        if billing_cycle.contains("-") {
            let parts: Vec<&str> = billing_cycle.split('-').collect();
            if parts.len() == 2 {
                let year: i32 = parts[0].parse().map_err(|e| Error::from(format!("Invalid year: {}", e)))?;
                let month: i32 = parts[1].parse().map_err(|e| Error::from(format!("Invalid month: {}", e)))?;
                
                if month == 12 {
                    return DateTime::from_str(&format!("{}-12-31 23:59:59", year))
                        .map_err(|e| Error::from(format!("Invalid date: {}", e)));
                } else {
                    return DateTime::from_str(&format!("{}-{:02}-01 23:59:59", year, month + 1))
                        .map_err(|e| Error::from(format!("Invalid date: {}", e)));
                }
            }
        } else if billing_cycle.contains("Q") {
            let parts: Vec<&str> = billing_cycle.split('Q').collect();
            if parts.len() == 2 {
                let year: i32 = parts[0].parse().map_err(|e| Error::from(format!("Invalid year: {}", e)))?;
                let quarter: i32 = parts[1].parse().map_err(|e| Error::from(format!("Invalid quarter: {}", e)))?;
                let end_month = quarter * 3;
                
                if end_month == 12 {
                    return DateTime::from_str(&format!("{}-12-31 23:59:59", year))
                        .map_err(|e| Error::from(format!("Invalid date: {}", e)));
                } else {
                    return DateTime::from_str(&format!("{}-{:02}-01 23:59:59", year, end_month + 1))
                        .map_err(|e| Error::from(format!("Invalid date: {}", e)));
                }
            }
        }
        
        // 默认按年处理
        let year: i32 = billing_cycle.parse().map_err(|e| Error::from(format!("Invalid year: {}", e)))?;
        DateTime::from_str(&format!("{}-12-31 23:59:59", year))
            .map_err(|e| Error::from(format!("Invalid date: {}", e)))
    }

    /// 获取周期范围
    fn get_period_range(&self, period: &str) -> Result<(DateTime, DateTime)> {
        let now = DateTime::now();
        let end_time = now.clone();
        
        let start_time = match period {
            "daily" => {
                // 今天
                let date_str = now.to_string();
                let date = &date_str[..10];
                DateTime::from_str(&format!("{} 00:00:00", date))
                    .map_err(|e| Error::from(format!("Invalid date: {}", e)))?
            }
            "weekly" => {
                // 最近7天
                let date_str = now.to_string();
                let date = &date_str[..10];
                DateTime::from_str(&format!("{} 00:00:00", date))
                    .map_err(|e| Error::from(format!("Invalid date: {}", e)))?
            }
            "monthly" => {
                // 最近30天
                let date_str = now.to_string();
                let date = &date_str[..10];
                DateTime::from_str(&format!("{} 00:00:00", date))
                    .map_err(|e| Error::from(format!("Invalid date: {}", e)))?
            }
            "quarterly" => {
                // 最近90天
                let date_str = now.to_string();
                let date = &date_str[..10];
                DateTime::from_str(&format!("{} 00:00:00", date))
                    .map_err(|e| Error::from(format!("Invalid date: {}", e)))?
            }
            "yearly" => {
                // 最近365天
                let date_str = now.to_string();
                let date = &date_str[..10];
                DateTime::from_str(&format!("{} 00:00:00", date))
                    .map_err(|e| Error::from(format!("Invalid date: {}", e)))?
            }
            _ => {
                // 默认最近30天
                let date_str = now.to_string();
                let date = &date_str[..10];
                DateTime::from_str(&format!("{} 00:00:00", date))
                    .map_err(|e| Error::from(format!("Invalid date: {}", e)))?
            }
        };

        Ok((start_time, end_time))
    }

    /// 转换为VO
    fn to_vo(&self, bill: AiHubBilling) -> AiHubBillingVO {
        AiHubBillingVO {
            id: bill.id,
            bill_number: bill.bill_number,
            user_id: bill.user_id,
            billing_cycle: bill.billing_cycle,
            total_amount: bill.total_amount,
            service_amount: bill.service_amount,
            tax_amount: bill.tax_amount,
            total_requests: bill.total_requests,
            total_tokens: bill.total_tokens,
            payment_status: bill.payment_status,
            payment_time: bill.payment_time.map(|t| t.to_string()),
            bill_status: bill.bill_status,
            remark: bill.remark,
            created_at: bill.created_at.map(|t| t.to_string()),
        }
    }
}

use crate::error::Error;