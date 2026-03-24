use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use aura_storage_auth::AuthUser;
use aura_storage_core::AppError;
use aura_storage_project_agents::{models, repo};

use crate::state::AppState;

pub async fn create_project_agent(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Json(input): Json<models::CreateProjectAgentRequest>,
) -> Result<Json<models::ProjectAgent>, AppError> {
    let created_by = auth
        .user_id
        .parse::<Uuid>()
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    let agent = repo::create(&state.pool, project_id, created_by, &input).await?;
    Ok(Json(agent))
}

pub async fn list_project_agents(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<models::ProjectAgent>>, AppError> {
    let agents = repo::list_by_project(&state.pool, project_id).await?;
    Ok(Json(agents))
}

pub async fn get_project_agent(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<models::ProjectAgent>, AppError> {
    let agent = repo::get(&state.pool, id).await?;
    Ok(Json(agent))
}

pub async fn update_project_agent(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<models::UpdateProjectAgentStatusRequest>,
) -> Result<Json<models::ProjectAgent>, AppError> {
    let agent = repo::update_status(&state.pool, id, &input).await?;

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
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    repo::delete(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
