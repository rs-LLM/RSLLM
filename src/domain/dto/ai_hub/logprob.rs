// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据传输
use serde::{Deserialize, Serialize};
// 用途：导入哈希映射类型
// 说明：用于存储logit_bias等键值对数据
use std::collections::HashMap;
// 用途：导入OpenAPI文档生成支持
// 说明：用于自动生成API文档
use utoipa::ToSchema;

// 用途：日志概率结构体
// 说明：用于存储token级别的概率信息，用于分析模型的决策过程
#[derive(Deserialize, Serialize, Clone, ToSchema, Debug)]
pub struct LogProbs {
    // 用途：令牌列表
    // 说明：包含所有被分析的token序列
    pub tokens: Vec<String>,
    // 用途：令牌对数概率
    // 说明：每个token的对数概率值
    pub token_logprobs: Vec<f32>,
    // 用途：顶级对数概率
    // 说明：每个位置顶级候选token的概率分布
    pub top_logprobs: Vec<HashMap<String, f32>>,
    // 用途：文本偏移
    // 说明：每个token在原始文本中的位置偏移
    pub text_offset: Vec<usize>,
}
