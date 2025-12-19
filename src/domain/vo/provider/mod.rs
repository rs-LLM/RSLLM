// 用途：导入日期时间类型
// 说明：用于记录服务提供商的创建和更新时间
use rbatis::rbdc::DateTime;
// 用途：导入序列化和反序列化支持
// 说明：用于结构体的JSON转换和数据持久化
use serde::{Serialize,Deserialize};

// 用途：导入管道插件配置结构体
// 说明：用于关联管道和插件配置
use crate::domain::table::provider::PipelinePluginConfig;

// 用途：带插件的管道组合结构体
// 说明：用于服务层和仓库层逻辑，不是直接的数据库模型，用于组合查询结果
#[derive(Clone, Debug,Serialize,Deserialize)]
pub struct PipelineWithPlugins {
    // 用途：管道ID
    // 说明：管道的唯一标识符
    pub id: Option<String>,
    // 用途：管道名称
    // 说明：管道的显示名称
    pub name: String,
    // 用途：管道类型
    // 说明：管道的类型分类
    pub pipeline_type: String,
    // 用途：管道描述
    // 说明：管道的功能描述
    pub description: Option<String>,
    // 用途：是否启用
    // 说明：控制当前管道是否可用
    pub enabled: Option<bool>,
    // 用途：创建时间
    // 说明：记录管道的创建时间
    pub created_at: Option<DateTime>,
    // 用途：更新时间
    // 说明：记录管道的最后修改时间
    pub updated_at: Option<DateTime>,
    // 用途：关联的插件列表
    // 说明：管道中包含的所有插件配置，按order_in_pipeline排序
    pub plugins: Vec<PipelinePluginConfig>,
}
