use axum::extract::{Path, Query, State};
use axum::Json;

use aura_storage_auth::InternalAuth;
use aura_storage_core::AppError;
use aura_storage_observability::{models, repo};

use crate::state::AppState;

pub async fn ingest_run(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Json(input): Json<models::IngestObservabilityRunRequest>,
) -> Result<Json<models::ObservabilityRun>, AppError> {
    let run = repo::ingest(&state.pool, &input).await?;
    Ok(Json(run))
}

pub async fn latest_run(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Query(query): Query<models::ObservabilityHistoryQuery>,
) -> Result<Json<models::ObservabilityRun>, AppError> {
    let run = repo::latest(
        &state.pool,
        query.source.as_deref(),
        query.environment.as_deref(),
    )
    .await?;
    Ok(Json(run))
}

pub async fn list_runs(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Query(query): Query<models::ObservabilityHistoryQuery>,
) -> Result<Json<Vec<models::ObservabilityRun>>, AppError> {
    let runs = repo::list_runs(&state.pool, &query).await?;
    Ok(Json(runs))
}

pub async fn feature_history(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(feature_id): Path<String>,
    Query(query): Query<models::ObservabilityHistoryQuery>,
) -> Result<Json<Vec<models::ObservabilityFeatureResult>>, AppError> {
    let rows = repo::feature_history(&state.pool, &feature_id, &query).await?;
    Ok(Json(rows))
}

pub async fn check_history(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Path(check_id): Path<String>,
    Query(query): Query<models::ObservabilityHistoryQuery>,
) -> Result<Json<Vec<models::ObservabilityCheckResult>>, AppError> {
    let rows = repo::check_history(&state.pool, &check_id, &query).await?;
    Ok(Json(rows))
}

pub async fn recent_failures(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Query(query): Query<models::ObservabilityFailureQuery>,
) -> Result<Json<Vec<models::ObservabilityCheckResult>>, AppError> {
    let rows = repo::recent_failures(&state.pool, &query).await?;
    Ok(Json(rows))
}

pub async fn top_failures(
    _auth: InternalAuth,
    State(state): State<AppState>,
    Query(query): Query<models::ObservabilityFailureQuery>,
) -> Result<Json<Vec<models::ObservabilityFailureSummary>>, AppError> {
    let rows = repo::top_failures(&state.pool, &query).await?;
    Ok(Json(rows))
}
