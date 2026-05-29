use rbatis::crud;
use rbs::Value;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, Default, ToSchema)]
pub struct ModelProviderMapping {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub model_id: String,
    pub provider_id: String,
    pub provider_model_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_encrypted: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

crud!(ModelProviderMapping {});

impl ModelProviderMapping {
    pub fn deduplicate_by_priority(
        mappings: Vec<ModelProviderMapping>,
    ) -> Vec<ModelProviderMapping> {
        use std::collections::HashMap;

        let mut provider_model_map: HashMap<(String, String), ModelProviderMapping> =
            HashMap::new();

        for mapping in mappings {
            let key = (mapping.model_id.clone(), mapping.provider_id.clone());
            let priority = mapping.priority.unwrap_or(10);

            if let Some(existing) = provider_model_map.get(&key) {
                let existing_priority = existing.priority.unwrap_or(10);
                if priority > existing_priority {
                    provider_model_map.insert(key, mapping);
                }
            } else {
                provider_model_map.insert(key, mapping);
            }
        }

        provider_model_map.into_values().collect()
    }

    pub async fn select_by_model_id(
        rb: &rbatis::RBatis,
        model_id: &str,
    ) -> rbatis::Result<Vec<ModelProviderMapping>> {
        let sql = "SELECT * FROM model_provider_mapping WHERE model_id = ?";
        rb.query(sql, vec![Value::String(model_id.to_string())])
            .await
            .map(|v| {
                if let Some(arr) = v.as_array() {
                    let mappings: Vec<ModelProviderMapping> = arr
                        .iter()
                        .filter_map(|item| {
                            let json_value = serde_json::to_value(item).unwrap_or_default();
                            serde_json::from_value(json_value).ok()
                        })
                        .collect();
                    return mappings;
                }
                Vec::new()
            })
    }

    pub async fn select_by_provider_id(
        rb: &rbatis::RBatis,
        provider_id: &str,
    ) -> rbatis::Result<Vec<ModelProviderMapping>> {
        let sql = "SELECT * FROM model_provider_mapping WHERE provider_id = ?";
        rb.query(sql, vec![Value::String(provider_id.to_string())])
            .await
            .map(|v| {
                if let Some(arr) = v.as_array() {
                    let mappings: Vec<ModelProviderMapping> = arr
                        .iter()
                        .filter_map(|item| {
                            let json_value = serde_json::to_value(item).unwrap_or_default();
                            serde_json::from_value(json_value).ok()
                        })
                        .collect();
                    return mappings;
                }
                Vec::new()
            })
    }

    pub async fn select_by_model_and_provider(
        rb: &rbatis::RBatis,
        model_id: &str,
        provider_id: &str,
    ) -> rbatis::Result<Option<ModelProviderMapping>> {
        let sql = "SELECT * FROM model_provider_mapping WHERE model_id = ? AND provider_id = ? ORDER BY priority DESC LIMIT 1";
        rb.query(
            sql,
            vec![
                Value::String(model_id.to_string()),
                Value::String(provider_id.to_string()),
            ],
        )
        .await
        .map(|v| {
            if let Some(arr) = v.as_array()
                && let Some(item) = arr.first()
            {
                let json_value = serde_json::to_value(item).unwrap_or_default();
                let mapping = serde_json::from_value(json_value)
                    .unwrap_or_else(|_| ModelProviderMapping::default());
                return Some(mapping);
            }
            None
        })
    }

    pub async fn select_by_model_and_provider_all(
        rb: &rbatis::RBatis,
        model_id: &str,
        provider_id: &str,
    ) -> rbatis::Result<Vec<ModelProviderMapping>> {
        let sql = "SELECT * FROM model_provider_mapping WHERE model_id = ? AND provider_id = ? ORDER BY priority DESC";
        rb.query(
            sql,
            vec![
                Value::String(model_id.to_string()),
                Value::String(provider_id.to_string()),
            ],
        )
        .await
        .map(|v| {
            if let Some(arr) = v.as_array() {
                let mappings: Vec<ModelProviderMapping> = arr
                    .iter()
                    .filter_map(|item| {
                        let json_value = serde_json::to_value(item).unwrap_or_default();
                        serde_json::from_value(json_value).ok()
                    })
                    .collect();
                return mappings;
            }
            Vec::new()
        })
    }
}
