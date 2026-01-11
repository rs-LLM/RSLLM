use rbatis::crud;
use rbatis::rbdc::DateTime;
use serde::{Deserialize, Serialize};
use serde_json;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, Default, ToSchema)]
pub struct ModelBase {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub model_code: String,
    pub name: String,
    pub model_type: String,
    pub input_price: f64,
    pub output_price: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_per_request: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_requests_per_minute: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_token_calculation_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch_multiplier: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tile_base_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tile_tokens_per_tile: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_tokens_per_second: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime>,
}

crud!(ModelBase {});

impl ModelBase {
    pub async fn select_by_model_code(
        rb: &rbatis::RBatis,
        model_code: &str,
    ) -> rbatis::Result<Option<ModelBase>> {
        let sql = "SELECT * FROM model_base WHERE model_code = ? LIMIT 1";
        rb.query(sql, vec![rbs::Value::String(model_code.to_string())])
            .await
            .map(|v| {
                if let Some(arr) = v.as_array()
                    && let Some(item) = arr.first()
                {
                    let json_value = serde_json::to_value(item).unwrap_or_default();
                    let model =
                        serde_json::from_value(json_value).unwrap_or_else(|_| ModelBase::default());
                    return Some(model);
                }
                None
            })
    }

    pub async fn select_by_model_type(
        rb: &rbatis::RBatis,
        model_type: &str,
    ) -> rbatis::Result<Vec<ModelBase>> {
        let sql = "SELECT * FROM model_base WHERE model_type = ? ORDER BY created_at DESC";
        rb.query(sql, vec![rbs::Value::String(model_type.to_string())])
            .await
            .map(|v| {
                v.as_array()
                    .map(|arr| {
                        arr.iter()
                            .map(|item| {
                                let json_value = serde_json::to_value(item).unwrap_or_default();
                                serde_json::from_value(json_value)
                                    .unwrap_or_else(|_| ModelBase::default())
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            })
    }

    pub async fn select_active_page(
        rb: &rbatis::RBatis,
        page: u64,
        size: u64,
    ) -> rbatis::Result<Vec<ModelBase>> {
        let sql = "SELECT * FROM model_base WHERE status = 'active' LIMIT ? OFFSET ?";
        rb.query(
            sql,
            vec![
                rbs::Value::I64(size as i64),
                rbs::Value::I64(((page - 1) * size) as i64),
            ],
        )
        .await
        .map(|v| {
            v.as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|item| {
                            let json_value = serde_json::to_value(item).unwrap_or_default();
                            serde_json::from_value(json_value)
                                .unwrap_or_else(|_| ModelBase::default())
                        })
                        .collect()
                })
                .unwrap_or_default()
        })
    }

    pub async fn select_page(
        rb: &rbatis::RBatis,
        page: u64,
        size: u64,
    ) -> rbatis::Result<Vec<ModelBase>> {
        let sql = "SELECT * FROM model_base ORDER BY created_at DESC LIMIT ? OFFSET ?";
        rb.query(
            sql,
            vec![
                rbs::Value::I64(size as i64),
                rbs::Value::I64(((page - 1) * size) as i64),
            ],
        )
        .await
        .map(|v| {
            v.as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|item| {
                            let json_value = serde_json::to_value(item).unwrap_or_default();
                            serde_json::from_value(json_value)
                                .unwrap_or_else(|_| ModelBase::default())
                        })
                        .collect()
                })
                .unwrap_or_default()
        })
    }

    pub async fn search_page(
        rb: &rbatis::RBatis,
        search: &str,
        page: u64,
        size: u64,
    ) -> rbatis::Result<Vec<ModelBase>> {
        let sql = "SELECT * FROM model_base WHERE name LIKE ? OR model_code LIKE ? ORDER BY created_at DESC LIMIT ? OFFSET ?";
        let search_pattern = format!("%{}%", search);
        rb.query(
            sql,
            vec![
                rbs::Value::String(search_pattern.clone()),
                rbs::Value::String(search_pattern),
                rbs::Value::I64(size as i64),
                rbs::Value::I64(((page - 1) * size) as i64),
            ],
        )
        .await
        .map(|v| {
            v.as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|item| {
                            let json_value = serde_json::to_value(item).unwrap_or_default();
                            serde_json::from_value(json_value)
                                .unwrap_or_else(|_| ModelBase::default())
                        })
                        .collect()
                })
                .unwrap_or_default()
        })
    }

    pub async fn count_all(rb: &rbatis::RBatis) -> rbatis::Result<i64> {
        let sql = "SELECT COUNT(*) as count FROM model_base";
        rb.query(sql, vec![]).await.map(|v| {
            if let Some(arr) = v.as_array()
                && let Some(item) = arr.first()
            {
                let json_value = serde_json::to_value(item).unwrap_or_default();
                if let Some(count) = json_value.get("count") {
                    count.as_i64().unwrap_or(0)
                } else {
                    0
                }
            } else {
                0
            }
        })
    }

    pub async fn count_search(rb: &rbatis::RBatis, search: &str) -> rbatis::Result<i64> {
        let sql = "SELECT COUNT(*) as count FROM model_base WHERE name LIKE ? OR model_code LIKE ?";
        let search_pattern = format!("%{}%", search);
        rb.query(
            sql,
            vec![
                rbs::Value::String(search_pattern.clone()),
                rbs::Value::String(search_pattern),
            ],
        )
        .await
        .map(|v| {
            if let Some(arr) = v.as_array()
                && let Some(item) = arr.first()
            {
                let json_value = serde_json::to_value(item).unwrap_or_default();
                if let Some(count) = json_value.get("count") {
                    count.as_i64().unwrap_or(0)
                } else {
                    0
                }
            } else {
                0
            }
        })
    }
}
