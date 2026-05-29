use rbatis::crud;
use rbatis::rbdc::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegistrationReview {
    pub id: Option<String>,
    pub name: String,
    pub email: String,
    pub password: String,
    pub agree_terms: bool,
    pub invite_code: Option<String>,
    pub user_level: Option<String>,
    pub apply_reason: Option<String>,
    pub status: String,
    pub review_note: Option<String>,
    pub notify_on_reject: bool,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<DateTime>,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}

crud!(RegistrationReview {}, "registration_review");
