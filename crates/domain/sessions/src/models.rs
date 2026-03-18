use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: Uuid,
    pub project_agent_id: Uuid,
    pub project_id: Uuid,
    pub created_by: Uuid,
    pub model: Option<String>,
    pub status: String,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub context_usage: f32,
    pub summary: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionRequest {
    pub project_id: Uuid,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSessionRequest {
    pub status: Option<String>,
    pub total_input_tokens: Option<i64>,
    pub total_output_tokens: Option<i64>,
    pub context_usage: Option<f32>,
    pub summary: Option<String>,
    pub ended_at: Option<DateTime<Utc>>,
}
