use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use aura_storage_artifacts::{models, repo};
use aura_storage_auth::AuthUser;
use aura_storage_core::AppError;

use crate::state::AppState;

pub async fn create_artifact(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Json(input): Json<models::CreateArtifactRequest>,
) -> Result<Json<models::Artifact>, AppError> {
    let created_by = auth
        .user_id
        .parse::<Uuid>()
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    let artifact = repo::create(&state.pool, project_id, created_by, &input).await?;
    Ok(Json(artifact))
}

pub async fn list_artifacts(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<models::ArtifactListQuery>,
) -> Result<Json<Vec<models::Artifact>>, AppError> {
    let artifacts = repo::list_by_project(
        &state.pool,
        project_id,
        query.artifact_type.as_deref(),
        query.limit(),
        query.offset(),
    )
    .await?;
    Ok(Json(artifacts))
}

pub async fn get_artifact(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<models::Artifact>, AppError> {
    let artifact = repo::get(&state.pool, id).await?;
    Ok(Json(artifact))
}

pub async fn get_artifact_children(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<models::Artifact>>, AppError> {
    let children = repo::get_children(&state.pool, id).await?;
    Ok(Json(children))
}

pub async fn delete_artifact(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    repo::delete(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
