use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use aura_storage_auth::AuthUser;
use aura_storage_core::AppError;
use aura_storage_processes::{models, repo};

use crate::{org_auth::require_org_access, state::AppState};

// ---------------------------------------------------------------------------
// Query params
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrgIdQuery {
    pub org_id: Uuid,
}

// ---------------------------------------------------------------------------
// Public write payloads
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRunRequest {
    pub id: Option<Uuid>,
    pub trigger: Option<String>,
    pub parent_run_id: Option<Uuid>,
    pub input_override: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRunEventRequest {
    pub id: Option<Uuid>,
    pub node_id: Uuid,
    pub status: Option<String>,
    pub input_snapshot: Option<String>,
    pub output: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRunArtifactRequest {
    pub id: Option<Uuid>,
    pub node_id: Uuid,
    pub artifact_type: String,
    pub name: String,
    pub file_path: String,
    pub size_bytes: Option<i64>,
    pub metadata: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Shared authorization helpers
// ---------------------------------------------------------------------------

async fn require_process_access(
    state: &AppState,
    auth: &AuthUser,
    process_id: Uuid,
) -> Result<models::Process, AppError> {
    let process = repo::get_process(&state.pool, process_id).await?;
    require_org_access(state, auth, process.org_id).await?;
    Ok(process)
}

async fn require_folder_access(
    state: &AppState,
    auth: &AuthUser,
    folder_id: Uuid,
) -> Result<models::ProcessFolder, AppError> {
    let folder = repo::get_folder(&state.pool, folder_id).await?;
    require_org_access(state, auth, folder.org_id).await?;
    Ok(folder)
}

async fn require_artifact_access(
    state: &AppState,
    auth: &AuthUser,
    artifact_id: Uuid,
) -> Result<models::ProcessArtifact, AppError> {
    let artifact = repo::get_artifact(&state.pool, artifact_id).await?;
    let _process = require_process_access(state, auth, artifact.process_id).await?;
    Ok(artifact)
}

async fn require_run_access(
    state: &AppState,
    auth: &AuthUser,
    process_id: Uuid,
    run_id: Uuid,
) -> Result<models::ProcessRun, AppError> {
    let _process = require_process_access(state, auth, process_id).await?;
    let run = repo::get_run(&state.pool, run_id).await?;
    if run.process_id != process_id {
        return Err(AppError::Forbidden(
            "Run does not belong to the requested process".into(),
        ));
    }
    Ok(run)
}

async fn require_event_access(
    state: &AppState,
    auth: &AuthUser,
    process_id: Uuid,
    run_id: Uuid,
    event_id: Uuid,
) -> Result<models::ProcessEvent, AppError> {
    let run = require_run_access(state, auth, process_id, run_id).await?;
    let event = repo::get_event(&state.pool, event_id).await?;
    if event.run_id != run.id || event.process_id != process_id {
        return Err(AppError::Forbidden(
            "Event does not belong to the requested process run".into(),
        ));
    }
    Ok(event)
}

// ---------------------------------------------------------------------------
// Processes
// ---------------------------------------------------------------------------

pub async fn create_process(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(input): Json<models::CreateProcessRequest>,
) -> Result<Json<models::Process>, AppError> {
    require_org_access(&state, &auth, input.org_id).await?;

    let created_by = auth
        .user_id
        .parse::<Uuid>()
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    let process = repo::create_process(&state.pool, created_by, &input).await?;
    Ok(Json(process))
}

pub async fn list_processes(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<OrgIdQuery>,
) -> Result<Json<Vec<models::Process>>, AppError> {
    require_org_access(&state, &auth, query.org_id).await?;
    let processes = repo::list_processes(&state.pool, query.org_id).await?;
    Ok(Json(processes))
}

pub async fn get_process(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<models::Process>, AppError> {
    let process = require_process_access(&state, &auth, id).await?;
    Ok(Json(process))
}

pub async fn update_process(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<models::UpdateProcessRequest>,
) -> Result<Json<models::Process>, AppError> {
    let _process = require_process_access(&state, &auth, id).await?;
    let process = repo::update_process(&state.pool, id, &input).await?;
    Ok(Json(process))
}

pub async fn delete_process(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let _process = require_process_access(&state, &auth, id).await?;
    repo::delete_process(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Process nodes
// ---------------------------------------------------------------------------

pub async fn create_node(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(process_id): Path<Uuid>,
    Json(input): Json<models::CreateProcessNodeRequest>,
) -> Result<Json<models::ProcessNode>, AppError> {
    let _process = require_process_access(&state, &auth, process_id).await?;
    let node = repo::create_node(&state.pool, process_id, &input).await?;
    Ok(Json(node))
}

pub async fn list_nodes(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(process_id): Path<Uuid>,
) -> Result<Json<Vec<models::ProcessNode>>, AppError> {
    let _process = require_process_access(&state, &auth, process_id).await?;
    let nodes = repo::list_nodes(&state.pool, process_id).await?;
    Ok(Json(nodes))
}

pub async fn update_node(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((process_id, node_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<models::UpdateProcessNodeRequest>,
) -> Result<Json<models::ProcessNode>, AppError> {
    let _process = require_process_access(&state, &auth, process_id).await?;
    let node = repo::update_node(&state.pool, node_id, &input).await?;
    Ok(Json(node))
}

pub async fn delete_node(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((process_id, node_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    let _process = require_process_access(&state, &auth, process_id).await?;
    repo::delete_node(&state.pool, node_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Process node connections
// ---------------------------------------------------------------------------

pub async fn create_connection(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(process_id): Path<Uuid>,
    Json(input): Json<models::CreateProcessNodeConnectionRequest>,
) -> Result<Json<models::ProcessNodeConnection>, AppError> {
    let _process = require_process_access(&state, &auth, process_id).await?;
    let conn = repo::create_connection(&state.pool, process_id, &input).await?;
    Ok(Json(conn))
}

pub async fn list_connections(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(process_id): Path<Uuid>,
) -> Result<Json<Vec<models::ProcessNodeConnection>>, AppError> {
    let _process = require_process_access(&state, &auth, process_id).await?;
    let conns = repo::list_connections(&state.pool, process_id).await?;
    Ok(Json(conns))
}

pub async fn delete_connection(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((process_id, connection_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    let _process = require_process_access(&state, &auth, process_id).await?;
    repo::delete_connection(&state.pool, connection_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Process runs
// ---------------------------------------------------------------------------

pub async fn list_runs(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(process_id): Path<Uuid>,
) -> Result<Json<Vec<models::ProcessRun>>, AppError> {
    let _process = require_process_access(&state, &auth, process_id).await?;
    let runs = repo::list_runs(&state.pool, process_id).await?;
    Ok(Json(runs))
}

pub async fn create_run(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(process_id): Path<Uuid>,
    Json(input): Json<CreateRunRequest>,
) -> Result<Json<models::ProcessRun>, AppError> {
    let _process = require_process_access(&state, &auth, process_id).await?;
    let req = models::CreateProcessRunRequest {
        id: input.id,
        process_id,
        trigger: input.trigger,
        parent_run_id: input.parent_run_id,
        input_override: input.input_override,
    };
    let run = repo::create_run(&state.pool, &req).await?;
    Ok(Json(run))
}

pub async fn get_run(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((process_id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<models::ProcessRun>, AppError> {
    let run = require_run_access(&state, &auth, process_id, run_id).await?;
    Ok(Json(run))
}

pub async fn update_run(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((process_id, run_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<models::UpdateProcessRunRequest>,
) -> Result<Json<models::ProcessRun>, AppError> {
    let _run = require_run_access(&state, &auth, process_id, run_id).await?;
    let run = repo::update_run(&state.pool, run_id, &input).await?;
    Ok(Json(run))
}

// ---------------------------------------------------------------------------
// Process events
// ---------------------------------------------------------------------------

pub async fn list_run_events(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((process_id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<models::ProcessEvent>>, AppError> {
    let _run = require_run_access(&state, &auth, process_id, run_id).await?;
    let events = repo::list_events(&state.pool, run_id).await?;
    Ok(Json(events))
}

pub async fn create_run_event(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((process_id, run_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<CreateRunEventRequest>,
) -> Result<Json<models::ProcessEvent>, AppError> {
    let _run = require_run_access(&state, &auth, process_id, run_id).await?;
    let req = models::CreateProcessEventRequest {
        id: input.id,
        run_id,
        node_id: input.node_id,
        process_id,
        status: input.status,
        input_snapshot: input.input_snapshot,
        output: input.output,
    };
    let event = repo::create_event(&state.pool, &req).await?;
    Ok(Json(event))
}

pub async fn update_run_event(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((process_id, _run_id, event_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(input): Json<models::UpdateProcessEventRequest>,
) -> Result<Json<models::ProcessEvent>, AppError> {
    let _event = require_event_access(&state, &auth, process_id, _run_id, event_id).await?;
    let event = repo::update_event(&state.pool, event_id, &input).await?;
    Ok(Json(event))
}

// ---------------------------------------------------------------------------
// Process artifacts
// ---------------------------------------------------------------------------

pub async fn list_run_artifacts(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((process_id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<models::ProcessArtifact>>, AppError> {
    let _run = require_run_access(&state, &auth, process_id, run_id).await?;
    let artifacts = repo::list_artifacts_for_run(&state.pool, run_id).await?;
    Ok(Json(artifacts))
}

pub async fn create_run_artifact(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((process_id, run_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<CreateRunArtifactRequest>,
) -> Result<Json<models::ProcessArtifact>, AppError> {
    let _run = require_run_access(&state, &auth, process_id, run_id).await?;
    let req = models::CreateProcessArtifactRequest {
        id: input.id,
        process_id,
        run_id,
        node_id: input.node_id,
        artifact_type: input.artifact_type,
        name: input.name,
        file_path: input.file_path,
        size_bytes: input.size_bytes,
        metadata: input.metadata,
    };
    let artifact = repo::create_artifact(&state.pool, &req).await?;
    Ok(Json(artifact))
}

pub async fn get_artifact(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<models::ProcessArtifact>, AppError> {
    let artifact = require_artifact_access(&state, &auth, id).await?;
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
    require_org_access(&state, &auth, input.org_id).await?;

    let created_by = auth
        .user_id
        .parse::<Uuid>()
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    let folder = repo::create_folder(&state.pool, created_by, &input).await?;
    Ok(Json(folder))
}

pub async fn list_folders(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<OrgIdQuery>,
) -> Result<Json<Vec<models::ProcessFolder>>, AppError> {
    require_org_access(&state, &auth, query.org_id).await?;
    let folders = repo::list_folders(&state.pool, query.org_id).await?;
    Ok(Json(folders))
}

pub async fn update_folder(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<models::UpdateProcessFolderRequest>,
) -> Result<Json<models::ProcessFolder>, AppError> {
    let _folder = require_folder_access(&state, &auth, id).await?;
    let folder = repo::update_folder(&state.pool, id, &input).await?;
    Ok(Json(folder))
}

pub async fn delete_folder(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let _folder = require_folder_access(&state, &auth, id).await?;
    repo::delete_folder(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
