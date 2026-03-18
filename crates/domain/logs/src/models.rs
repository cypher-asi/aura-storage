use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: Uuid,
    pub project_id: Uuid,
    pub project_agent_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub level: String,
    pub message: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLogEntryRequest {
    pub project_agent_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub level: String,
    pub message: String,
    pub metadata: Option<serde_json::Value>,
}
