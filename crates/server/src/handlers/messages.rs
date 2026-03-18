use axum::extract::{Path, Query, State};
use axum::Json;
use uuid::Uuid;

use aura_storage_auth::AuthUser;
use aura_storage_core::{AppError, PaginationParams};
use aura_storage_messages::{models, repo};

use crate::state::AppState;

pub async fn create_message(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(input): Json<models::CreateMessageRequest>,
) -> Result<Json<models::Message>, AppError> {
    let message = repo::create(&state.pool, session_id, &input).await?;
    Ok(Json(message))
}

pub async fn list_messages(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<Vec<models::Message>>, AppError> {
    let messages = repo::list_by_session(
        &state.pool,
        session_id,
        pagination.limit(),
        pagination.offset(),
    )
    .await?;
    Ok(Json(messages))
}
