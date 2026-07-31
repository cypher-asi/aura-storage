use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use aura_storage_auth::AuthUser;
use aura_storage_core::AppError;
use aura_storage_skills::{models, repo};

use crate::{org_auth::require_org_access, state::AppState};

fn user_id(auth: &AuthUser) -> Result<Uuid, AppError> {
    auth.user_id
        .parse::<Uuid>()
        .map_err(|_| AppError::BadRequest("Invalid user ID".into()))
}

async fn require_skill_access(
    state: &AppState,
    auth: &AuthUser,
    skill: &models::Skill,
) -> Result<(), AppError> {
    if let Some(org_id) = skill.org_id {
        require_org_access(state, auth, org_id).await
    } else if skill.created_by == user_id(auth)? {
        Ok(())
    } else {
        Err(AppError::NotFound("Skill not found".into()))
    }
}

pub async fn create_skill(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(input): Json<models::CreateSkillRequest>,
) -> Result<(StatusCode, Json<models::Skill>), AppError> {
    if let Some(org_id) = input.org_id {
        require_org_access(&state, &auth, org_id).await?;
    }
    let skill = repo::create(&state.pool, user_id(&auth)?, &input).await?;
    Ok((StatusCode::CREATED, Json(skill)))
}

pub async fn list_skills(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<models::SkillScopeQuery>,
) -> Result<Json<Vec<models::Skill>>, AppError> {
    if let Some(org_id) = query.org_id {
        require_org_access(&state, &auth, org_id).await?;
    }
    Ok(Json(
        repo::list(&state.pool, user_id(&auth)?, query.org_id).await?,
    ))
}

/// Synchronization feed includes soft-deleted tombstones so other Aura
/// runtimes can remove a definition that was deleted elsewhere.
pub async fn list_skills_for_sync(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<models::SkillScopeQuery>,
) -> Result<Json<Vec<models::Skill>>, AppError> {
    if let Some(org_id) = query.org_id {
        require_org_access(&state, &auth, org_id).await?;
    }
    Ok(Json(
        repo::list_for_sync(&state.pool, user_id(&auth)?, query.org_id).await?,
    ))
}

pub async fn get_skill(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<models::Skill>, AppError> {
    let skill = repo::get(&state.pool, id).await?;
    require_skill_access(&state, &auth, &skill).await?;
    Ok(Json(skill))
}

pub async fn update_skill(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<models::UpdateSkillRequest>,
) -> Result<Json<models::Skill>, AppError> {
    let existing = repo::get(&state.pool, id).await?;
    require_skill_access(&state, &auth, &existing).await?;
    Ok(Json(repo::update(&state.pool, id, &input).await?))
}

pub async fn delete_skill(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let skill = repo::get(&state.pool, id).await?;
    require_skill_access(&state, &auth, &skill).await?;
    repo::soft_delete(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_agent_skills(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
    Query(query): Query<models::SkillScopeQuery>,
) -> Result<Json<Vec<models::Skill>>, AppError> {
    if let Some(org_id) = query.org_id {
        require_org_access(&state, &auth, org_id).await?;
    }
    Ok(Json(
        repo::list_for_agent(&state.pool, agent_id, user_id(&auth)?, query.org_id).await?,
    ))
}

pub async fn assign_agent_skill(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((agent_id, skill_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<models::AgentSkillAssignment>, AppError> {
    let skill = repo::get(&state.pool, skill_id).await?;
    require_skill_access(&state, &auth, &skill).await?;
    Ok(Json(
        repo::assign(&state.pool, &skill, agent_id, user_id(&auth)?).await?,
    ))
}

pub async fn unassign_agent_skill(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((agent_id, skill_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    let skill = repo::get(&state.pool, skill_id).await?;
    require_skill_access(&state, &auth, &skill).await?;
    repo::unassign(&state.pool, skill_id, agent_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
