// 供应商模块测试用例
use rsllm::domain::dto::{ProviderConfig, ProviderType};
use rsllm::domain::table::provider::Provider as DbProvider;
use rsllm::providers::registry::ProviderRegistry;

// 测试ProviderType枚举的转换功能
#[test]
fn test_provider_type_conversion() {
    // 测试从字符串转换为ProviderType
    assert_eq!(ProviderType::from_str("openai"), Some(ProviderType::OpenAI));
    assert_eq!(ProviderType::from_str("anthropic"), Some(ProviderType::Anthropic));
    assert_eq!(ProviderType::from_str("azure"), Some(ProviderType::Azure));
    assert_eq!(ProviderType::from_str("bedrock"), Some(ProviderType::Bedrock));
    assert_eq!(ProviderType::from_str("vertexai"), Some(ProviderType::VertexAI));
    assert_eq!(ProviderType::from_str("custom"), Some(ProviderType::Custom));
    assert_eq!(ProviderType::from_str("invalid"), None);
    
    // 测试从ProviderType转换为字符串
    assert_eq!(ProviderType::OpenAI.to_string(), "openai");
    assert_eq!(ProviderType::Anthropic.to_string(), "anthropic");
    assert_eq!(ProviderType::Azure.to_string(), "azure");
    assert_eq!(ProviderType::Bedrock.to_string(), "bedrock");
    assert_eq!(ProviderType::VertexAI.to_string(), "vertexai");
    assert_eq!(ProviderType::Custom.to_string(), "custom");
}

// 测试从DbProvider转换为ProviderConfig
#[test]
fn test_provider_config_from_db() {
    // 创建一个DbProvider实例
    let db_provider = DbProvider {
        id: Some("test-id".to_string()),
        name: "Test Provider".to_string(),
        provider_type: "openai".to_string(),
        config_details: serde_json::json!({"api_key": "test-key", "base_url": "https://api.test.com/v1"}),
        enabled: Some(true),
        base_price: Some(0.0),
        context_price: Some(0.0),
        output_price: Some(0.0),
        created_at: None,
        updated_at: None,
    };
    
    // 转换为ProviderConfig
    let provider_config = ProviderConfig::from(&db_provider);
    
    // 验证转换结果
    assert_eq!(provider_config.id, "test-id");
    assert_eq!(provider_config.name, "Test Provider");
    assert_eq!(provider_config.provider_type, ProviderType::OpenAI);
    assert_eq!(provider_config.config_details, serde_json::json!({"api_key": "test-key", "base_url": "https://api.test.com/v1"}));
}

// 测试无效ProviderType的处理
#[test]
fn test_invalid_provider_type() {
    // 创建一个带有无效provider_type的DbProvider实例
    let db_provider = DbProvider {
        id: Some("test-id".to_string()),
        name: "Test Provider".to_string(),
        provider_type: "invalid-type".to_string(),
        config_details: serde_json::json!({"api_key": "test-key"}),
        enabled: Some(true),
        base_price: Some(0.0),
        context_price: Some(0.0),
        output_price: Some(0.0),
        created_at: None,
        updated_at: None,
    };
    
    // 转换为ProviderConfig，应该使用默认的Custom类型
    let provider_config = ProviderConfig::from(&db_provider);
    assert_eq!(provider_config.provider_type, ProviderType::Custom);
}

// 测试供应商注册表的初始化
#[test]
fn test_provider_registry_init() {
    // 创建一个空的注册表
    let registry = ProviderRegistry::new();
    assert_eq!(registry.get_all_providers().len(), 0);
    
    // 创建一个带有供应商的注册表
    let db_providers = vec![
        DbProvider {
            id: Some("provider-1".to_string()),
            name: "OpenAI".to_string(),
            provider_type: "openai".to_string(),
            config_details: serde_json::json!({"api_key": "test-key-1"}),
            enabled: Some(true),
            base_price: Some(0.0),
            context_price: Some(0.0),
            output_price: Some(0.0),
            created_at: None,
            updated_at: None,
        },
        DbProvider {
            id: Some("provider-2".to_string()),
            name: "Anthropic".to_string(),
            provider_type: "anthropic".to_string(),
            config_details: serde_json::json!({"api_key": "test-key-2"}),
            enabled: Some(true),
            base_price: Some(0.0),
            context_price: Some(0.0),
            output_price: Some(0.0),
            created_at: None,
            updated_at: None,
        },
    ];
    
    let registry = ProviderRegistry::from_db(&db_providers);
    assert_eq!(registry.get_all_providers().len(), 2);
    
    // 验证可以获取到供应商
    assert!(registry.get_provider("provider-1").is_some());
    assert!(registry.get_provider("provider-2").is_some());
    assert!(registry.get_provider("provider-3").is_none());
}
