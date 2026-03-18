use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: Uuid,
    pub project_id: Uuid,
    pub spec_id: Uuid,
    pub created_by: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub order_index: i32,
    pub dependency_task_ids: serde_json::Value,
    pub parent_task_id: Option<Uuid>,
    pub assigned_project_agent_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub execution_notes: Option<String>,
    pub files_changed: Option<serde_json::Value>,
    pub model: Option<String>,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskRequest {
    pub spec_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub order_index: i32,
    pub dependency_task_ids: Option<Vec<Uuid>>,
    pub parent_task_id: Option<Uuid>,
    pub assigned_project_agent_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub execution_notes: Option<String>,
    pub files_changed: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitionRequest {
    pub status: String,
}
