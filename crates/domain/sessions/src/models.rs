use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: Uuid,
    pub project_agent_id: Uuid,
    pub project_id: Uuid,
    pub org_id: Option<Uuid>,
    pub created_by: Uuid,
    pub model: Option<String>,
    pub status: String,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub context_usage: f32,
    pub summary: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    /// Maintained by the `session_events_after_insert` trigger on
    /// `session_events`. Lets the chat-app session list filter empty
    /// orphan rows in a single indexed `WHERE event_count > 0` instead
    /// of fanning out one `list_events?limit=1` probe per session from
    /// aura-os-server (see migration 0014).
    pub event_count: i32,
    /// Timestamp of the most recent `session_events` row for this
    /// session. Used as the primary sort key on the chat-app session
    /// list so the most recently-active sessions float to the top
    /// regardless of when the row was first created.
    pub last_event_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionRequest {
    pub project_id: Uuid,
    pub org_id: Option<Uuid>,
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
