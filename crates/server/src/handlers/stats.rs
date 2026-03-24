use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use aura_storage_auth::AuthUser;
use aura_storage_core::AppError;

use crate::state::AppState;

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
    pub total_tokens: i64,
    pub total_messages: i64,
    pub total_agents: i64,
    pub total_sessions: i64,
    pub total_time_seconds: f64,
    pub lines_changed: i64,
    pub total_specs: i64,
    pub contributors: i64,
}

pub async fn get_stats(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<StatsQuery>,
) -> Result<Json<ExecutionStats>, AppError> {
    match query.scope.as_str() {
        "project" => {
            let project_id = query.project_id.ok_or_else(|| {
                AppError::BadRequest("projectId is required for scope=project".into())
            })?;
            let stats = query_stats(&state.pool, "project_id", project_id, query.agent_id).await?;
            Ok(Json(stats))
        }
        "org" => {
            let org_id = query
                .org_id
                .ok_or_else(|| AppError::BadRequest("orgId is required for scope=org".into()))?;
            let stats = query_stats(&state.pool, "org_id", org_id, query.agent_id).await?;
            Ok(Json(stats))
        }
        "network" => {
            let stats = query_network_stats(&state.pool, query.agent_id).await?;
            Ok(Json(stats))
        }
        _ => Err(AppError::BadRequest(format!(
            "Invalid scope: '{}'. Must be: project, org, or network",
            query.scope
        ))),
    }
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
    // Tasks use assigned_project_agent_id, sessions/messages use project_agent_id.
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
            COALESCE((SELECT SUM(total_input_tokens + total_output_tokens)::int8 FROM sessions WHERE {col} = $1 {saf}), 0) as total_tokens,
            COALESCE((SELECT COUNT(*) FROM messages WHERE {col} = $1 {saf}), 0) as total_messages,
            COALESCE((SELECT COUNT(*) FROM project_agents WHERE {col} = $1), 0) as total_agents,
            COALESCE((SELECT COUNT(*) FROM sessions WHERE {col} = $1 {saf}), 0) as total_sessions,
            COALESCE((SELECT SUM(EXTRACT(EPOCH FROM (ended_at - started_at)))::float8 FROM sessions WHERE {col} = $1 AND ended_at IS NOT NULL {saf}), 0)::float8 as total_time_seconds,
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
            COALESCE((SELECT SUM(total_input_tokens + total_output_tokens)::int8 FROM sessions WHERE {col} = $1), 0) as total_tokens,
            COALESCE((SELECT COUNT(*) FROM messages WHERE {col} = $1), 0) as total_messages,
            COALESCE((SELECT COUNT(*) FROM project_agents WHERE {col} = $1), 0) as total_agents,
            COALESCE((SELECT COUNT(*) FROM sessions WHERE {col} = $1), 0) as total_sessions,
            COALESCE((SELECT SUM(EXTRACT(EPOCH FROM (ended_at - started_at)))::float8 FROM sessions WHERE {col} = $1 AND ended_at IS NOT NULL), 0)::float8 as total_time_seconds,
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
            COALESCE((SELECT SUM(total_input_tokens + total_output_tokens)::int8 FROM sessions), 0) as total_tokens,
            COALESCE((SELECT COUNT(*) FROM messages), 0) as total_messages,
            COALESCE((SELECT COUNT(*) FROM project_agents), 0) as total_agents,
            COALESCE((SELECT COUNT(*) FROM sessions), 0) as total_sessions,
            COALESCE((SELECT SUM(EXTRACT(EPOCH FROM (ended_at - started_at)))::float8 FROM sessions WHERE ended_at IS NOT NULL), 0)::float8 as total_time_seconds,
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
