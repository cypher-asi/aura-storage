use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use aura_storage_auth::InternalAuth;
use aura_storage_core::AppError;
use aura_storage_logs::{models as log_models, repo as log_repo};
use aura_storage_messages::{models as msg_models, repo as msg_repo};
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
pub struct InternalCreateMessageRequest {
    pub session_id: Uuid,
    pub project_agent_id: Uuid,
    pub project_id: Uuid,
    pub org_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub role: String,
    pub content: String,
    pub content_blocks: Option<serde_json::Value>,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub thinking: Option<String>,
    pub thinking_duration_ms: Option<i64>,
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
    let session = session_repo::create(&state.pool, input.project_agent_id, input.created_by, &req).await?;

    let _ = state.events_tx.send(serde_json::json!({
        "type": "session.started",
        "sessionId": session.id,
        "projectAgentId": session.project_agent_id,
        "projectId": session.project_id,
    }).to_string());

    Ok(Json(session))
}

pub async fn create_message(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Json(input): Json<InternalCreateMessageRequest>,
) -> Result<Json<msg_models::Message>, AppError> {
    let req = msg_models::CreateMessageRequest {
        project_agent_id: input.project_agent_id,
        project_id: input.project_id,
        org_id: input.org_id,
        created_by: input.created_by,
        role: input.role,
        content: input.content,
        content_blocks: input.content_blocks,
        input_tokens: input.input_tokens,
        output_tokens: input.output_tokens,
        thinking: input.thinking,
        thinking_duration_ms: input.thinking_duration_ms,
    };
    let message = msg_repo::create(&state.pool, input.session_id, &req).await?;
    Ok(Json(message))
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

    let _ = state.events_tx.send(serde_json::json!({
        "type": "project_agent.status_changed",
        "projectAgentId": agent.id,
        "projectId": agent.project_id,
        "status": agent.status,
    }).to_string());

    Ok(Json(agent))
}
