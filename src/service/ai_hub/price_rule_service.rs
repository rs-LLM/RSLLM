//! 价格规则服务模块
//! 提供价格规则的管理和计算功能
use crate::domain::table::ai_hub::price_rule::AiHubPriceRule;
use crate::domain::dto::price_rule::{CreatePriceRuleDTO, UpdatePriceRuleDTO, PriceCalculationDTO, PriceRuleQueryDTO};
use crate::domain::vo::price_rule::{AiHubPriceRuleVO, PriceCalculationVO, AppliedRuleVO, PriceRuleOverviewVO};
use crate::error::{ApplicationError, ApplicationResult};
use crate::pool;
use rbatis::rbdc::DateTime;
use std::cmp::min;
use std::str::FromStr;

/// 价格规则服务
///
/// 负责价格规则的管理和价格计算
#[derive(Clone)]
pub struct PriceRuleService {}

impl PriceRuleService {
    /// 创建价格规则
    pub async fn create_rule(&self, dto: CreatePriceRuleDTO) -> ApplicationResult<String> {
        let effective_start = match &dto.effective_start {
            Some(t) => Some(DateTime::from_str(t).map_err(|e| ApplicationError::ValidationError {
                message: format!("Invalid effective_start: {}", e),
                field: Some("effective_start".to_string()),
                value: Some(t.clone()),
            })?),
            None => None,
        };
        
        let effective_end = match &dto.effective_end {
            Some(t) => Some(DateTime::from_str(t).map_err(|e| ApplicationError::ValidationError {
                message: format!("Invalid effective_end: {}", e),
                field: Some("effective_end".to_string()),
                value: Some(t.clone()),
            })?),
            None => None,
        };

        let rule = AiHubPriceRule {
            id: Some(uuid::Uuid::new_v4().to_string()),
            rule_name: dto.rule_name,
            conditions: dto.conditions,
            discount_rate: dto.discount_rate,
            additional_rate: dto.additional_rate,
            priority: dto.priority,
            effective_start,
            effective_end,
            status: dto.status,
            description: dto.description,
            created_at: Some(DateTime::now()),
            updated_at: Some(DateTime::now()),
        };

        let id = rule.id.clone().ok_or_else(|| ApplicationError::BusinessError {
            message: "Failed to generate rule ID".to_string(),
            code: Some("RULE_ID_GENERATION_FAILED".to_string()),
            context: Some("Failed to generate rule ID after successful creation".to_string()),
        })?;
        AiHubPriceRule::insert(pool!(), &rule).await?;
        Ok(id)
    }

    /// 更新价格规则
    pub async fn update_rule(&self, id: &str, dto: UpdatePriceRuleDTO) -> ApplicationResult<()> {
        // 使用select_by_map替代select_by_id
        let rules = AiHubPriceRule::select_by_map(pool!(), rbs::value!("id": id)).await?;
        let mut rule = rules.into_iter().next().ok_or_else(|| ApplicationError::NotFound {
            message: "Rule not found".to_string(),
            resource: Some("price_rule".to_string()),
            id: Some(id.to_string()),
        })?;

        if let Some(rule_name) = dto.rule_name {
            rule.rule_name = rule_name;
        }
        if let Some(conditions) = dto.conditions {
            rule.conditions = Some(conditions);
        }
        if let Some(discount_rate) = dto.discount_rate {
            rule.discount_rate = Some(discount_rate);
        }
        if let Some(additional_rate) = dto.additional_rate {
            rule.additional_rate = Some(additional_rate);
        }
        if let Some(priority) = dto.priority {
            rule.priority = priority;
        }
        if let Some(effective_start) = &dto.effective_start {
            rule.effective_start = Some(DateTime::from_str(effective_start).map_err(|e| ApplicationError::ValidationError {
                message: format!("Invalid effective_start: {}", e),
                field: Some("effective_start".to_string()),
                value: Some(effective_start.clone()),
            })?);
        }
        if let Some(effective_end) = &dto.effective_end {
            rule.effective_end = Some(DateTime::from_str(effective_end).map_err(|e| ApplicationError::ValidationError {
                message: format!("Invalid effective_end: {}", e),
                field: Some("effective_end".to_string()),
                value: Some(effective_end.clone()),
            })?);
        }
        if let Some(status) = dto.status {
            rule.status = status;
        }
        if let Some(description) = dto.description {
            rule.description = Some(description);
        }

        rule.updated_at = Some(DateTime::now());
        // 使用update_by_map替代update_by_id
        AiHubPriceRule::update_by_map(pool!(), &rule, rbs::value!("id": id)).await?;
        Ok(())
    }

    /// 删除价格规则
    pub async fn delete_rule(&self, id: &str) -> ApplicationResult<()> {
        AiHubPriceRule::delete_by_map(pool!(), rbs::value!("id": id)).await?;
        Ok(())
    }

    /// 获取价格规则详情
    pub async fn get_rule(&self, id: &str) -> ApplicationResult<AiHubPriceRuleVO> {
        let rules = AiHubPriceRule::select_by_map(pool!(), rbs::value!("id": id)).await?;
        let rule = rules.into_iter().next()
            .ok_or_else(|| ApplicationError::NotFound {
                message: "Rule not found".to_string(),
                resource: Some("price_rule".to_string()),
                id: Some(id.to_string()),
            })?;
        Ok(self.to_vo(rule))
    }

    /// 查询价格规则列表
    pub async fn list_rules(&self, query: PriceRuleQueryDTO) -> ApplicationResult<Vec<AiHubPriceRuleVO>> {
        // 使用select_by_map查询所有记录，然后手动筛选
        let all_rules = AiHubPriceRule::select_all(pool!()).await?;
        
        // 手动筛选记录
        let mut filtered_rules: Vec<AiHubPriceRule> = Vec::new();
        
        for rule in all_rules {
            // 筛选规则名称
            if let Some(rule_name) = &query.rule_name {
                if !rule.rule_name.contains(rule_name) {
                    continue;
                }
            }
            
            // 筛选状态
            if let Some(status) = &query.status {
                if rule.status != *status {
                    continue;
                }
            }
            
            // 筛选优先级
            if let Some(priority) = query.priority {
                if rule.priority != priority {
                    continue;
                }
            }
            
            // 筛选是否为活跃规则
            if let Some(true) = query.active_only {
                let now = DateTime::now();
                let effective_start = rule.effective_start.clone().unwrap_or(
                    DateTime::from_str("1970-01-01T00:00:00").map_err(|_| ApplicationError::ConfigError {
                        message: "Invalid default date".to_string(),
                        key: Some("default_date".to_string()),
                    })?
                );
                let effective_end = rule.effective_end.clone();
                
                if effective_start > now {
                    continue;
                }
                
                if let Some(end) = effective_end {
                    if end < now {
                        continue;
                    }
                }
            }
            
            filtered_rules.push(rule);
        }
        
        // 按优先级排序
        filtered_rules.sort_by(|a, b| a.priority.cmp(&b.priority));
        
        // 处理分页
        let mut paginated_rules = filtered_rules;
        if let Some(page) = query.page {
            if let Some(page_size) = query.page_size {
                let page = page.max(1);
                let page_size = page_size.max(1);
                let start = ((page - 1) * page_size) as usize;
                let end = min(start + page_size as usize, paginated_rules.len());
                paginated_rules = paginated_rules[start..end].to_vec();
            }
        }
        
        Ok(paginated_rules.into_iter().map(|r| self.to_vo(r)).collect())
    }

    /// 获取价格规则概览
    pub async fn get_overview(&self) -> ApplicationResult<PriceRuleOverviewVO> {
        let now = DateTime::now();
        
        // 获取所有活跃规则
        let all_active_rules = AiHubPriceRule::select_by_map(
            pool!(),
            rbs::value!("status": "active")
        ).await?;
        
        // 手动计算各类规则数量
        let mut active_count = 0;
        let mut pending_count = 0;
        let mut expired_count = 0;
        
        for rule in all_active_rules {
            let effective_start = rule.effective_start.unwrap_or(
                DateTime::from_str("1970-01-01T00:00:00").map_err(|_| ApplicationError::ConfigError {
                    message: "Invalid default date".to_string(),
                    key: Some("default_date".to_string()),
                })?
            );
            let effective_end = rule.effective_end;
            
            if effective_start > now {
                // 待生效规则
                pending_count += 1;
            } else if let Some(end) = effective_end {
                if end < now {
                    // 过期规则
                    expired_count += 1;
                } else {
                    // 活跃规则
                    active_count += 1;
                }
            } else {
                // 永久有效规则
                active_count += 1;
            }
        }

        let rules = self.list_rules(PriceRuleQueryDTO {
            rule_name: None,
            status: None,
            priority: None,
            active_only: None,
            page: Some(1),
            page_size: Some(100),
        }).await?;

        Ok(PriceRuleOverviewVO {
            active_rules: active_count,
            pending_rules: pending_count,
            expired_rules: expired_count,
            rules,
        })
    }

    /// 计算价格
    pub async fn calculate_price(&self, dto: PriceCalculationDTO) -> ApplicationResult<PriceCalculationVO> {
        let base_price = dto.base_price;
        let input_tokens = dto.input_tokens;
        let output_tokens = dto.output_tokens;
        
        // 获取匹配的规则
        let matching_rules = self.get_matching_rules(&dto).await?;
        
        let mut final_price = base_price;
        let mut discount_amount = 0.0;
        let mut additional_amount = 0.0;
        let mut applied_rules = vec![];

        for rule in matching_rules {
            let rule_discount = rule.discount_rate.unwrap_or(0.0);
            let rule_additional = rule.additional_rate.unwrap_or(0.0);
            
            // 计算规则影响
            let discount_impact = base_price * rule_discount;
            let additional_impact = base_price * rule_additional;
            
            final_price = final_price * (1.0 - rule_discount) * (1.0 + rule_additional);
            discount_amount += discount_impact;
            additional_amount += additional_impact;

            applied_rules.push(AppliedRuleVO {
                rule_id: rule.id.unwrap_or_default(),
                rule_name: rule.rule_name,
                discount_rate: rule.discount_rate,
                additional_rate: rule.additional_rate,
                priority: rule.priority,
                impact_amount: discount_impact - additional_impact,
            });
        }

        // 计算总费用（分）
        let total_cost = (final_price * (input_tokens + output_tokens) as f64) / 1000.0;

        Ok(PriceCalculationVO {
            base_price,
            applied_rules,
            final_price,
            discount_amount,
            additional_amount,
            total_amount: total_cost,
        })
    }

    /// 获取匹配的规则
    async fn get_matching_rules(&self, dto: &PriceCalculationDTO) -> ApplicationResult<Vec<AiHubPriceRule>> {
        let now = DateTime::now();
        
        // 查询所有活跃规则
        let all_active_rules = AiHubPriceRule::select_by_map(
            pool!(),
            rbs::value!("status": "active")
        ).await?;
        
        // 手动筛选在有效期内的规则
        let mut valid_rules = vec![];
        
        for rule in all_active_rules {
            let effective_start = rule.effective_start.clone().unwrap_or(
                DateTime::from_str("1970-01-01T00:00:00").map_err(|_| ApplicationError::ConfigError {
                    message: "Invalid default date".to_string(),
                    key: Some("default_date".to_string()),
                })?
            );
            let effective_end = rule.effective_end.clone();
            
            // 检查是否在有效期内
            let is_valid = effective_start <= now && 
                          (effective_end.is_none() || effective_end.map(|end| end >= now).unwrap_or(true));
            
            if is_valid {
                valid_rules.push(rule);
            }
        }
        
        // 按优先级排序
        valid_rules.sort_by(|a, b| a.priority.cmp(&b.priority));
        
        let mut matching_rules = vec![];

        for rule in valid_rules {
            if self.rule_matches(&rule, dto) {
                matching_rules.push(rule);
            }
        }

        Ok(matching_rules)
    }

    /// 检查规则是否匹配
    fn rule_matches(&self, rule: &AiHubPriceRule, dto: &PriceCalculationDTO) -> bool {
        let conditions = match &rule.conditions {
            Some(c) => c,
            None => return true, // 无条件限制
        };

        // 检查用户等级
        if let Some(user_level) = &dto.user_level {
            if let Some(conditions_obj) = conditions.as_object() {
                if let Some(required_level) = conditions_obj.get("user_level") {
                    if let Some(level_str) = required_level.as_str() {
                        if level_str != user_level {
                            return false;
                        }
                    }
                }
            }
        }

        // 检查用量区间
        if let Some(total_usage) = &dto.total_usage {
            if let Some(conditions_obj) = conditions.as_object() {
                if let Some(min_usage) = conditions_obj.get("min_usage") {
                    if let Some(min) = min_usage.as_f64() {
                        if total_usage < &min {
                            return false;
                        }
                    }
                }
                if let Some(max_usage) = conditions_obj.get("max_usage") {
                    if let Some(max) = max_usage.as_f64() {
                        if total_usage > &max {
                            return false;
                        }
                    }
                }
            }
        }

        // 检查token数量范围
        if let Some(conditions_obj) = conditions.as_object() {
            if let Some(min_tokens) = conditions_obj.get("min_tokens") {
                if let Some(min) = min_tokens.as_i64() {
                    if (dto.input_tokens + dto.output_tokens) < min {
                        return false;
                    }
                }
            }
            if let Some(max_tokens) = conditions_obj.get("max_tokens") {
                if let Some(max) = max_tokens.as_i64() {
                    if (dto.input_tokens + dto.output_tokens) > max {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// 转换为VO
    fn to_vo(&self, rule: AiHubPriceRule) -> AiHubPriceRuleVO {
        AiHubPriceRuleVO {
            id: rule.id,
            rule_name: rule.rule_name,
            conditions: rule.conditions,
            discount_rate: rule.discount_rate,
            additional_rate: rule.additional_rate,
            priority: rule.priority,
            effective_start: rule.effective_start.map(|t| t.to_string()),
            effective_end: rule.effective_end.map(|t| t.to_string()),
            status: rule.status,
            description: rule.description,
            created_at: rule.created_at.map(|t| t.to_string()),
        }
    }
}