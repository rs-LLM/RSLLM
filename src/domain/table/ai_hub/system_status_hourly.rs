use rbatis::crud;
use rbatis::rbdc::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemStatusHourly {
    pub id: Option<String>,
    pub service_id: String,
    pub hour_ts: String,
    pub total_samples: i64,
    pub success_samples: i64,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}

crud!(SystemStatusHourly {});
