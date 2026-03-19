use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAgent {
    pub id: Uuid,
    pub project_id: Uuid,
    pub org_id: Option<Uuid>,
    pub agent_id: Uuid,
    pub created_by: Uuid,
    pub status: String,
    pub model: Option<String>,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectAgentRequest {
    pub agent_id: Uuid,
    pub org_id: Option<Uuid>,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectAgentStatusRequest {
    pub status: String,
}
