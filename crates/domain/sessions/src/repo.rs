use sqlx::PgPool;
use uuid::Uuid;

use aura_storage_core::AppError;

use crate::models::{
    is_valid_public_share_id, CreateSessionRequest, EnrichedSession, Session, UpdateSessionRequest,
};

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
/// (see migration 0015). It can be `NULL` when `include_empty=true`
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

/// User-scoped session listing across every project + agent the
/// user has touched. Powers the chat-app left panel
/// (`apps/chat-app/components/ChatAppLeftPanel/ChatAppLeftPanel.tsx`)
/// which used to fan out one /api/projects/:p/agents/:a/sessions
/// call per (agent, project_binding) pair on first paint. This is
/// a single indexed query against `idx_sessions_user_recent` (see
/// migration 0016), so the panel's first paint is now O(1) HTTP
/// calls instead of O(A x B).
///
/// `LEFT JOIN project_agents` mirrors `list_project_agents`'
/// tolerance for orphan rows: if a binding has been deleted or
/// migrated away under a session, the session still surfaces (with
/// `agent_id = NULL`) so the FE can render it as a non-clickable
/// row instead of vanishing it.
///
/// `include_empty=false` (the chat-app default) is a covering
/// match for the partial index, so the query plan reads only
/// navigable sessions.
pub async fn list_by_user(
    pool: &PgPool,
    user_id: Uuid,
    include_empty: bool,
) -> Result<Vec<EnrichedSession>, AppError> {
    // The session join introduces column-name collisions on `id`,
    // `project_id`, `created_by`, and `org_id` between `sessions`
    // and `project_agents`, which trips up `sqlx::FromRow` with
    // `#[sqlx(flatten)]`. We project the session columns with the
    // `s_` prefix into a flat row struct, then reconstruct
    // `EnrichedSession` in Rust. The agent metadata join is
    // `LEFT JOIN` so deleted bindings still surface the session
    // row (matches `list_by_project_agent` tolerance for orphans).
    let sql = if include_empty {
        "SELECT
             s.id                  AS s_id,
             s.project_agent_id    AS s_project_agent_id,
             s.project_id          AS s_project_id,
             s.org_id              AS s_org_id,
             s.created_by          AS s_created_by,
             s.model               AS s_model,
             s.status              AS s_status,
             s.total_input_tokens  AS s_total_input_tokens,
             s.total_output_tokens AS s_total_output_tokens,
             s.context_usage       AS s_context_usage,
             s.summary             AS s_summary,
             s.started_at          AS s_started_at,
             s.ended_at            AS s_ended_at,
             s.event_count         AS s_event_count,
             s.last_event_at       AS s_last_event_at,
             s.is_public           AS s_is_public,
             s.public_share_id     AS s_public_share_id,
             pa.agent_id           AS pa_agent_id
         FROM sessions s
         LEFT JOIN project_agents pa ON pa.id = s.project_agent_id
         WHERE s.created_by = $1
         ORDER BY s.last_event_at DESC NULLS LAST, s.started_at DESC"
    } else {
        "SELECT
             s.id                  AS s_id,
             s.project_agent_id    AS s_project_agent_id,
             s.project_id          AS s_project_id,
             s.org_id              AS s_org_id,
             s.created_by          AS s_created_by,
             s.model               AS s_model,
             s.status              AS s_status,
             s.total_input_tokens  AS s_total_input_tokens,
             s.total_output_tokens AS s_total_output_tokens,
             s.context_usage       AS s_context_usage,
             s.summary             AS s_summary,
             s.started_at          AS s_started_at,
             s.ended_at            AS s_ended_at,
             s.event_count         AS s_event_count,
             s.last_event_at       AS s_last_event_at,
             s.is_public           AS s_is_public,
             s.public_share_id     AS s_public_share_id,
             pa.agent_id           AS pa_agent_id
         FROM sessions s
         LEFT JOIN project_agents pa ON pa.id = s.project_agent_id
         WHERE s.created_by = $1 AND s.event_count > 0
         ORDER BY s.last_event_at DESC NULLS LAST, s.started_at DESC"
    };

    let rows = sqlx::query_as::<_, EnrichedSessionRow>(sql)
        .bind(user_id)
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(EnrichedSessionRow::into_enriched)
        .collect())
}

#[derive(sqlx::FromRow)]
struct EnrichedSessionRow {
    s_id: Uuid,
    s_project_agent_id: Uuid,
    s_project_id: Uuid,
    s_org_id: Option<Uuid>,
    s_created_by: Uuid,
    s_model: Option<String>,
    s_status: String,
    s_total_input_tokens: i64,
    s_total_output_tokens: i64,
    s_context_usage: f32,
    s_summary: Option<String>,
    s_started_at: chrono::DateTime<chrono::Utc>,
    s_ended_at: Option<chrono::DateTime<chrono::Utc>>,
    s_event_count: i32,
    s_last_event_at: Option<chrono::DateTime<chrono::Utc>>,
    s_is_public: bool,
    s_public_share_id: Option<String>,
    pa_agent_id: Option<Uuid>,
}

impl EnrichedSessionRow {
    fn into_enriched(self) -> EnrichedSession {
        EnrichedSession {
            session: Session {
                id: self.s_id,
                project_agent_id: self.s_project_agent_id,
                project_id: self.s_project_id,
                org_id: self.s_org_id,
                created_by: self.s_created_by,
                model: self.s_model,
                status: self.s_status,
                total_input_tokens: self.s_total_input_tokens,
                total_output_tokens: self.s_total_output_tokens,
                context_usage: self.s_context_usage,
                summary: self.s_summary,
                started_at: self.s_started_at,
                ended_at: self.s_ended_at,
                event_count: self.s_event_count,
                last_event_at: self.s_last_event_at,
                is_public: self.s_is_public,
                public_share_id: self.s_public_share_id,
            },
            agent_id: self.pa_agent_id,
        }
    }
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

    if let Some(ref public_share_id) = input.public_share_id {
        if !is_valid_public_share_id(public_share_id) {
            return Err(AppError::BadRequest("Invalid public share id".into()));
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
            ended_at = COALESCE($7, ended_at),
            is_public = COALESCE($8, is_public),
            public_share_id = COALESCE($9, public_share_id)
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
    .bind(input.is_public)
    .bind(&input.public_share_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Session not found".into()))
}

/// Look up a session by its opaque public share token.
///
/// `public_share_id` is the `t_`-prefixed capability token set when a
/// session is shared (see `UpdateSessionRequest::public_share_id`). The
/// partial unique index `idx_sessions_public_share_id` (migration 0017)
/// makes this an indexed point lookup. Callers MUST validate the token
/// shape at the boundary before calling this. Returns
/// `AppError::NotFound` when no session carries the token.
///
/// Note this does NOT itself gate on `is_public`; the public read path
/// is responsible for rejecting sessions whose flag has been turned off
/// after a token was issued.
pub async fn get_by_share(pool: &PgPool, public_share_id: &str) -> Result<Session, AppError> {
    sqlx::query_as::<_, Session>("SELECT * FROM sessions WHERE public_share_id = $1")
        .bind(public_share_id)
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
