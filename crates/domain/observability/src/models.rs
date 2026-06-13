use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ObservabilityRun {
    pub id: Uuid,
    pub source: String,
    pub environment: String,
    pub external_run_id: Option<String>,
    pub external_run_attempt: i32,
    pub git_sha: Option<String>,
    pub git_branch: Option<String>,
    pub workflow_name: Option<String>,
    pub release_channel: Option<String>,
    pub generated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub overall_status: String,
    pub totals: serde_json::Value,
    pub snapshot: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ObservabilityFeatureResult {
    pub id: Uuid,
    pub run_id: Uuid,
    pub feature_id: String,
    pub label: String,
    pub category: String,
    pub priority: i32,
    pub status: String,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub checks_passed: i32,
    pub checks_total: i32,
    pub checks_failed: i32,
    pub checks_unknown: i32,
    pub latency_p95_ms: Option<i32>,
    pub message: String,
    pub feature: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ObservabilityCheckResult {
    pub id: Uuid,
    pub run_id: Uuid,
    pub feature_id: String,
    pub check_id: String,
    pub required: bool,
    pub status: String,
    pub feature_status: String,
    pub message: String,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub latency_ms: Option<i32>,
    pub evidence: serde_json::Value,
    pub check_payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ObservabilityFailureSummary {
    pub check_id: String,
    pub feature_id: String,
    pub failure_count: i64,
    pub latest_status: String,
    pub latest_message: String,
    pub latest_seen_at: Option<DateTime<Utc>>,
    pub max_latency_ms: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestObservabilityRunRequest {
    pub snapshot: serde_json::Value,
    pub source: Option<String>,
    pub environment: Option<String>,
    pub external_run_id: Option<String>,
    pub external_run_attempt: Option<i32>,
    pub git_sha: Option<String>,
    pub git_branch: Option<String>,
    pub workflow_name: Option<String>,
    pub release_channel: Option<String>,
    pub generated_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub overall_status: Option<String>,
    pub totals: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservabilityHistoryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub source: Option<String>,
    pub environment: Option<String>,
}

impl ObservabilityHistoryQuery {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(50).clamp(1, 500)
    }

    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservabilityFailureQuery {
    pub limit: Option<i64>,
    pub source: Option<String>,
    pub environment: Option<String>,
}

impl ObservabilityFailureQuery {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(50).clamp(1, 500)
    }
}
