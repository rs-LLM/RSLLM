// 用途：导入AI处理管道相关结构体
// 说明：用于初始化管道配置表结构
use crate::domain::table::provider::{Pipeline, PipelinePluginConfig};
// 用途：导入AI模型定义相关结构体
// 说明：用于初始化模型定义表结构
use crate::domain::table::ai_hub::model_definition::ModelDefinition;
// 用途：导入AI服务提供商相关结构体
// 说明：用于初始化提供商配置表结构
use crate::domain::table::ai_hub::provider_config::ProviderConfig;
// 用途：导入用量计费相关结构体
// 说明：用于初始化用量记录、配额、账单、价格规则表结构
use crate::domain::table::ai_hub::usage_log::AiHubUsageLog;
use crate::domain::table::ai_hub::user_quota::AiHubUserQuota;
use crate::domain::table::ai_hub::billing::AiHubBilling;
use crate::domain::table::ai_hub::price_rule::AiHubPriceRule;
// 用途：导入API密钥相关结构体
// 说明：用于初始化API密钥表结构
use crate::domain::table::ai_hub::api_key::ApiKey;
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
pub async fn ai_hub_sync_tables(rb: &RBatis) {
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
    
    // 用途：同步供应商配置表结构
    // 说明：存储各种AI服务供应商的基础配置信息
    let table = ProviderConfig {
        id: Some(Default::default()),
        name: Default::default(),
        provider_type: Default::default(),
        api_base: Default::default(),
        api_key_encrypted: Some(Default::default()),
        auth_type: Some(Default::default()),
        auth_config: Some(Default::default()),
        default_input_price: Some(Default::default()),
        default_output_price: Some(Default::default()),
        rate_limit_enabled: Some(Default::default()),
        max_concurrent_requests: Some(Default::default()),
        circuit_breaker_enabled: Some(Default::default()),
        failure_threshold: Some(Default::default()),
        timeout_seconds: Some(Default::default()),
        enabled: Some(Default::default()),
        status: Some(Default::default()),
        description: Some(Default::default()),
        documentation_url: Some(Default::default()),
        created_at: Some(Default::default()),
        updated_at: Some(Default::default()),
    };
    let _ = RBatis::sync(&conn, mapper, &table, "provider_config").await;
    
    // 用途：同步AI模型定义表结构
    // 说明：存储AI模型的基础信息和配置参数
    let table = ModelDefinition {
        id: Some(Default::default()),
        key: Default::default(),
        name: Default::default(),
        model_type: Default::default(),
        provider_id: Default::default(),
        api_endpoint: Some(Default::default()),
        api_key_encrypted: Some(Default::default()),
        input_price: Default::default(),
        output_price: Default::default(),
        currency: Some(Default::default()),
        max_tokens_per_request: Some(Default::default()),
        max_requests_per_minute: Some(Default::default()),
        enabled: Some(Default::default()),
        status: Some(Default::default()),
        description: Some(Default::default()),
        capabilities: Some(Default::default()),
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
    
    // 用途：同步用量记录表结构
    // 说明：记录每次AI请求的详细用量信息
    let table = AiHubUsageLog {
        id: Some(Default::default()),
        request_id: Default::default(),
        user_id: Default::default(),
        model_id: Default::default(),
        input_tokens: Default::default(),
        output_tokens: Default::default(),
        total_tokens: Default::default(),
        input_price: Default::default(),
        output_price: Default::default(),
        total_cost: Default::default(),
        currency: Some(Default::default()),
        request_method: Some(Default::default()),
        request_path: Some(Default::default()),
        request_headers: Some(Default::default()),
        request_body: Some(Default::default()),
        status_code: Some(Default::default()),
        response_time_ms: Some(Default::default()),
        error_message: Some(Default::default()),
        quota_deducted: Some(Default::default()),
        quota_snapshot: Some(Default::default()),
        ip_address: Some(Default::default()),
        user_agent: Some(Default::default()),
        created_at: Some(Default::default()),
    };
    let _ = RBatis::sync(&conn, mapper, &table, "ai_hub_usage_log").await;
    
    // 用途：同步用户配额表结构
    // 说明：管理用户的配额和余额
    let table = AiHubUserQuota {
        id: Some(Default::default()),
        user_id: Default::default(),
        total_quota: Default::default(),
        used_quota: Default::default(),
        remaining_quota: Default::default(),
        quota_period: Some(Default::default()),
        period_start: Some(Default::default()),
        period_end: Some(Default::default()),
        status: Some(Default::default()),
        warning_threshold: Some(Default::default()),
        critical_threshold: Some(Default::default()),
        created_at: Some(Default::default()),
        updated_at: Some(Default::default()),
        last_used_at: Some(Default::default()),
    };
    let _ = RBatis::sync(&conn, mapper, &table, "ai_hub_user_quota").await;
    
    // 用途：同步账单表结构
    // 说明：记录周期性账单信息
    let table = AiHubBilling {
        id: Some(Default::default()),
        bill_number: Default::default(),
        user_id: Default::default(),
        billing_cycle: Default::default(),
        total_amount: Default::default(),
        service_amount: Default::default(),
        tax_amount: Default::default(),
        total_requests: Default::default(),
        total_tokens: Default::default(),
        payment_status: Default::default(),
        payment_time: Some(Default::default()),
        bill_status: Default::default(),
        remark: Some(Default::default()),
        created_at: Some(Default::default()),
        updated_at: Some(Default::default()),
    };
    let _ = RBatis::sync(&conn, mapper, &table, "ai_hub_billing").await;
    
    // 用途：同步价格规则表结构
    // 说明：管理动态价格规则
    let table = AiHubPriceRule {
        id: Some(Default::default()),
        rule_name: Default::default(),
        conditions: Some(Default::default()),
        discount_rate: Some(Default::default()),
        additional_rate: Some(Default::default()),
        priority: Default::default(),
        effective_start: Some(Default::default()),
        effective_end: Some(Default::default()),
        status: Default::default(),
        description: Some(Default::default()),
        created_at: Some(Default::default()),
        updated_at: Some(Default::default()),
    };
    let _ = RBatis::sync(&conn, mapper, &table, "ai_hub_price_rule").await;
    
    // 用途：同步API密钥表结构
    // 说明：管理用户的API密钥和权限控制
    let table = ApiKey {
        id: Some(Default::default()),
        key_hash: Default::default(),
        user_id: Default::default(),
        name: Some(Default::default()),
        prefix: Some(Default::default()),
        permissions: Some(Default::default()),
        allowed_models: Some(Default::default()),
        denied_models: Some(Default::default()),
        rate_limit_enabled: Some(Default::default()),
        max_requests_per_minute: Some(Default::default()),
        ip_whitelist: Some(Default::default()),
        ip_blacklist: Some(Default::default()),
        enabled: Some(Default::default()),
        status: Some(Default::default()),
        expires_at: Some(Default::default()),
        created_at: Some(Default::default()),
        updated_at: Some(Default::default()),
        last_used_at: Some(Default::default()),
    };
    let _ = RBatis::sync(&conn, mapper, &table, "api_key").await;
}

// 用途：初始化AI Hub默认数据
// 说明：创建默认的AI服务提供商、模型和管道配置，确保AI Hub功能能正常使用
pub async fn ai_hub_sync_tables_data(rb: &RBatis) {
    // 用途：获取数据库连接
    // 说明：用于执行数据初始化操作
    let conn = rb.acquire().await.expect("init ai_hub data fail");
    
    // 用途：检查是否已存在默认提供商
    // 说明：避免重复初始化数据
    if let Ok(v) = ProviderConfig::select_by_map(&conn, rbs::value! {"name":"OpenAI"}).await {
        if v.len() > 0 {
            // 用途：如果提供商已存在，直接返回
            // 说明：避免重复初始化数据
            return;
        }
    };
    
    // 用途：插入默认的OpenAI提供商配置
    // 说明：提供基础的AI服务提供商配置
    let openai_provider_id = Ulid::new().to_string();
    let _ = ProviderConfig::insert(
        &conn,
        &ProviderConfig {
            id: Some(openai_provider_id.clone()),
            name: "OpenAI".to_string(),
            provider_type: "openai".to_string(),
            api_base: "https://api.openai.com/v1".to_string(),
            api_key_encrypted: None,
            auth_type: Some("api_key".to_string()),
            auth_config: Some(serde_json::json!({})),
            default_input_price: Some(0.01),
            default_output_price: Some(0.03),
            rate_limit_enabled: Some(true),
            max_concurrent_requests: Some(10),
            circuit_breaker_enabled: Some(true),
            failure_threshold: Some(5),
            timeout_seconds: Some(30),
            enabled: Some(true),
            status: Some("active".to_string()),
            description: Some("OpenAI AI Service Provider".to_string()),
            documentation_url: Some("https://platform.openai.com/docs".to_string()),
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
            name: "GPT-3.5 Turbo".to_string(),
            model_type: "chat".to_string(),
            provider_id: openai_provider_id.clone(),
            api_endpoint: None,
            api_key_encrypted: None,
            input_price: 0.0015,
            output_price: 0.002,
            currency: Some("USD".to_string()),
            max_tokens_per_request: Some(4096),
            max_requests_per_minute: Some(60),
            enabled: Some(true),
            status: Some("active".to_string()),
            description: Some("OpenAI GPT-3.5 Turbo Chat Model".to_string()),
            capabilities: Some(serde_json::json!([
                "streaming",
                "function_calling",
                "json_mode"
            ])),
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
            name: "GPT-3.5 Turbo Instruct".to_string(),
            model_type: "completion".to_string(),
            provider_id: openai_provider_id.clone(),
            api_endpoint: None,
            api_key_encrypted: None,
            input_price: 0.0015,
            output_price: 0.002,
            currency: Some("USD".to_string()),
            max_tokens_per_request: Some(4096),
            max_requests_per_minute: Some(60),
            enabled: Some(true),
            status: Some("active".to_string()),
            description: Some("OpenAI GPT-3.5 Turbo Instruct Model".to_string()),
            capabilities: Some(serde_json::json!([
                "streaming"
            ])),
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