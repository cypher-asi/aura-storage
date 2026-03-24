use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use aura_storage_auth::InternalAuth;
use aura_storage_core::AppError;
use aura_storage_events::{models as event_models, repo as event_repo};
use aura_storage_logs::{models as log_models, repo as log_repo};

use aura_storage_project_agents::{models as pa_models, repo as pa_repo};
use aura_storage_sessions::{models as session_models, repo as session_repo};

use crate::state::AppState;

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

pub async fn create_event(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Json(input): Json<event_models::CreateEventRequest>,
) -> Result<Json<event_models::SessionEvent>, AppError> {
    let event = event_repo::create(&state.pool, &input).await?;
    Ok(Json(event))
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
