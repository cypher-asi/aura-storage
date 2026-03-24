use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use aura_storage_auth::AuthUser;
use aura_storage_core::AppError;
use aura_storage_sessions::{models, repo};

use crate::state::AppState;

pub async fn create_session(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(project_agent_id): Path<Uuid>,
    Json(input): Json<models::CreateSessionRequest>,
) -> Result<Json<models::Session>, AppError> {
    let created_by = auth
        .user_id
        .parse::<Uuid>()
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    let session = repo::create(&state.pool, project_agent_id, created_by, &input).await?;

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

pub async fn list_sessions(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(project_agent_id): Path<Uuid>,
) -> Result<Json<Vec<models::Session>>, AppError> {
    let sessions = repo::list_by_project_agent(&state.pool, project_agent_id).await?;
    Ok(Json(sessions))
}

pub async fn get_session(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<models::Session>, AppError> {
    let session = repo::get(&state.pool, id).await?;
    Ok(Json(session))
}

pub async fn update_session(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<models::UpdateSessionRequest>,
) -> Result<Json<models::Session>, AppError> {
    let session = repo::update(&state.pool, id, &input).await?;

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
