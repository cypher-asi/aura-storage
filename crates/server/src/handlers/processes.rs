use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use aura_storage_auth::AuthUser;
use aura_storage_core::AppError;
use aura_storage_processes::{models, repo};

use crate::state::AppState;

// ---------------------------------------------------------------------------
// Query params
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgIdQuery {
    pub org_id: Uuid,
}

// ---------------------------------------------------------------------------
// Processes
// ---------------------------------------------------------------------------

pub async fn create_process(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(input): Json<models::CreateProcessRequest>,
) -> Result<Json<models::Process>, AppError> {
    let created_by = auth
        .user_id
        .parse::<Uuid>()
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    let process = repo::create_process(&state.pool, created_by, &input).await?;
    Ok(Json(process))
}

pub async fn list_processes(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<OrgIdQuery>,
) -> Result<Json<Vec<models::Process>>, AppError> {
    let processes = repo::list_processes(&state.pool, query.org_id).await?;
    Ok(Json(processes))
}

pub async fn get_process(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<models::Process>, AppError> {
    let process = repo::get_process(&state.pool, id).await?;
    Ok(Json(process))
}

pub async fn update_process(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<models::UpdateProcessRequest>,
) -> Result<Json<models::Process>, AppError> {
    let process = repo::update_process(&state.pool, id, &input).await?;
    Ok(Json(process))
}

pub async fn delete_process(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    repo::delete_process(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Process nodes
// ---------------------------------------------------------------------------

pub async fn create_node(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(process_id): Path<Uuid>,
    Json(input): Json<models::CreateProcessNodeRequest>,
) -> Result<Json<models::ProcessNode>, AppError> {
    let node = repo::create_node(&state.pool, process_id, &input).await?;
    Ok(Json(node))
}

pub async fn list_nodes(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(process_id): Path<Uuid>,
) -> Result<Json<Vec<models::ProcessNode>>, AppError> {
    let nodes = repo::list_nodes(&state.pool, process_id).await?;
    Ok(Json(nodes))
}

pub async fn update_node(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path((_process_id, node_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<models::UpdateProcessNodeRequest>,
) -> Result<Json<models::ProcessNode>, AppError> {
    let node = repo::update_node(&state.pool, node_id, &input).await?;
    Ok(Json(node))
}

pub async fn delete_node(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path((_process_id, node_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    repo::delete_node(&state.pool, node_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Process node connections
// ---------------------------------------------------------------------------

pub async fn create_connection(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(process_id): Path<Uuid>,
    Json(input): Json<models::CreateProcessNodeConnectionRequest>,
) -> Result<Json<models::ProcessNodeConnection>, AppError> {
    let conn = repo::create_connection(&state.pool, process_id, &input).await?;
    Ok(Json(conn))
}

pub async fn list_connections(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(process_id): Path<Uuid>,
) -> Result<Json<Vec<models::ProcessNodeConnection>>, AppError> {
    let conns = repo::list_connections(&state.pool, process_id).await?;
    Ok(Json(conns))
}

pub async fn delete_connection(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path((_process_id, connection_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    repo::delete_connection(&state.pool, connection_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Process runs
// ---------------------------------------------------------------------------

pub async fn list_runs(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(process_id): Path<Uuid>,
) -> Result<Json<Vec<models::ProcessRun>>, AppError> {
    let runs = repo::list_runs(&state.pool, process_id).await?;
    Ok(Json(runs))
}

pub async fn get_run(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path((_process_id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<models::ProcessRun>, AppError> {
    let run = repo::get_run(&state.pool, run_id).await?;
    Ok(Json(run))
}

// ---------------------------------------------------------------------------
// Process events
// ---------------------------------------------------------------------------

pub async fn list_run_events(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path((_process_id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<models::ProcessEvent>>, AppError> {
    let events = repo::list_events(&state.pool, run_id).await?;
    Ok(Json(events))
}

// ---------------------------------------------------------------------------
// Process artifacts
// ---------------------------------------------------------------------------

pub async fn list_run_artifacts(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path((_process_id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<models::ProcessArtifact>>, AppError> {
    let artifacts = repo::list_artifacts_for_run(&state.pool, run_id).await?;
    Ok(Json(artifacts))
}

pub async fn get_artifact(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<models::ProcessArtifact>, AppError> {
    let artifact = repo::get_artifact(&state.pool, id).await?;
    Ok(Json(artifact))
}

// ---------------------------------------------------------------------------
// Process folders
// ---------------------------------------------------------------------------

pub async fn create_folder(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(input): Json<models::CreateProcessFolderRequest>,
) -> Result<Json<models::ProcessFolder>, AppError> {
    let created_by = auth
        .user_id
        .parse::<Uuid>()
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    let folder = repo::create_folder(&state.pool, created_by, &input).await?;
    Ok(Json(folder))
}

pub async fn list_folders(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<OrgIdQuery>,
) -> Result<Json<Vec<models::ProcessFolder>>, AppError> {
    let folders = repo::list_folders(&state.pool, query.org_id).await?;
    Ok(Json(folders))
}

pub async fn update_folder(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<models::UpdateProcessFolderRequest>,
) -> Result<Json<models::ProcessFolder>, AppError> {
    let folder = repo::update_folder(&state.pool, id, &input).await?;
    Ok(Json(folder))
}

pub async fn delete_folder(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    repo::delete_folder(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
