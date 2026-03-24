use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use aura_storage_auth::AuthUser;
use aura_storage_core::AppError;
use aura_storage_specs::{models, repo};

use crate::state::AppState;

pub async fn create_spec(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Json(input): Json<models::CreateSpecRequest>,
) -> Result<Json<models::Spec>, AppError> {
    let created_by = auth
        .user_id
        .parse::<Uuid>()
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    let spec = repo::create(&state.pool, project_id, created_by, &input).await?;
    Ok(Json(spec))
}

pub async fn list_specs(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<models::Spec>>, AppError> {
    let specs = repo::list_by_project(&state.pool, project_id).await?;
    Ok(Json(specs))
}

pub async fn get_spec(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<models::Spec>, AppError> {
    let spec = repo::get(&state.pool, id).await?;
    Ok(Json(spec))
}

pub async fn update_spec(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<models::UpdateSpecRequest>,
) -> Result<Json<models::Spec>, AppError> {
    let spec = repo::update(&state.pool, id, &input).await?;
    Ok(Json(spec))
}

pub async fn delete_spec(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    repo::delete(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
