use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: Uuid,
    pub session_id: Uuid,
    pub project_agent_id: Uuid,
    pub project_id: Uuid,
    pub org_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub role: String,
    pub content: String,
    pub content_blocks: Option<serde_json::Value>,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub thinking: Option<String>,
    pub thinking_duration_ms: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageRequest {
    pub project_agent_id: Uuid,
    pub project_id: Uuid,
    pub org_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub role: String,
    pub content: String,
    pub content_blocks: Option<serde_json::Value>,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub thinking: Option<String>,
    pub thinking_duration_ms: Option<i64>,
}
