use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Spec {
    pub id: Uuid,
    pub project_id: Uuid,
    pub created_by: Uuid,
    pub title: String,
    pub order_index: i32,
    pub markdown_contents: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSpecRequest {
    pub title: String,
    pub order_index: i32,
    pub markdown_contents: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSpecRequest {
    pub title: Option<String>,
    pub order_index: Option<i32>,
    pub markdown_contents: Option<String>,
}
