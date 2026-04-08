use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Entity structs (DB rows)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ProcessFolder {
    pub id: Uuid,
    pub org_id: Uuid,
    pub created_by: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Process {
    pub id: Uuid,
    pub org_id: Uuid,
    pub created_by: Uuid,
    pub project_id: Option<Uuid>,
    pub folder_id: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub schedule: Option<String>,
    pub tags: serde_json::Value,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ProcessNode {
    pub id: Uuid,
    pub process_id: Uuid,
    pub node_type: String,
    pub label: String,
    pub agent_id: Option<Uuid>,
    pub prompt: String,
    pub config: serde_json::Value,
    pub position_x: f64,
    pub position_y: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ProcessNodeConnection {
    pub id: Uuid,
    pub process_id: Uuid,
    pub source_node_id: Uuid,
    pub source_handle: Option<String>,
    pub target_node_id: Uuid,
    pub target_handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ProcessRun {
    pub id: Uuid,
    pub process_id: Uuid,
    pub status: String,
    pub trigger: String,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub total_input_tokens: Option<i64>,
    pub total_output_tokens: Option<i64>,
    pub cost_usd: Option<f64>,
    pub output: Option<String>,
    pub parent_run_id: Option<Uuid>,
    pub input_override: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ProcessEvent {
    pub id: Uuid,
    pub run_id: Uuid,
    pub node_id: Uuid,
    pub process_id: Uuid,
    pub status: String,
    pub input_snapshot: String,
    pub output: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub model: Option<String>,
    pub content_blocks: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ProcessArtifact {
    pub id: Uuid,
    pub process_id: Uuid,
    pub run_id: Uuid,
    pub node_id: Uuid,
    pub artifact_type: String,
    pub name: String,
    pub file_path: String,
    pub size_bytes: i64,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Request structs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProcessRequest {
    pub org_id: Uuid,
    pub project_id: Option<Uuid>,
    pub folder_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub schedule: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProcessRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub project_id: Option<Option<Uuid>>,
    pub folder_id: Option<Option<Uuid>>,
    pub enabled: Option<bool>,
    pub schedule: Option<Option<String>>,
    pub tags: Option<Vec<String>>,
    pub last_run_at: Option<Option<DateTime<Utc>>>,
    pub next_run_at: Option<Option<DateTime<Utc>>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProcessFolderRequest {
    pub org_id: Uuid,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProcessFolderRequest {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProcessNodeRequest {
    pub node_type: String,
    pub label: Option<String>,
    pub agent_id: Option<Uuid>,
    pub prompt: Option<String>,
    pub config: Option<serde_json::Value>,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProcessNodeRequest {
    pub label: Option<String>,
    pub agent_id: Option<Option<Uuid>>,
    pub prompt: Option<String>,
    pub config: Option<serde_json::Value>,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProcessNodeConnectionRequest {
    pub source_node_id: Uuid,
    pub source_handle: Option<String>,
    pub target_node_id: Uuid,
    pub target_handle: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProcessRunRequest {
    pub process_id: Uuid,
    pub trigger: Option<String>,
    pub parent_run_id: Option<Uuid>,
    pub input_override: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProcessRunRequest {
    pub status: Option<String>,
    pub error: Option<Option<String>>,
    pub completed_at: Option<Option<DateTime<Utc>>>,
    pub total_input_tokens: Option<i64>,
    pub total_output_tokens: Option<i64>,
    pub cost_usd: Option<f64>,
    pub output: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProcessEventRequest {
    pub run_id: Uuid,
    pub node_id: Uuid,
    pub process_id: Uuid,
    pub status: Option<String>,
    pub input_snapshot: Option<String>,
    pub output: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProcessEventRequest {
    pub status: Option<String>,
    pub output: Option<String>,
    pub completed_at: Option<Option<DateTime<Utc>>>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub model: Option<String>,
    pub content_blocks: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProcessArtifactRequest {
    pub process_id: Uuid,
    pub run_id: Uuid,
    pub node_id: Uuid,
    pub artifact_type: String,
    pub name: String,
    pub file_path: String,
    pub size_bytes: Option<i64>,
    pub metadata: Option<serde_json::Value>,
}
