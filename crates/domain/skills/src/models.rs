use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub id: Uuid,
    pub org_id: Option<Uuid>,
    pub created_by: Uuid,
    pub name: String,
    pub description: String,
    pub body: String,
    pub allowed_tools: serde_json::Value,
    pub model: Option<String>,
    pub context: Option<String>,
    pub user_invocable: bool,
    pub model_invocable: bool,
    pub agent_target: Option<serde_json::Value>,
    pub revision: i64,
    pub content_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSkillRequest {
    pub org_id: Option<Uuid>,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    pub model: Option<String>,
    pub context: Option<String>,
    pub user_invocable: Option<bool>,
    pub model_invocable: Option<bool>,
    pub agent_target: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSkillRequest {
    pub description: Option<String>,
    pub body: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
    pub model: Option<Option<String>>,
    pub context: Option<Option<String>>,
    pub user_invocable: Option<bool>,
    pub model_invocable: Option<bool>,
    pub agent_target: Option<Option<serde_json::Value>>,
    /// Reject stale writers rather than silently clobbering an edit made in
    /// another Aura runtime.
    pub expected_revision: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AgentSkillAssignment {
    pub id: Uuid,
    pub skill_id: Uuid,
    pub agent_id: Uuid,
    pub org_id: Option<Uuid>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillScopeQuery {
    pub org_id: Option<Uuid>,
}
