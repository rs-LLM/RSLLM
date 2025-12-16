// 用途：导入AI处理管道相关结构体
// 说明：用于初始化管道配置表结构
use crate::domain::table::ai_hub::provider::{Pipeline, PipelinePluginConfig};
// 用途：导入AI模型定义相关结构体
// 说明：用于初始化模型定义表结构
use crate::domain::table::ai_hub::provider::ModelDefinition;
// 用途：导入AI服务提供商相关结构体
// 说明：用于初始化提供商配置表结构
use crate::domain::table::ai_hub::provider::Provider;
// 用途：导入日志级别枚举
// 说明：用于控制日志输出级别
use log::LevelFilter;
// 用途：导入RBatis结构体
// 说明：用于数据库操作和表结构同步
use rbatis::RBatis;
// 用途：导入defer宏
// 说明：用于延迟执行代码，确保日志级别恢复
use rbatis::dark_std::defer;
// 用途：导入日志拦截器
// 说明：用于临时关闭日志输出，避免表同步过程中产生大量日志
use rbatis::intercept_log::LogInterceptor;
// 用途：导入日期时间类型
// 说明：用于记录创建时间
use rbatis::rbdc::DateTime;
// 用途：导入表同步相关的映射器
// 说明：用于根据数据库类型选择合适的表映射器
use rbatis::table_sync::{
    ColumnMapper, MssqlTableMapper, MysqlTableMapper, PGTableMapper, SqliteTableMapper,
};
// 用途：导入ULID类型
// 说明：用于生成唯一标识符，确保数据库表中的主键唯一性
use ulid::Ulid;

// 用途：同步AI Hub相关数据库表结构
// 说明：根据表结构定义自动创建或更新AI Hub相关的数据库表
pub async fn sync_tables(rb: &RBatis) {
    // 用途：获取日志拦截器
    // 说明：用于临时关闭日志输出
    let log_intercept = rb.get_intercept::<LogInterceptor>().expect("not find log interceptor");
    // 用途：保存当前日志级别
    // 说明：便于后续恢复
    let level = log_intercept.get_level_filter().clone();
    // 用途：关闭日志输出
    // 说明：避免表同步过程中产生大量日志，影响性能和可读性
    log_intercept.set_level_filter(LevelFilter::Off);
    // 用途：延迟恢复日志级别
    // 说明：确保无论函数如何退出，日志级别都会恢复
    defer!(|| {
        log_intercept.set_level_filter(level);
    });
    
    // 用途：根据数据库类型选择合适的表映射器
    // 说明：不同数据库的表结构定义语法不同，需要专用的映射器
    let mapper: &dyn ColumnMapper = {
        match rb.driver_type().unwrap_or_default() {
            "sqlite" => &SqliteTableMapper {},
            "mssql" => &MssqlTableMapper {},
            "mysql" => &MysqlTableMapper {},
            "postgres" => &PGTableMapper {},
            _ => {
                panic!("not find driver mapper")
            }
        }
    };
    
    // 用途：获取数据库连接
    // 说明：用于执行表同步操作
    let conn = rb.acquire().await.expect("connection database fail");
    
    // 用途：同步AI服务提供商表结构
    // 说明：存储各种AI服务提供商的基础配置信息，如OpenAI、Claude等
    let table = Provider {
        id: Some(Default::default()),
        name: Default::default(),
        provider_type: Default::default(),
        config_details: Default::default(),
        enabled: Some(Default::default()),
        created_at: Some(Default::default()),
        updated_at: Some(Default::default()),
    };
    let _ = RBatis::sync(&conn, mapper, &table, "provider").await;
    
    // 用途：同步AI模型定义表结构
    // 说明：存储AI模型的基础信息和配置参数
    let table = ModelDefinition {
        id: Some(Default::default()),
        key: Default::default(),
        model_type: Default::default(),
        provider_id: Some(Default::default()),
        config_details: Some(Default::default()),
        enabled: Some(Default::default()),
        created_at: Some(Default::default()),
        updated_at: Some(Default::default()),
    };
    let _ = RBatis::sync(&conn, mapper, &table, "model_definition").await;
    
    // 用途：同步AI处理管道表结构
    // 说明：存储AI处理流程的配置信息，支持多步骤处理
    let table = Pipeline {
        id: Some(Default::default()),
        name: Default::default(),
        pipeline_type: Default::default(),
        description: Some(Default::default()),
        enabled: Some(Default::default()),
        created_at: Some(Default::default()),
        updated_at: Some(Default::default()),
    };
    let _ = RBatis::sync(&conn, mapper, &table, "pipeline").await;
    
    // 用途：同步管道插件配置表结构
    // 说明：存储管道中各个插件的具体配置信息
    let table = PipelinePluginConfig {
        id: Some(Default::default()),
        pipeline_id: Some(Default::default()),
        plugin_type: Default::default(),
        config_data: Default::default(),
        enabled: Some(Default::default()),
        order_in_pipeline: Some(Default::default()),
        created_at: Some(Default::default()),
        updated_at: Some(Default::default()),
    };
    let _ = RBatis::sync(&conn, mapper, &table, "pipeline_plugin_config").await;
}

// 用途：初始化AI Hub默认数据
// 说明：创建默认的AI服务提供商、模型和管道配置，确保AI Hub功能能正常使用
pub async fn sync_tables_data(rb: &RBatis) {
    // 用途：获取数据库连接
    // 说明：用于执行数据初始化操作
    let conn = rb.acquire().await.expect("init ai_hub data fail");
    
    // 用途：检查是否已存在默认提供商
    // 说明：避免重复初始化数据
    if let Ok(v) = Provider::select_by_map(&conn, rbs::value! {"name":"OpenAI"}).await {
        if v.len() > 0 {
            // 用途：如果提供商已存在，直接返回
            // 说明：避免重复初始化数据
            return;
        }
    };
    
    // 用途：插入默认的OpenAI提供商配置
    // 说明：提供基础的AI服务提供商配置
    let openai_provider_id = Ulid::new().to_string();
    let _ = Provider::insert(
        &conn,
        &Provider {
            id: Some(openai_provider_id.clone()),
            name: "OpenAI".to_string(),
            provider_type: "openai".to_string(),
            config_details: serde_json::json!({
                "api_base": "https://api.openai.com/v1",
                "timeout": 30,
                "max_retries": 3
            }),
            enabled: Some(true),
            created_at: Some(DateTime::now()),
            updated_at: Some(DateTime::now()),
        },
    )
    .await;

    // 用途：插入默认的GPT模型定义
    // 说明：提供基础的聊天模型配置
    let gpt_3_5_turbo_chat_id = Ulid::new().to_string();
    let _ = ModelDefinition::insert(
        &conn,
        &ModelDefinition {
            id: Some(gpt_3_5_turbo_chat_id.clone()),
            key: "gpt-3.5-turbo".to_string(),
            model_type: "chat".to_string(),
            provider_id: Some(openai_provider_id.clone()),
            config_details: Some(serde_json::json!({
                "max_tokens": 4096,
                "temperature": 0.7,
                "top_p": 1.0,
                "frequency_penalty": 0.0,
                "presence_penalty": 0.0
            })),
            enabled: Some(true),
            created_at: Some(DateTime::now()),
            updated_at: Some(DateTime::now()),
        },
    )
    .await;

    let gpt_3_5_turbo_id = Ulid::new().to_string();
    // 用途：插入默认的文本补全模型
    // 说明：提供基础的补全模型配置
    let _ = ModelDefinition::insert(
        &conn,
        &ModelDefinition {
            id: Some(gpt_3_5_turbo_id.clone()),
            key: "gpt-3.5-turbo-instruct".to_string(),
            model_type: "completion".to_string(),
            provider_id: Some(openai_provider_id.clone()),
            config_details: Some(serde_json::json!({
                "max_tokens": 4096,
                "temperature": 0.7,
                "top_p": 1.0
            })),
            enabled: Some(true),
            created_at: Some(DateTime::now()),
            updated_at: Some(DateTime::now()),
        },
    )
    .await;

    // 用途：插入默认的基础聊天管道
    // 说明：提供基础的聊天处理流程
    let _ = Pipeline::insert(
        &conn,
        &Pipeline {
            id: Some(gpt_3_5_turbo_id.clone()),
            name: "基础聊天管道".to_string(),
            pipeline_type: "chat".to_string(),
            description: Some("用于处理基础聊天对话的管道".to_string()),
            enabled: Some(true),
            created_at: Some(DateTime::now()),
            updated_at: Some(DateTime::now()),
        },
    )
    .await;

    // 用途：插入基础管道的模型插件配置
    // 说明：将模型配置到管道中，形成完整的处理流程
    let _ = PipelinePluginConfig::insert(
        &conn,
        &PipelinePluginConfig {
            id: Some(Ulid::new().to_string()),
            pipeline_id: Some(gpt_3_5_turbo_id.clone()),
            plugin_type: "model".to_string(),
            config_data: serde_json::json!({
                "model_id": gpt_3_5_turbo_id.clone(),
                "model_key": "gpt-3.5-turbo"
            }),
            enabled: Some(true),
            order_in_pipeline: Some(1),
            created_at: Some(DateTime::now()),
            updated_at: Some(DateTime::now()),
        },
    )
    .await;
}