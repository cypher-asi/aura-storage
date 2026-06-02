use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use aura_storage_auth::AuthUser;
use aura_storage_core::AppError;
use aura_storage_sessions::{models, repo};

use crate::state::AppState;

/// Query string for the session list endpoints. `include_empty=true`
/// returns sessions with `event_count = 0` (orphan rows from races on
/// session creation, plus pre-`lazy-+` legacy data); the default
/// `false` is what the chat-app session list wants — every row in the
/// response is guaranteed to land in a transcript with at least one
/// user message.
#[derive(Debug, Deserialize, Default)]
pub struct SessionListQuery {
    #[serde(default)]
    pub include_empty: bool,
}

pub async fn create_session(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(project_agent_id): Path<Uuid>,
    Json(input): Json<models::CreateSessionRequest>,
) -> Result<Json<models::Session>, AppError> {
    let created_by = auth
        .user_id
        .parse::<Uuid>()
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;

    let session = repo::create(&state.pool, project_agent_id, created_by, &input).await?;

    let _ = state.events_tx.send(
        serde_json::json!({
            "type": "session.started",
            "sessionId": session.id,
            "projectAgentId": session.project_agent_id,
            "projectId": session.project_id,
        })
        .to_string(),
    );

    Ok(Json(session))
}

pub async fn list_sessions(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(project_agent_id): Path<Uuid>,
    Query(query): Query<SessionListQuery>,
) -> Result<Json<Vec<models::Session>>, AppError> {
    let sessions =
        repo::list_by_project_agent(&state.pool, project_agent_id, query.include_empty).await?;
    Ok(Json(sessions))
}

pub async fn list_project_sessions(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<SessionListQuery>,
) -> Result<Json<Vec<models::Session>>, AppError> {
    let sessions = repo::list_by_project(&state.pool, project_id, query.include_empty).await?;
    Ok(Json(sessions))
}

/// Cross-agent user-scoped session list. Powers the chat-app left
/// panel which used to fan out across every agent the user owns
/// (see `apps/chat-app/components/ChatAppLeftPanel/ChatAppLeftPanel.tsx`
/// in aura-os pre-this-commit). The user_id is derived from the
/// JWT -- there is no `:userId` path param on the public endpoint
/// so callers cannot peek at other users' sessions. Internal
/// callers that need to scope by an arbitrary user_id should use
/// `/internal/users/:userId/sessions` instead.
pub async fn list_my_sessions(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<SessionListQuery>,
) -> Result<Json<Vec<models::EnrichedSession>>, AppError> {
    let user_id = auth
        .user_id
        .parse::<Uuid>()
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;
    let sessions = repo::list_by_user(&state.pool, user_id, query.include_empty).await?;
    Ok(Json(sessions))
}

pub async fn get_session(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<models::Session>, AppError> {
    let session = repo::get(&state.pool, id).await?;
    Ok(Json(session))
}

pub async fn update_session(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<models::UpdateSessionRequest>,
) -> Result<Json<models::Session>, AppError> {
    require_share_update_owner(&state, &auth, id, &input).await?;

    let session = repo::update(&state.pool, id, &input).await?;

    if input.status.is_some() {
        let _ = state.events_tx.send(
            serde_json::json!({
                "type": "session.status_changed",
                "sessionId": session.id,
                "projectAgentId": session.project_agent_id,
                "projectId": session.project_id,
                "status": session.status,
            })
            .to_string(),
        );
    }

    Ok(Json(session))
}

async fn require_share_update_owner(
    state: &AppState,
    auth: &AuthUser,
    session_id: Uuid,
    input: &models::UpdateSessionRequest,
) -> Result<(), AppError> {
    if input.is_public.is_none() && input.public_share_id.is_none() {
        return Ok(());
    }

    let user_id = auth
        .user_id
        .parse::<Uuid>()
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))?;
    let session = repo::get(&state.pool, session_id).await?;

    if session.created_by != user_id {
        return Err(AppError::Forbidden(
            "Only the session owner can update share settings".into(),
        ));
    }

    Ok(())
}
