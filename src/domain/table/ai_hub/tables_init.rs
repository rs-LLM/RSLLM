// 用途：导入AI模型基础信息相关结构体
// 说明：用于初始化模型基础信息表结构
use crate::domain::table::ai_hub::model_base::ModelBase;
// 用途：导入AI模型-供应商映射相关结构体
// 说明：用于初始化模型-供应商映射表结构
use crate::domain::table::ai_hub::model_provider_mapping::ModelProviderMapping;
// 用途：导入AI服务提供商相关结构体
// 说明：用于初始化提供商配置表结构
use crate::domain::table::ai_hub::provider_config::ProviderConfig;
// 用途：导入用量计费相关结构体
// 说明：用于初始化用量记录表结构
use crate::domain::table::ai_hub::usage_log::AiHubUsageLog;
// 用途：导入API密钥相关结构体
// 说明：用于初始化API密钥表结构
use crate::domain::table::ai_hub::api_key::ApiKey;
use crate::domain::table::ai_hub::user_level_config::UserLevelConfig;
use crate::domain::table::ai_hub::user_level_model_rate_limit::UserLevelModelRateLimit;
// 用途：导入交易相关结构体
// 说明：用于初始化交易表结构
use crate::domain::table::ai_hub::transaction::Transaction;
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
// 用途：导入表同步相关的映射器
// 说明：用于根据数据库类型选择合适的表映射器
use rbatis::table_sync::{
    ColumnMapper, MssqlTableMapper, MysqlTableMapper, PGTableMapper, SqliteTableMapper,
};

// 用途：同步AI Hub相关数据库表结构
// 说明：根据表结构定义自动创建或更新AI Hub相关的数据库表
pub async fn ai_hub_sync_tables(rb: &RBatis) {
    // 用途：获取日志拦截器
    // 说明：用于临时关闭日志输出
    let log_intercept = rb
        .get_intercept::<LogInterceptor>()
        .expect("not find log interceptor");
    // 用途：保存当前日志级别
    // 说明：便于后续恢复
    let level = log_intercept.get_level_filter();
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
        provider_code: Default::default(),
        name: Default::default(),
        provider_type: Default::default(),
        api_base: Default::default(),
        api_key_encrypted: Some(Default::default()),
        auth_config: Some(Default::default()),
        circuit_breaker_enabled: Some(Default::default()),
        failure_threshold: Some(Default::default()),
        status: Some(Default::default()),
        description: Some(Default::default()),
        documentation_url: Some(Default::default()),
        created_at: Some(Default::default()),
        updated_at: Some(Default::default()),
    };
    let _ = RBatis::sync(&conn, mapper, &table, "provider_config").await;

    // 用途：同步AI模型基础信息表结构
    // 说明：存储AI模型的通用信息和定价
    let table = ModelBase {
        id: Some(Default::default()),
        model_code: Default::default(),
        name: Default::default(),
        model_type: Default::default(),
        input_price: Default::default(),
        output_price: Default::default(),
        currency: Some(Default::default()),
        max_tokens_per_request: Some(Default::default()),
        max_requests_per_minute: Some(Default::default()),
        description: Some(Default::default()),
        capabilities: Some(Default::default()),
        status: Some(Default::default()),
        image_token_calculation_type: Some(Default::default()),
        patch_multiplier: Some(Default::default()),
        tile_base_tokens: Some(Default::default()),
        tile_tokens_per_tile: Some(Default::default()),
        audio_tokens_per_second: Some(Default::default()),
        created_at: Some(Default::default()),
        updated_at: Some(Default::default()),
    };
    let _ = RBatis::sync(&conn, mapper, &table, "model_base").await;

    // 用途：同步AI模型-供应商映射表结构
    // 说明：存储模型与供应商的映射关系和供应商特定配置
    let table = ModelProviderMapping {
        id: Some(Default::default()),
        model_id: Default::default(),
        provider_id: Default::default(),
        provider_model_name: Default::default(),
        api_endpoint: Some(Default::default()),
        api_key_encrypted: Some(Default::default()),
        priority: Some(Default::default()),
        status: Some(Default::default()),
        created_at: Some(Default::default()),
        updated_at: Some(Default::default()),
    };
    let _ = RBatis::sync(&conn, mapper, &table, "model_provider_mapping").await;

    // 用途：同步用量记录表结构
    // 说明：记录每次AI请求的详细用量信息
    let table = AiHubUsageLog {
        id: Some(Default::default()),
        user_id: Default::default(),
        model_id: Default::default(),
        api_key: Default::default(),
        input_tokens: Default::default(),
        output_tokens: Default::default(),
        total_tokens: Default::default(),
        input_price: Default::default(),
        output_price: Default::default(),
        input_cost: Some(Default::default()),
        output_cost: Some(Default::default()),
        total_cost: Default::default(),
        currency: Some(Default::default()),
        request_method: Some(Default::default()),
        request_path: Some(Default::default()),
        request_headers: Some(Default::default()),
        request_body: Some(Default::default()),
        status_code: Some(Default::default()),
        request_type: Some(Default::default()),
        status: Some(Default::default()),
        request_time: Some(Default::default()),
        response_time: Some(Default::default()),
        response_time_ms: Some(Default::default()),
        error_message: Some(Default::default()),
        extra: Some(Default::default()),
        quota_deducted: Some(Default::default()),
        quota_snapshot: Some(Default::default()),
        ip_address: Some(Default::default()),
        user_agent: Some(Default::default()),
        created_at: Some(Default::default()),
    };
    let _ = RBatis::sync(&conn, mapper, &table, "ai_hub_usage_log").await;

    // 用途：同步API密钥表结构
    // 说明：管理用户的API密钥和权限控制
    let table = ApiKey {
        id: Some(Default::default()),
        key: Default::default(),
        key_hash: Default::default(),
        user_id: Default::default(),
        name: Some(Default::default()),
        description: Some(Default::default()),
        prefix: Some(Default::default()),
        quota_limit: Some(Default::default()),
        quota_period: Some(Default::default()),
        quota_used: Some(Default::default()),
        quota_reset_time: Some(Default::default()),
        last_used_time: Some(Default::default()),
        enabled: Some(Default::default()),
        status: Some(Default::default()),
        rate_limit_enabled: Some(Default::default()),
        max_requests_per_minute: Some(Default::default()),
        max_tokens_per_minute: Some(Default::default()),
        model_rate_limits: Some(Default::default()),
        permissions: Some(Default::default()),
        created_at: Some(Default::default()),
        updated_at: Some(Default::default()),
    };
    let _ = RBatis::sync(&conn, mapper, &table, "api_key").await;

    // 用途：同步交易表结构
    // 说明：记录用户余额的所有变更历史，使用自定义表名"user_transaction"避免SQL保留字冲突
    let table = Transaction {
        id: Some(Default::default()),
        user_id: Default::default(),
        type_: Default::default(),
        amount: Default::default(),
        balance_before: Default::default(),
        balance_after: Default::default(),
        operator_id: Some(Default::default()),
        reason: Default::default(),
        created_at: Some(Default::default()),
    };
    let _ = RBatis::sync(&conn, mapper, &table, "user_transaction").await;

    let table = UserLevelModelRateLimit {
        id: Some(Default::default()),
        user_level: Default::default(),
        model_key: Default::default(),
        rpm_limit: Default::default(),
        tpm_limit: Default::default(),
        enabled: Default::default(),
        created_at: Some(Default::default()),
        updated_at: Some(Default::default()),
    };
    let _ = RBatis::sync(&conn, mapper, &table, "user_level_model_rate_limit").await;

    let table = UserLevelConfig {
        id: Some(Default::default()),
        level: Default::default(),
        level_name: Default::default(),
        description: Some(Default::default()),
        rpm_limit: Default::default(),
        tpm_limit: Default::default(),
        enabled: Default::default(),
        created_at: Some(Default::default()),
        updated_at: Some(Default::default()),
    };
    let _ = RBatis::sync(&conn, mapper, &table, "user_level_config").await;
}

// 用途：初始化AI Hub默认数据
// 说明：创建默认的AI服务提供商、模型和管道配置，确保AI Hub功能能正常使用
pub async fn ai_hub_sync_tables_data(rb: &RBatis) {
    // 用途：获取数据库连接
    // 说明：用于执行数据初始化操作
    let conn = rb.acquire().await.expect("init ai_hub data fail");

    // 用途：检查是否已存在默认提供商
    // 说明：避免重复初始化数据
    if let Ok(v) = ProviderConfig::select_by_map(&conn, rbs::value! {"name":"OpenAI"}).await
        && !v.is_empty()
    {
        // 用途：如果提供商已存在，直接返回
        // 说明：避免重复初始化数据
    }
}
