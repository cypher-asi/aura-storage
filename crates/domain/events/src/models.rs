use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use aura_storage_core::AppError;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct SessionEvent {
    pub event_id: Uuid,
    pub session_id: Uuid,
    pub user_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub sender: Option<String>,
    pub project_id: Option<Uuid>,
    pub org_id: Option<Uuid>,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub event_type: String,
    pub content: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEventRequest {
    pub session_id: Uuid,
    pub user_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub sender: Option<String>,
    pub project_id: Option<Uuid>,
    pub org_id: Option<Uuid>,
    #[serde(rename = "type")]
    pub event_type: String,
    pub content: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl EventListQuery {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(100).min(500).max(1)
    }

    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

/// Valid event types — validated on insert.
const VALID_EVENT_TYPES: &[&str] = &[
    // Chat / LLM streaming
    "delta",
    "thinking_delta",
    "progress",
    "tool_call_started",
    "tool_call_snapshot",
    "tool_call",
    "tool_result",
    "message_saved",
    "agent_instance_updated",
    "token_usage",
    "done",
    // Spec generation
    "spec_saved",
    "specs_title",
    "specs_summary",
    "spec_gen_started",
    "spec_gen_progress",
    "spec_gen_completed",
    "spec_gen_failed",
    // Task lifecycle
    "task_saved",
    "task_started",
    "task_completed",
    "task_failed",
    "task_retrying",
    "task_became_ready",
    "tasks_became_ready",
    "task_output_delta",
    "follow_up_task_created",
    "file_ops_applied",
    // Loop lifecycle
    "loop_started",
    "loop_paused",
    "loop_stopped",
    "loop_finished",
    "loop_iteration_summary",
    "session_rolled_over",
    // Build / test verification
    "build_verification_skipped",
    "build_verification_started",
    "build_verification_passed",
    "build_verification_failed",
    "build_fix_attempt",
    "test_verification_started",
    "test_verification_passed",
    "test_verification_failed",
    "test_fix_attempt",
    // Git
    "git_committed",
    "git_pushed",
    // Other
    "log_line",
    "network_event",
    "error",
];

pub fn validate_event_type(event_type: &str) -> Result<(), AppError> {
    if VALID_EVENT_TYPES.contains(&event_type) {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!(
            "Invalid event type: '{event_type}'"
        )))
    }
}
