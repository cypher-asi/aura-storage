use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub id: Uuid,
    pub project_id: Uuid,
    pub org_id: Option<Uuid>,
    pub created_by: Uuid,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub artifact_type: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub asset_url: String,
    pub thumbnail_url: Option<String>,
    pub original_url: Option<String>,
    pub parent_id: Option<Uuid>,
    pub is_iteration: bool,
    pub prompt: Option<String>,
    pub prompt_mode: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub meta: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateArtifactRequest {
    pub org_id: Option<Uuid>,
    #[serde(rename = "type")]
    pub artifact_type: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub asset_url: String,
    pub thumbnail_url: Option<String>,
    pub original_url: Option<String>,
    pub parent_id: Option<Uuid>,
    #[serde(default)]
    pub is_iteration: bool,
    pub prompt: Option<String>,
    pub prompt_mode: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub meta: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactListQuery {
    #[serde(rename = "type")]
    pub artifact_type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl ArtifactListQuery {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(50).min(100).max(1)
    }

    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}
