use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use aura_storage_auth::InternalAuth;
use aura_storage_core::AppError;
use aura_storage_events::{models as event_models, repo as event_repo};
use aura_storage_logs::{models as log_models, repo as log_repo};
use aura_storage_project_agents::{models as pa_models, repo as pa_repo};
use aura_storage_sessions::{models as session_models, repo as session_repo};
use aura_storage_specs::{models as spec_models, repo as spec_repo};
use aura_storage_tasks::{models as task_models, repo as task_repo};
use aura_storage_artifacts::{models as artifact_models, repo as artifact_repo};
use aura_storage_processes::{models as process_models, repo as process_repo};

use crate::state::AppState;

// ============================================================================
// Request types (internal endpoints include fields derived from JWT in public API)
// ============================================================================

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalCreateSessionRequest {
    pub project_agent_id: Uuid,
    pub project_id: Uuid,
    pub org_id: Option<Uuid>,
    pub created_by: Uuid,
    pub model: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalCreateLogRequest {
    pub project_id: Uuid,
    pub org_id: Option<Uuid>,
    pub project_agent_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub level: String,
    pub message: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalCreateSpecRequest {
    pub project_id: Uuid,
    pub created_by: Uuid,
    #[serde(flatten)]
    pub spec: spec_models::CreateSpecRequest,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalCreateTaskRequest {
    pub project_id: Uuid,
    pub created_by: Uuid,
    #[serde(flatten)]
    pub task: task_models::CreateTaskRequest,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalCreateProjectAgentRequest {
    pub project_id: Uuid,
    pub created_by: Uuid,
    #[serde(flatten)]
    pub agent: pa_models::CreateProjectAgentRequest,
}

#[derive(Debug, serde::Deserialize)]
pub struct TaskListQuery {
    pub status: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct LogListQuery {
    pub level: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================================================
// Sessions
// ============================================================================

pub async fn create_session(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Json(input): Json<InternalCreateSessionRequest>,
) -> Result<Json<session_models::Session>, AppError> {
    let req = session_models::CreateSessionRequest {
        project_id: input.project_id,
        org_id: input.org_id,
        model: input.model,
    };
    let session =
        session_repo::create(&state.pool, input.project_agent_id, input.created_by, &req).await?;

    let _ = state.events_tx.send(
        serde_json::json!({
            "type": "session.started",
            "sessionId": session.id,
            "projectAgentId": session.project_agent_id,
            "projectId": session.project_id,
        })
        .to_string(),
    );

    Ok(Json(session))
}

pub async fn get_session(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<session_models::Session>, AppError> {
    let session = session_repo::get(&state.pool, id).await?;
    Ok(Json(session))
}

pub async fn update_session(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<session_models::UpdateSessionRequest>,
) -> Result<Json<session_models::Session>, AppError> {
    let session = session_repo::update(&state.pool, id, &input).await?;

    if input.status.is_some() {
        let _ = state.events_tx.send(
            serde_json::json!({
                "type": "session.status_changed",
                "sessionId": session.id,
                "projectAgentId": session.project_agent_id,
                "projectId": session.project_id,
                "status": session.status,
            })
            .to_string(),
        );
    }

    Ok(Json(session))
}

pub async fn list_sessions(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(project_agent_id): Path<Uuid>,
) -> Result<Json<Vec<session_models::Session>>, AppError> {
    let sessions = session_repo::list_by_project_agent(&state.pool, project_agent_id).await?;
    Ok(Json(sessions))
}

// ============================================================================
// Events
// ============================================================================

pub async fn create_event(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Json(input): Json<event_models::CreateEventRequest>,
) -> Result<Json<event_models::SessionEvent>, AppError> {
    let event = event_repo::create(&state.pool, &input).await?;
    Ok(Json(event))
}

pub async fn list_events(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Query(query): Query<event_models::EventListQuery>,
) -> Result<Json<Vec<event_models::SessionEvent>>, AppError> {
    let events =
        event_repo::list_by_session(&state.pool, session_id, query.limit(), query.offset()).await?;
    Ok(Json(events))
}

// ============================================================================
// Logs
// ============================================================================

pub async fn create_log(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Json(input): Json<InternalCreateLogRequest>,
) -> Result<Json<log_models::LogEntry>, AppError> {
    let req = log_models::CreateLogEntryRequest {
        org_id: input.org_id,
        project_agent_id: input.project_agent_id,
        created_by: input.created_by,
        level: input.level,
        message: input.message,
        metadata: input.metadata,
    };
    let entry = log_repo::create(&state.pool, input.project_id, &req).await?;
    Ok(Json(entry))
}

pub async fn list_logs(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<LogListQuery>,
) -> Result<Json<Vec<log_models::LogEntry>>, AppError> {
    let limit = query.limit.unwrap_or(100).min(500).max(1);
    let offset = query.offset.unwrap_or(0).max(0);
    let entries =
        log_repo::list_by_project(&state.pool, project_id, query.level.as_deref(), limit, offset)
            .await?;
    Ok(Json(entries))
}

// ============================================================================
// Project Agents
// ============================================================================

pub async fn create_project_agent(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Json(input): Json<InternalCreateProjectAgentRequest>,
) -> Result<Json<pa_models::ProjectAgent>, AppError> {
    let agent =
        pa_repo::create(&state.pool, input.project_id, input.created_by, &input.agent).await?;
    Ok(Json(agent))
}

pub async fn get_project_agent(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<pa_models::ProjectAgent>, AppError> {
    let agent = pa_repo::get(&state.pool, id).await?;
    Ok(Json(agent))
}

pub async fn list_project_agents(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<pa_models::ProjectAgent>>, AppError> {
    let agents = pa_repo::list_by_project(&state.pool, project_id).await?;
    Ok(Json(agents))
}

pub async fn update_agent_status(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<pa_models::UpdateProjectAgentStatusRequest>,
) -> Result<Json<pa_models::ProjectAgent>, AppError> {
    let agent = pa_repo::update_status(&state.pool, id, &input).await?;

    let _ = state.events_tx.send(
        serde_json::json!({
            "type": "project_agent.status_changed",
            "projectAgentId": agent.id,
            "projectId": agent.project_id,
            "status": agent.status,
        })
        .to_string(),
    );

    Ok(Json(agent))
}

pub async fn delete_project_agent(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    pa_repo::delete(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_project_agent_count(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM project_agents WHERE project_id = $1")
            .bind(project_id)
            .fetch_one(&state.pool)
            .await?;

    Ok(Json(serde_json::json!({ "count": count })))
}

/// Delete all data associated with a project (cascade cleanup).
/// Called by aura-network after it deletes the project record.
pub async fn delete_project_data(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut tx = state.pool.begin().await.map_err(|e| {
        AppError::Internal(format!("failed to begin transaction: {e}"))
    })?;

    let events = sqlx::query("DELETE FROM session_events WHERE project_id = $1")
        .bind(project_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

    let messages = sqlx::query("DELETE FROM messages WHERE project_id = $1")
        .bind(project_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

    let logs = sqlx::query("DELETE FROM log_entries WHERE project_id = $1")
        .bind(project_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

    let artifacts = sqlx::query("DELETE FROM artifacts WHERE project_id = $1")
        .bind(project_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

    let tasks = sqlx::query("DELETE FROM tasks WHERE project_id = $1")
        .bind(project_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

    let sessions = sqlx::query("DELETE FROM sessions WHERE project_id = $1")
        .bind(project_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

    let specs = sqlx::query("DELETE FROM specs WHERE project_id = $1")
        .bind(project_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

    let agents = sqlx::query("DELETE FROM project_agents WHERE project_id = $1")
        .bind(project_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

    tx.commit().await.map_err(|e| {
        AppError::Internal(format!("failed to commit transaction: {e}"))
    })?;

    tracing::info!(
        project_id = %project_id,
        events, messages, logs, artifacts, tasks, sessions, specs, agents,
        "project data cascade delete complete"
    );

    Ok(Json(serde_json::json!({
        "project_id": project_id,
        "deleted": {
            "session_events": events,
            "messages": messages,
            "log_entries": logs,
            "artifacts": artifacts,
            "tasks": tasks,
            "sessions": sessions,
            "specs": specs,
            "project_agents": agents,
        }
    })))
}

// ============================================================================
// Specs
// ============================================================================

pub async fn create_spec(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Json(input): Json<InternalCreateSpecRequest>,
) -> Result<Json<spec_models::Spec>, AppError> {
    let spec =
        spec_repo::create(&state.pool, input.project_id, input.created_by, &input.spec).await?;
    Ok(Json(spec))
}

pub async fn get_spec(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<spec_models::Spec>, AppError> {
    let spec = spec_repo::get(&state.pool, id).await?;
    Ok(Json(spec))
}

pub async fn list_specs(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<spec_models::Spec>>, AppError> {
    let specs = spec_repo::list_by_project(&state.pool, project_id).await?;
    Ok(Json(specs))
}

pub async fn update_spec(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<spec_models::UpdateSpecRequest>,
) -> Result<Json<spec_models::Spec>, AppError> {
    let spec = spec_repo::update(&state.pool, id, &input).await?;
    Ok(Json(spec))
}

pub async fn delete_spec(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    spec_repo::delete(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Tasks
// ============================================================================

pub async fn create_task(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Json(input): Json<InternalCreateTaskRequest>,
) -> Result<Json<task_models::Task>, AppError> {
    let task =
        task_repo::create(&state.pool, input.project_id, input.created_by, &input.task).await?;
    Ok(Json(task))
}

pub async fn get_task(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<task_models::Task>, AppError> {
    let task = task_repo::get(&state.pool, id).await?;
    Ok(Json(task))
}

pub async fn list_tasks(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<TaskListQuery>,
) -> Result<Json<Vec<task_models::Task>>, AppError> {
    let tasks =
        task_repo::list_by_project(&state.pool, project_id, query.status.as_deref()).await?;
    Ok(Json(tasks))
}

pub async fn update_task(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<task_models::UpdateTaskRequest>,
) -> Result<Json<task_models::Task>, AppError> {
    let task = task_repo::update(&state.pool, id, &input).await?;
    Ok(Json(task))
}

pub async fn delete_task(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    task_repo::delete(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn transition_task(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<task_models::TransitionRequest>,
) -> Result<Json<task_models::Task>, AppError> {
    let task = task_repo::transition(&state.pool, id, &input).await?;

    let _ = state.events_tx.send(
        serde_json::json!({
            "type": "task.status_changed",
            "taskId": task.id,
            "projectId": task.project_id,
            "status": task.status,
        })
        .to_string(),
    );

    Ok(Json(task))
}

// ============================================================================
// Stats
// ============================================================================

pub async fn get_stats(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Query(query): Query<super::stats::StatsQuery>,
) -> Result<Json<super::stats::ExecutionStats>, AppError> {
    super::stats::get_stats_inner(
        &state.pool,
        &state.http_client,
        state.aura_network_url.as_deref(),
        state.aura_network_token.as_deref(),
        query,
    )
    .await
}

// ============================================================================
// Artifacts
// ============================================================================

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalCreateArtifactRequest {
    pub project_id: Uuid,
    pub created_by: Uuid,
    #[serde(flatten)]
    pub artifact: artifact_models::CreateArtifactRequest,
}

pub async fn create_artifact(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Json(input): Json<InternalCreateArtifactRequest>,
) -> Result<Json<artifact_models::Artifact>, AppError> {
    let artifact =
        artifact_repo::create(&state.pool, input.project_id, input.created_by, &input.artifact)
            .await?;
    Ok(Json(artifact))
}

pub async fn get_artifact(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<artifact_models::Artifact>, AppError> {
    let artifact = artifact_repo::get(&state.pool, id).await?;
    Ok(Json(artifact))
}

pub async fn list_artifacts(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<artifact_models::ArtifactListQuery>,
) -> Result<Json<Vec<artifact_models::Artifact>>, AppError> {
    let artifacts = artifact_repo::list_by_project(
        &state.pool,
        project_id,
        query.artifact_type.as_deref(),
        query.limit(),
        query.offset(),
    )
    .await?;
    Ok(Json(artifacts))
}

pub async fn delete_artifact(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    artifact_repo::delete(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Processes (internal — for executor and scheduler)
// ============================================================================

pub async fn get_process(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<process_models::Process>, AppError> {
    let process = process_repo::get_process(&state.pool, id).await?;
    Ok(Json(process))
}

pub async fn list_process_nodes(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(process_id): Path<Uuid>,
) -> Result<Json<Vec<process_models::ProcessNode>>, AppError> {
    let nodes = process_repo::list_nodes(&state.pool, process_id).await?;
    Ok(Json(nodes))
}

pub async fn list_process_connections(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(process_id): Path<Uuid>,
) -> Result<Json<Vec<process_models::ProcessNodeConnection>>, AppError> {
    let conns = process_repo::list_connections(&state.pool, process_id).await?;
    Ok(Json(conns))
}

pub async fn update_process(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<process_models::UpdateProcessRequest>,
) -> Result<Json<process_models::Process>, AppError> {
    let process = process_repo::update_process(&state.pool, id, &input).await?;
    Ok(Json(process))
}

pub async fn list_scheduled_processes(
    _auth: InternalAuth,
    State(state): State<AppState>,
) -> Result<Json<Vec<process_models::Process>>, AppError> {
    let processes = process_repo::list_scheduled_processes(&state.pool).await?;
    Ok(Json(processes))
}

pub async fn create_process_run(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Json(input): Json<process_models::CreateProcessRunRequest>,
) -> Result<Json<process_models::ProcessRun>, AppError> {
    let run = process_repo::create_run(&state.pool, &input).await?;
    Ok(Json(run))
}

pub async fn update_process_run(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<process_models::UpdateProcessRunRequest>,
) -> Result<Json<process_models::ProcessRun>, AppError> {
    let run = process_repo::update_run(&state.pool, id, &input).await?;
    Ok(Json(run))
}

pub async fn create_process_event(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Json(input): Json<process_models::CreateProcessEventRequest>,
) -> Result<Json<process_models::ProcessEvent>, AppError> {
    let event = process_repo::create_event(&state.pool, &input).await?;
    Ok(Json(event))
}

pub async fn update_process_event(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<process_models::UpdateProcessEventRequest>,
) -> Result<Json<process_models::ProcessEvent>, AppError> {
    let event = process_repo::update_event(&state.pool, id, &input).await?;
    Ok(Json(event))
}

pub async fn create_process_artifact(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Json(input): Json<process_models::CreateProcessArtifactRequest>,
) -> Result<Json<process_models::ProcessArtifact>, AppError> {
    let artifact = process_repo::create_artifact(&state.pool, &input).await?;
    Ok(Json(artifact))
}
