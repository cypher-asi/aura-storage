use sqlx::PgPool;
use uuid::Uuid;

use aura_storage_core::AppError;

use crate::models::{CreateSessionRequest, Session, UpdateSessionRequest};

const VALID_STATUSES: &[&str] = &["active", "completed", "failed", "rolled_over"];

pub async fn create(
    pool: &PgPool,
    project_agent_id: Uuid,
    created_by: Uuid,
    input: &CreateSessionRequest,
) -> Result<Session, AppError> {
    let session = sqlx::query_as::<_, Session>(
        r#"
        INSERT INTO sessions (project_agent_id, project_id, org_id, created_by, model)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(project_agent_id)
    .bind(input.project_id)
    .bind(input.org_id)
    .bind(created_by)
    .bind(&input.model)
    .fetch_one(pool)
    .await?;

    Ok(session)
}

/// List sessions belonging to a project-agent.
///
/// `include_empty` controls whether orphan zero-event sessions are
/// returned — the chat-app session list always wants `false` (empty
/// rows render as un-clickable "New chat" rows in the sidekick), and
/// the partial index `idx_sessions_pa_recent` is a covering match for
/// that path so the filter is essentially free. Diagnostics tooling
/// can pass `true` to inspect the full table.
///
/// Ordering: most-recently-active first. `last_event_at` is the
/// timestamp of the most recent row in `session_events` for the
/// session, maintained by the `session_events_after_insert` trigger
/// (see migration 0014). It can be `NULL` when `include_empty=true`
/// for sessions that genuinely have no events; we sort `NULLS LAST`
/// and tiebreak on `started_at` so the ordering stays stable.
pub async fn list_by_project_agent(
    pool: &PgPool,
    project_agent_id: Uuid,
    include_empty: bool,
) -> Result<Vec<Session>, AppError> {
    let sql = if include_empty {
        "SELECT * FROM sessions
         WHERE project_agent_id = $1
         ORDER BY last_event_at DESC NULLS LAST, started_at DESC"
    } else {
        "SELECT * FROM sessions
         WHERE project_agent_id = $1 AND event_count > 0
         ORDER BY last_event_at DESC NULLS LAST, started_at DESC"
    };

    let sessions = sqlx::query_as::<_, Session>(sql)
        .bind(project_agent_id)
        .fetch_all(pool)
        .await?;

    Ok(sessions)
}

/// Project-scoped session listing across every project-agent on the
/// project. Replaces the per-agent fan-out aura-os-server used to do:
/// it iterated `list_project_agents` and called
/// `list_by_project_agent` for each, sequentially. This is one
/// indexed query against `idx_sessions_project_recent`.
pub async fn list_by_project(
    pool: &PgPool,
    project_id: Uuid,
    include_empty: bool,
) -> Result<Vec<Session>, AppError> {
    let sql = if include_empty {
        "SELECT * FROM sessions
         WHERE project_id = $1
         ORDER BY last_event_at DESC NULLS LAST, started_at DESC"
    } else {
        "SELECT * FROM sessions
         WHERE project_id = $1 AND event_count > 0
         ORDER BY last_event_at DESC NULLS LAST, started_at DESC"
    };

    let sessions = sqlx::query_as::<_, Session>(sql)
        .bind(project_id)
        .fetch_all(pool)
        .await?;

    Ok(sessions)
}

pub async fn get(pool: &PgPool, id: Uuid) -> Result<Session, AppError> {
    sqlx::query_as::<_, Session>("SELECT * FROM sessions WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Session not found".into()))
}

pub async fn update(
    pool: &PgPool,
    id: Uuid,
    input: &UpdateSessionRequest,
) -> Result<Session, AppError> {
    if let Some(ref status) = input.status {
        if !VALID_STATUSES.contains(&status.as_str()) {
            return Err(AppError::BadRequest(format!(
                "Invalid session status: '{}'. Must be one of: {}",
                status,
                VALID_STATUSES.join(", ")
            )));
        }
    }

    sqlx::query_as::<_, Session>(
        r#"
        UPDATE sessions SET
            status = COALESCE($2, status),
            total_input_tokens = COALESCE($3, total_input_tokens),
            total_output_tokens = COALESCE($4, total_output_tokens),
            context_usage = COALESCE($5, context_usage),
            summary = COALESCE($6, summary),
            ended_at = COALESCE($7, ended_at)
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&input.status)
    .bind(input.total_input_tokens)
    .bind(input.total_output_tokens)
    .bind(input.context_usage)
    .bind(&input.summary)
    .bind(input.ended_at)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Session not found".into()))
}

/// Atomically add token deltas to a session's running totals.
///
/// Called by aura-router on every successful LLM round-trip so token data
/// persists per-call regardless of whether the dev-loop session ever closes
/// cleanly. SET-based writers (`update`) and this delta-based writer must not
/// both run for the same session — by convention the router owns the increment
/// path and the dev loop sends `None` for token fields in `update`.
pub async fn increment_tokens(
    pool: &PgPool,
    id: Uuid,
    input_delta: i64,
    output_delta: i64,
) -> Result<Session, AppError> {
    sqlx::query_as::<_, Session>(
        r#"
        UPDATE sessions SET
            total_input_tokens = total_input_tokens + $2,
            total_output_tokens = total_output_tokens + $3
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(input_delta)
    .bind(output_delta)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Session not found".into()))
}
