use rbatis::crud;
use rbatis::rbdc::DateTime;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct OfficialAnnouncement {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub enabled: i32,
    pub popup: i32,
    pub sort_order: i32,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}

crud!(OfficialAnnouncement {});
