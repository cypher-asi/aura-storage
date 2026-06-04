use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use aura_storage_auth::AuthUser;
use aura_storage_core::AppError;
use aura_storage_notes::{models, repo};

use crate::state::AppState;

// ============================================================================
// Notes
// ============================================================================

pub async fn create_note(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Json(input): Json<models::CreateNoteRequest>,
) -> Result<Json<models::Note>, AppError> {
    let created_by = auth
        .user_id
        .parse::<Uuid>()
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    let note = repo::create(&state.pool, project_id, created_by, &input).await?;
    Ok(Json(note))
}

pub async fn list_notes(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<models::Note>>, AppError> {
    let notes = repo::list_by_project(&state.pool, project_id).await?;
    Ok(Json(notes))
}

pub async fn get_note(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<models::Note>, AppError> {
    let note = repo::get(&state.pool, id).await?;
    Ok(Json(note))
}

pub async fn update_note(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<models::UpdateNoteRequest>,
) -> Result<Json<models::Note>, AppError> {
    let note = repo::update(&state.pool, id, &input).await?;
    Ok(Json(note))
}

pub async fn transition_note(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<models::TransitionNoteRequest>,
) -> Result<Json<models::Note>, AppError> {
    let note = repo::transition_status(&state.pool, id, &input).await?;

    let _ = state.events_tx.send(
        serde_json::json!({
            "type": "note.status_changed",
            "noteId": note.id,
            "projectId": note.project_id,
            "status": note.status,
        })
        .to_string(),
    );

    Ok(Json(note))
}

pub async fn delete_note(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    repo::delete(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Note folders
// ============================================================================

pub async fn create_note_folder(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Json(input): Json<models::CreateNoteFolderRequest>,
) -> Result<Json<models::NoteFolder>, AppError> {
    let created_by = auth
        .user_id
        .parse::<Uuid>()
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    let folder = repo::create_folder(&state.pool, project_id, created_by, &input).await?;
    Ok(Json(folder))
}

pub async fn list_note_folders(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<models::NoteFolder>>, AppError> {
    let folders = repo::list_folders_by_project(&state.pool, project_id).await?;
    Ok(Json(folders))
}

pub async fn update_note_folder(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<models::UpdateNoteFolderRequest>,
) -> Result<Json<models::NoteFolder>, AppError> {
    let folder = repo::update_folder(&state.pool, id, &input).await?;
    Ok(Json(folder))
}

pub async fn delete_note_folder(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    repo::delete_folder(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// Note comments
// ============================================================================

pub async fn create_note_comment(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(note_id): Path<Uuid>,
    Json(input): Json<models::CreateNoteCommentRequest>,
) -> Result<Json<models::NoteComment>, AppError> {
    let comment = repo::create_comment(&state.pool, note_id, &input).await?;
    Ok(Json(comment))
}

pub async fn list_note_comments(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(note_id): Path<Uuid>,
) -> Result<Json<Vec<models::NoteComment>>, AppError> {
    let comments = repo::list_comments_by_note(&state.pool, note_id).await?;
    Ok(Json(comments))
}

pub async fn delete_note_comment(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    repo::delete_comment(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
