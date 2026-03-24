use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use aura_storage_auth::AuthUser;
use aura_storage_core::AppError;
use aura_storage_tasks::{models, repo};

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct TaskListQuery {
    pub status: Option<String>,
}

pub async fn create_task(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Json(input): Json<models::CreateTaskRequest>,
) -> Result<Json<models::Task>, AppError> {
    let created_by = auth
        .user_id
        .parse::<Uuid>()
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    let task = repo::create(&state.pool, project_id, created_by, &input).await?;
    Ok(Json(task))
}

pub async fn list_tasks(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<TaskListQuery>,
) -> Result<Json<Vec<models::Task>>, AppError> {
    let tasks = repo::list_by_project(&state.pool, project_id, query.status.as_deref()).await?;
    Ok(Json(tasks))
}

pub async fn get_task(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<models::Task>, AppError> {
    let task = repo::get(&state.pool, id).await?;
    Ok(Json(task))
}

pub async fn update_task(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<models::UpdateTaskRequest>,
) -> Result<Json<models::Task>, AppError> {
    let task = repo::update(&state.pool, id, &input).await?;
    Ok(Json(task))
}

pub async fn delete_task(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    repo::delete(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn transition_task(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<models::TransitionRequest>,
) -> Result<Json<models::Task>, AppError> {
    let task = repo::transition(&state.pool, id, &input).await?;

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
