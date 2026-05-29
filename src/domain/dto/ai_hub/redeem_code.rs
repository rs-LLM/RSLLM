use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct GenerateRedeemCodesDTO {
    pub amount: f64,
    pub quantity: Option<u32>,
    #[serde(rename = "maxUses")]
    pub max_uses: Option<i64>,
    pub remark: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct QueryRedeemCodesDTO {
    pub code: Option<String>,
    pub status: Option<String>,
    pub page: Option<u64>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateRedeemCodeStatusDTO {
    pub id: String,
    pub status: String,
    pub remark: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct RedeemCodeExchangeDTO {
    pub code: String,
}
