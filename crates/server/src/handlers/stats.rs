use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use aura_storage_auth::AuthUser;
use aura_storage_core::AppError;

use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageCostResponse {
    #[allow(dead_code)]
    total_input_tokens: i64,
    #[allow(dead_code)]
    total_output_tokens: i64,
    #[allow(dead_code)]
    total_tokens: i64,
    total_cost_usd: f64,
    /// Sum of per-call inference duration ("model time"). Optional because
    /// older aura-network deployments do not return it.
    #[serde(default)]
    total_duration_ms: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsQuery {
    pub scope: String,
    pub project_id: Option<Uuid>,
    pub org_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionStats {
    pub total_tasks: i64,
    pub pending_tasks: i64,
    pub ready_tasks: i64,
    pub in_progress_tasks: i64,
    pub blocked_tasks: i64,
    pub done_tasks: i64,
    pub failed_tasks: i64,
    pub completion_percentage: f64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_tokens: i64,
    pub total_events: i64,
    pub total_agents: i64,
    pub total_sessions: i64,
    pub total_time_seconds: f64,
    /// Sum of per-task wall-clock durations (ended_at - started_at). For tasks
    /// still in progress, ended_at is coalesced to NOW(). Distinct from
    /// `total_time_seconds` (session-based) — both are exposed; UI chooses.
    pub total_task_time_seconds: f64,
    pub lines_changed: i64,
    pub total_specs: i64,
    pub contributors: i64,
    #[sqlx(skip)]
    pub estimated_cost_usd: f64,
    /// Sum of per-LLM-call inference duration, sourced from aura-network's
    /// usage endpoint. 0.0 when aura-network is unreachable or hasn't been
    /// updated to return `totalDurationMs` yet.
    #[sqlx(skip)]
    pub total_model_time_seconds: f64,
}

pub async fn get_stats(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<StatsQuery>,
) -> Result<Json<ExecutionStats>, AppError> {
    get_stats_inner(
        &state.pool,
        &state.http_client,
        state.aura_network_url.as_deref(),
        state.aura_network_token.as_deref(),
        query,
    )
    .await
}

pub async fn get_stats_inner(
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    network_url: Option<&str>,
    network_token: Option<&str>,
    query: StatsQuery,
) -> Result<Json<ExecutionStats>, AppError> {
    let (mut stats, cost_path) = match query.scope.as_str() {
        "project" => {
            let project_id = query.project_id.ok_or_else(|| {
                AppError::BadRequest("projectId is required for scope=project".into())
            })?;
            let stats = query_stats(pool, "project_id", project_id, query.agent_id).await?;
            (stats, format!("/internal/projects/{project_id}/usage"))
        }
        "org" => {
            let org_id = query
                .org_id
                .ok_or_else(|| AppError::BadRequest("orgId is required for scope=org".into()))?;
            let stats = query_stats(pool, "org_id", org_id, query.agent_id).await?;
            (stats, format!("/internal/orgs/{org_id}/usage"))
        }
        "network" => {
            let stats = query_network_stats(pool, query.agent_id).await?;
            (stats, "/internal/usage/network".to_string())
        }
        _ => {
            return Err(AppError::BadRequest(format!(
                "Invalid scope: '{}'. Must be: project, org, or network",
                query.scope
            )));
        }
    };

    // Fetch cost + model time from aura-network if configured. Both fields are
    // best-effort: if aura-network is unreachable they stay at their zero
    // initialization and the rest of the stats response remains valid.
    if let (Some(url), Some(token)) = (network_url, network_token) {
        if let Ok(resp) = http_client
            .get(format!("{url}{cost_path}"))
            .header("x-internal-token", token)
            .send()
            .await
        {
            if let Ok(usage) = resp.json::<UsageCostResponse>().await {
                stats.estimated_cost_usd = usage.total_cost_usd;
                if let Some(ms) = usage.total_duration_ms {
                    stats.total_model_time_seconds = ms as f64 / 1000.0;
                }
            }
        }
    }

    Ok(Json(stats))
}

/// Query stats scoped by a column (project_id or org_id), with optional agent filter.
async fn query_stats(
    pool: &sqlx::PgPool,
    scope_column: &str,
    scope_id: Uuid,
    agent_id: Option<Uuid>,
) -> Result<ExecutionStats, AppError> {
    // If no agent filter, use the simpler query without $2 binds.
    if agent_id.is_none() {
        return query_stats_unfiltered(pool, scope_column, scope_id).await;
    }

    let aid = agent_id.unwrap();

    // Build query with agent filter. $1 = scope_id, $2 = agent_id.
    // Safe: scope_column is only ever "project_id" or "org_id" from our match.
    // Tasks use assigned_project_agent_id, sessions/events use project_agent_id.
    let taf =
        "AND assigned_project_agent_id IN (SELECT id FROM project_agents WHERE agent_id = $2)";
    let saf = "AND project_agent_id IN (SELECT id FROM project_agents WHERE agent_id = $2)";

    let sql = format!(
        r#"
        SELECT
            COALESCE((SELECT COUNT(*) FROM tasks WHERE {col} = $1 {taf}), 0) as total_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE {col} = $1 AND status = 'pending' {taf}), 0) as pending_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE {col} = $1 AND status = 'ready' {taf}), 0) as ready_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE {col} = $1 AND status = 'in_progress' {taf}), 0) as in_progress_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE {col} = $1 AND status = 'blocked' {taf}), 0) as blocked_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE {col} = $1 AND status = 'done' {taf}), 0) as done_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE {col} = $1 AND status = 'failed' {taf}), 0) as failed_tasks,
            CASE
                WHEN (SELECT COUNT(*) FROM tasks WHERE {col} = $1 {taf}) = 0 THEN 0.0::float8
                ELSE (ROUND(
                    (SELECT COUNT(*) FROM tasks WHERE {col} = $1 AND status = 'done' {taf})::numeric /
                    (SELECT COUNT(*) FROM tasks WHERE {col} = $1 {taf})::numeric * 100, 1
                ))::float8
            END as completion_percentage,
            -- Tokens are now sourced from `tasks` (where aura-os-server's
            -- `persist_task_output` lands them on task termination), not
            -- `sessions`. The dev-loop architecture stopped writing token
            -- totals to `sessions.total_input_tokens` after the
            -- task_output_cache refactor, so reading from sessions returned
            -- 0 even for active projects. The `tasks` source matches what
            -- the dev-loop writes today.
            COALESCE((SELECT SUM(total_input_tokens)::int8 FROM tasks WHERE {col} = $1 {taf}), 0) as total_input_tokens,
            COALESCE((SELECT SUM(total_output_tokens)::int8 FROM tasks WHERE {col} = $1 {taf}), 0) as total_output_tokens,
            COALESCE((SELECT SUM(total_input_tokens + total_output_tokens)::int8 FROM tasks WHERE {col} = $1 {taf}), 0) as total_tokens,
            COALESCE((SELECT COUNT(*) FROM session_events WHERE {col} = $1 {saf}), 0) as total_events,
            COALESCE((SELECT COUNT(*) FROM project_agents WHERE {col} = $1), 0) as total_agents,
            COALESCE((SELECT COUNT(*) FROM sessions WHERE {col} = $1 {saf}), 0) as total_sessions,
            COALESCE((SELECT SUM(EXTRACT(EPOCH FROM (ended_at - started_at)))::float8 FROM sessions WHERE {col} = $1 AND ended_at IS NOT NULL {saf}), 0)::float8 as total_time_seconds,
            COALESCE((SELECT SUM(EXTRACT(EPOCH FROM (COALESCE(ended_at, NOW()) - started_at)))::float8 FROM tasks WHERE {col} = $1 AND started_at IS NOT NULL {taf}), 0)::float8 as total_task_time_seconds,
            COALESCE((
                SELECT SUM(
                    COALESCE((elem->>'linesAdded')::bigint, 0) + COALESCE((elem->>'linesRemoved')::bigint, 0)
                )::int8
                FROM tasks
                CROSS JOIN LATERAL jsonb_array_elements(COALESCE(files_changed, '[]'::jsonb)) AS elem
                WHERE tasks.{col} = $1 AND files_changed IS NOT NULL AND jsonb_typeof(files_changed) = 'array' {taf}
            ), 0) as lines_changed,
            COALESCE((SELECT COUNT(*) FROM specs WHERE {col} = $1), 0) as total_specs,
            COALESCE((SELECT COUNT(DISTINCT created_by) FROM sessions WHERE {col} = $1 {saf}), 0) as contributors
        "#,
        col = scope_column,
        taf = taf,
        saf = saf,
    );

    let stats = sqlx::query_as::<_, ExecutionStats>(&sql)
        .bind(scope_id)
        .bind(aid)
        .fetch_one(pool)
        .await?;

    Ok(stats)
}

/// Query stats without agent filter (original query).
async fn query_stats_unfiltered(
    pool: &sqlx::PgPool,
    scope_column: &str,
    scope_id: Uuid,
) -> Result<ExecutionStats, AppError> {
    let sql = format!(
        r#"
        SELECT
            COALESCE((SELECT COUNT(*) FROM tasks WHERE {col} = $1), 0) as total_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE {col} = $1 AND status = 'pending'), 0) as pending_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE {col} = $1 AND status = 'ready'), 0) as ready_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE {col} = $1 AND status = 'in_progress'), 0) as in_progress_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE {col} = $1 AND status = 'blocked'), 0) as blocked_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE {col} = $1 AND status = 'done'), 0) as done_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE {col} = $1 AND status = 'failed'), 0) as failed_tasks,
            CASE
                WHEN (SELECT COUNT(*) FROM tasks WHERE {col} = $1) = 0 THEN 0.0::float8
                ELSE (ROUND(
                    (SELECT COUNT(*) FROM tasks WHERE {col} = $1 AND status = 'done')::numeric /
                    (SELECT COUNT(*) FROM tasks WHERE {col} = $1)::numeric * 100, 1
                ))::float8
            END as completion_percentage,
            -- Tokens sourced from `tasks` where aura-os-server's
            -- `persist_task_output` lands them; see filtered variant above.
            COALESCE((SELECT SUM(total_input_tokens)::int8 FROM tasks WHERE {col} = $1), 0) as total_input_tokens,
            COALESCE((SELECT SUM(total_output_tokens)::int8 FROM tasks WHERE {col} = $1), 0) as total_output_tokens,
            COALESCE((SELECT SUM(total_input_tokens + total_output_tokens)::int8 FROM tasks WHERE {col} = $1), 0) as total_tokens,
            COALESCE((SELECT COUNT(*) FROM session_events WHERE {col} = $1), 0) as total_events,
            COALESCE((SELECT COUNT(*) FROM project_agents WHERE {col} = $1), 0) as total_agents,
            COALESCE((SELECT COUNT(*) FROM sessions WHERE {col} = $1), 0) as total_sessions,
            COALESCE((SELECT SUM(EXTRACT(EPOCH FROM (ended_at - started_at)))::float8 FROM sessions WHERE {col} = $1 AND ended_at IS NOT NULL), 0)::float8 as total_time_seconds,
            COALESCE((SELECT SUM(EXTRACT(EPOCH FROM (COALESCE(ended_at, NOW()) - started_at)))::float8 FROM tasks WHERE {col} = $1 AND started_at IS NOT NULL), 0)::float8 as total_task_time_seconds,
            COALESCE((
                SELECT SUM(
                    COALESCE((elem->>'linesAdded')::bigint, 0) + COALESCE((elem->>'linesRemoved')::bigint, 0)
                )::int8
                FROM tasks
                CROSS JOIN LATERAL jsonb_array_elements(COALESCE(files_changed, '[]'::jsonb)) AS elem
                WHERE tasks.{col} = $1 AND files_changed IS NOT NULL AND jsonb_typeof(files_changed) = 'array'
            ), 0) as lines_changed,
            COALESCE((SELECT COUNT(*) FROM specs WHERE {col} = $1), 0) as total_specs,
            COALESCE((SELECT COUNT(DISTINCT created_by) FROM sessions WHERE {col} = $1), 0) as contributors
        "#,
        col = scope_column
    );

    let stats = sqlx::query_as::<_, ExecutionStats>(&sql)
        .bind(scope_id)
        .fetch_one(pool)
        .await?;

    Ok(stats)
}

/// Query stats across the entire network (no scope filter).
async fn query_network_stats(
    pool: &sqlx::PgPool,
    _agent_id: Option<Uuid>,
) -> Result<ExecutionStats, AppError> {
    let stats = sqlx::query_as::<_, ExecutionStats>(
        r#"
        SELECT
            COALESCE((SELECT COUNT(*) FROM tasks), 0) as total_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE status = 'pending'), 0) as pending_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE status = 'ready'), 0) as ready_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE status = 'in_progress'), 0) as in_progress_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE status = 'blocked'), 0) as blocked_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE status = 'done'), 0) as done_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE status = 'failed'), 0) as failed_tasks,
            CASE
                WHEN (SELECT COUNT(*) FROM tasks) = 0 THEN 0.0::float8
                ELSE (ROUND(
                    (SELECT COUNT(*) FROM tasks WHERE status = 'done')::numeric /
                    (SELECT COUNT(*) FROM tasks)::numeric * 100, 1
                ))::float8
            END as completion_percentage,
            -- Tokens sourced from `tasks`; sessions tokens are not written
            -- under the current aura-os-server architecture.
            COALESCE((SELECT SUM(total_input_tokens)::int8 FROM tasks), 0) as total_input_tokens,
            COALESCE((SELECT SUM(total_output_tokens)::int8 FROM tasks), 0) as total_output_tokens,
            COALESCE((SELECT SUM(total_input_tokens + total_output_tokens)::int8 FROM tasks), 0) as total_tokens,
            COALESCE((SELECT COUNT(*) FROM session_events), 0) as total_events,
            COALESCE((SELECT COUNT(*) FROM project_agents), 0) as total_agents,
            COALESCE((SELECT COUNT(*) FROM sessions), 0) as total_sessions,
            COALESCE((SELECT SUM(EXTRACT(EPOCH FROM (ended_at - started_at)))::float8 FROM sessions WHERE ended_at IS NOT NULL), 0)::float8 as total_time_seconds,
            COALESCE((SELECT SUM(EXTRACT(EPOCH FROM (COALESCE(ended_at, NOW()) - started_at)))::float8 FROM tasks WHERE started_at IS NOT NULL), 0)::float8 as total_task_time_seconds,
            COALESCE((
                SELECT SUM(
                    COALESCE((elem->>'linesAdded')::bigint, 0) + COALESCE((elem->>'linesRemoved')::bigint, 0)
                )::int8
                FROM tasks
                CROSS JOIN LATERAL jsonb_array_elements(COALESCE(files_changed, '[]'::jsonb)) AS elem
                WHERE files_changed IS NOT NULL AND jsonb_typeof(files_changed) = 'array'
            ), 0) as lines_changed,
            COALESCE((SELECT COUNT(*) FROM specs), 0) as total_specs,
            COALESCE((SELECT COUNT(DISTINCT created_by) FROM sessions), 0) as contributors
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok(stats)
}
