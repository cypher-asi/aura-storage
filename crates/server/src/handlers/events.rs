use axum::extract::{Path, Query, State};
use axum::Json;
use uuid::Uuid;

use aura_storage_auth::AuthUser;
use aura_storage_core::AppError;
use aura_storage_events::{models, repo};

use crate::state::AppState;

pub async fn create_event(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(mut input): Json<models::CreateEventRequest>,
) -> Result<Json<models::SessionEvent>, AppError> {
    input.session_id = session_id;
    let event = repo::create(&state.pool, &input).await?;
    Ok(Json(event))
}

pub async fn list_events(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Query(query): Query<models::EventListQuery>,
) -> Result<Json<Vec<models::SessionEvent>>, AppError> {
    let events =
        repo::list_by_session(&state.pool, session_id, query.limit(), query.offset()).await?;
    Ok(Json(events))
}
