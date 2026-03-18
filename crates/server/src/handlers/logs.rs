use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use aura_storage_auth::AuthUser;
use aura_storage_core::{AppError, PaginationParams};
use aura_storage_logs::{models, repo};

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct LogListQuery {
    pub level: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn create_log_entry(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Json(input): Json<models::CreateLogEntryRequest>,
) -> Result<Json<models::LogEntry>, AppError> {
    let entry = repo::create(&state.pool, project_id, &input).await?;
    Ok(Json(entry))
}

pub async fn list_log_entries(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<LogListQuery>,
) -> Result<Json<Vec<models::LogEntry>>, AppError> {
    let pagination = PaginationParams {
        limit: query.limit,
        offset: query.offset,
    };
    let entries = repo::list_by_project(
        &state.pool,
        project_id,
        query.level.as_deref(),
        pagination.limit(),
        pagination.offset(),
    )
    .await?;
    Ok(Json(entries))
}
