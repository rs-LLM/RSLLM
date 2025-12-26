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
// 用途：导入计费标准表
// 说明：用于数据库操作
use crate::domain::table::ai_hub::Pricing;
// 用途：导入计费标准DTO
// 说明：用于接收请求数据
use crate::domain::dto::ai_hub::{CreatePricingDTO, UpdatePricingDTO, QueryPricingDTO};
// 用途：导入计费标准VO
// 说明：用于返回响应数据
use crate::domain::vo::ai_hub::PricingVO;
// 用途：导入模型定义表
// 说明：用于查询模型信息
use crate::domain::table::ai_hub::ModelDefinition;
// 用途：导入数据库连接池
// 说明：用于获取数据库连接
use crate::pool;

/// 计费标准管理服务
///
/// 负责AI模型计费标准的增删改查操作
#[derive(Clone)]
pub struct PricingService {}

impl PricingService {
    /// 创建计费标准
    pub async fn create_pricing(&self, dto: CreatePricingDTO, _operator_id: Option<String>) -> ApplicationResult<String> {
        let id = Ulid::new().to_string();
        let now = DateTime::now();
        
        let pricing = Pricing {
            id: Some(id.clone()),
            model_id: dto.model_id.clone(),
            input_price: dto.input_price,
            output_price: dto.output_price,
            status: dto.status.clone(),
            description: dto.description,
            created_at: Some(now.clone()),
            updated_at: Some(now),
        };
        
        Pricing::insert(pool!(), &pricing).await?;
        
        Ok(id)
    }

    /// 更新计费标准
    pub async fn update_pricing(&self, dto: UpdatePricingDTO) -> ApplicationResult<()> {
        let pricings = Pricing::select_by_map(pool!(), rbs::value! { "id": &dto.id }).await?;
        
        if pricings.is_empty() {
            return Err(ApplicationError::NotFound {
                message: "Pricing not found".to_string(),
                resource: Some("pricing".to_string()),
                id: Some(dto.id.clone()),
            });
        }
        
        let mut pricing = pricings[0].clone();
        
        if let Some(input_price) = dto.input_price {
            pricing.input_price = input_price;
        }
        if let Some(output_price) = dto.output_price {
            pricing.output_price = output_price;
        }
        if let Some(status) = dto.status {
            pricing.status = status;
        }
        if let Some(description) = dto.description {
            pricing.description = Some(description);
        }
        pricing.updated_at = Some(DateTime::now());
        
        Pricing::update_by_map(pool!(), &pricing, rbs::value! { "id": &dto.id }).await?;
        
        Ok(())
    }

    /// 删除计费标准
    pub async fn delete_pricing(&self, id: &str) -> ApplicationResult<()> {
        Pricing::delete_by_map(pool!(), rbs::value! { "id": id }).await?;
        Ok(())
    }

    /// 查询计费标准详情
    pub async fn get_pricing(&self, id: &str) -> ApplicationResult<PricingVO> {
        let pricings = Pricing::select_by_map(pool!(), rbs::value! { "id": id }).await?;
        
        if pricings.is_empty() {
            return Err(ApplicationError::NotFound {
                message: "Pricing not found".to_string(),
                resource: Some("pricing".to_string()),
                id: Some(id.to_string()),
            });
        }
        
        let pricing = &pricings[0];
        
        let model_name = if let Ok(models) = ModelDefinition::select_by_map(pool!(), rbs::value! { "id": &pricing.model_id }).await {
            if !models.is_empty() {
                Some(models[0].name.clone())
            } else {
                None
            }
        } else {
            None
        };
        
        Ok(PricingVO {
            id: pricing.id.clone(),
            model_id: pricing.model_id.clone(),
            model_name,
            input_price: pricing.input_price,
            output_price: pricing.output_price,
            status: pricing.status.clone(),
            description: pricing.description.clone(),
            created_at: pricing.created_at.as_ref().map(|dt| dt.to_string()),
            updated_at: pricing.updated_at.as_ref().map(|dt| dt.to_string()),
        })
    }

    /// 查询计费标准列表
    pub async fn list_pricing(&self, dto: QueryPricingDTO) -> ApplicationResult<Vec<PricingVO>> {
        let mut conditions = rbs::value!({});
        
        if let Some(model_id) = dto.model_id {
            conditions["model_id"] = rbs::value!(model_id);
        }
        if let Some(status) = dto.status {
            conditions["status"] = rbs::value!(status);
        }
        
        let page = dto.page.unwrap_or(1);
        let page_size = dto.page_size.unwrap_or(20);
        let offset = (page - 1) * page_size;
        
        let mut pricings = Pricing::select_by_map(pool!(), conditions).await?;
        
        pricings = pricings.into_iter()
            .skip(offset as usize)
            .take(page_size as usize)
            .collect();
        
        let mut result = Vec::new();
        for pricing in pricings {
            let model_name = if let Ok(models) = ModelDefinition::select_by_map(pool!(), rbs::value! { "id": &pricing.model_id }).await {
                if !models.is_empty() {
                    Some(models[0].name.clone())
                } else {
                    None
                }
            } else {
                None
            };
            
            result.push(PricingVO {
                id: pricing.id.clone(),
                model_id: pricing.model_id.clone(),
                model_name,
                input_price: pricing.input_price,
                output_price: pricing.output_price,
                status: pricing.status.clone(),
                description: pricing.description.clone(),
                created_at: pricing.created_at.as_ref().map(|dt| dt.to_string()),
                updated_at: pricing.updated_at.as_ref().map(|dt| dt.to_string()),
            });
        }
        
        Ok(result)
    }

    /// 根据模型ID查询计费标准
    pub async fn get_pricing_by_model(&self, model_id: &str) -> ApplicationResult<Option<PricingVO>> {
        let pricings = Pricing::select_by_map(pool!(), rbs::value! { "model_id": model_id }).await?;
        
        if pricings.is_empty() {
            return Ok(None);
        }
        
        let pricing = &pricings[0];
        
        let model_name = if let Ok(models) = ModelDefinition::select_by_map(pool!(), rbs::value! { "id": &pricing.model_id }).await {
            if !models.is_empty() {
                Some(models[0].name.clone())
            } else {
                None
            }
        } else {
            None
        };
        
        Ok(Some(PricingVO {
            id: pricing.id.clone(),
            model_id: pricing.model_id.clone(),
            model_name,
            input_price: pricing.input_price,
            output_price: pricing.output_price,
            status: pricing.status.clone(),
            description: pricing.description.clone(),
            created_at: pricing.created_at.as_ref().map(|dt| dt.to_string()),
            updated_at: pricing.updated_at.as_ref().map(|dt| dt.to_string()),
        }))
    }
}
