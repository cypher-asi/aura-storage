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
            let stats = get_project_stats(&state.pool, project_id).await?;
            Ok(Json(stats))
        }
        _ => Err(AppError::BadRequest(format!(
            "Invalid scope: '{}'. Must be: project",
            query.scope
        ))),
    }
}

async fn get_project_stats(
    pool: &sqlx::PgPool,
    project_id: Uuid,
) -> Result<ExecutionStats, AppError> {
    let stats = sqlx::query_as::<_, ExecutionStats>(
        r#"
        SELECT
            COALESCE((SELECT COUNT(*) FROM tasks WHERE project_id = $1), 0) as total_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE project_id = $1 AND status = 'pending'), 0) as pending_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE project_id = $1 AND status = 'ready'), 0) as ready_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE project_id = $1 AND status = 'in_progress'), 0) as in_progress_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE project_id = $1 AND status = 'blocked'), 0) as blocked_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE project_id = $1 AND status = 'done'), 0) as done_tasks,
            COALESCE((SELECT COUNT(*) FROM tasks WHERE project_id = $1 AND status = 'failed'), 0) as failed_tasks,
            CASE
                WHEN (SELECT COUNT(*) FROM tasks WHERE project_id = $1) = 0 THEN 0.0::float8
                ELSE (ROUND(
                    (SELECT COUNT(*) FROM tasks WHERE project_id = $1 AND status = 'done')::numeric /
                    (SELECT COUNT(*) FROM tasks WHERE project_id = $1)::numeric * 100, 1
                ))::float8
            END as completion_percentage,
            COALESCE((SELECT SUM(total_input_tokens + total_output_tokens)::int8 FROM sessions WHERE project_id = $1), 0) as total_tokens,
            COALESCE((SELECT COUNT(*) FROM messages WHERE project_id = $1), 0) as total_messages,
            COALESCE((SELECT COUNT(*) FROM project_agents WHERE project_id = $1), 0) as total_agents,
            COALESCE((SELECT COUNT(*) FROM sessions WHERE project_id = $1), 0) as total_sessions,
            COALESCE((SELECT SUM(EXTRACT(EPOCH FROM (ended_at - started_at)))::float8 FROM sessions WHERE project_id = $1 AND ended_at IS NOT NULL), 0)::float8 as total_time_seconds,
            COALESCE((
                SELECT SUM(
                    COALESCE((elem->>'linesAdded')::bigint, 0) + COALESCE((elem->>'linesRemoved')::bigint, 0)
                )::int8
                FROM tasks
                CROSS JOIN LATERAL jsonb_array_elements(COALESCE(files_changed, '[]'::jsonb)) AS elem
                WHERE tasks.project_id = $1 AND files_changed IS NOT NULL AND jsonb_typeof(files_changed) = 'array'
            ), 0) as lines_changed,
            COALESCE((SELECT COUNT(*) FROM specs WHERE project_id = $1), 0) as total_specs
        "#,
    )
    .bind(project_id)
    .fetch_one(pool)
    .await?;

    Ok(stats)
}
