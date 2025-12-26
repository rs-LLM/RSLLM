// 用途：导入日期时间类型
// 说明：用于记录计费标准的创建和更新时间
use rbatis::rbdc::DateTime;
// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据持久化
use serde::{Serialize, Deserialize};
// 用途：导入rbatis的CRUD宏
// 说明：用于自动生成增删改查操作
use rbatis::{crud};

// 用途：计费标准表结构体
// 说明：管理AI模型的计费标准，包括输入和输出的价格
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pricing {
    // 用途：计费标准ID
    // 说明：计费标准的唯一标识符
    pub id: Option<String>,
    // 用途：模型ID
    // 说明：关联的AI模型标识符
    pub model_id: String,
    // 用途：输入价格
    // 说明：每百万输入token的价格，单位为元
    pub input_price: f64,
    // 用途：输出价格
    // 说明：每百万输出token的价格，单位为元
    pub output_price: f64,
    // 用途：状态
    // 说明：计费标准的状态，如"active"、"inactive"等
    pub status: String,
    // 用途：描述
    // 说明：计费标准的详细描述
    pub description: Option<String>,
    // 用途：创建时间
    // 说明：记录的创建时间
    pub created_at: Option<DateTime>,
    // 用途：更新时间
    // 说明：记录的最后更新时间
    pub updated_at: Option<DateTime>,
}

// 用途：生成Pricing的CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(Pricing {});
