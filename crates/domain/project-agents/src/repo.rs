use sqlx::PgPool;
use uuid::Uuid;

use aura_storage_core::AppError;

use crate::models::{CreateProjectAgentRequest, ProjectAgent, UpdateProjectAgentStatusRequest};

const VALID_STATUSES: &[&str] = &["idle", "working", "blocked", "stopped", "error"];

pub async fn create(
    pool: &PgPool,
    project_id: Uuid,
    created_by: Uuid,
    input: &CreateProjectAgentRequest,
) -> Result<ProjectAgent, AppError> {
    let project_agent = sqlx::query_as::<_, ProjectAgent>(
        r#"
        INSERT INTO project_agents (project_id, org_id, agent_id, created_by, model)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(project_id)
    .bind(input.org_id)
    .bind(input.agent_id)
    .bind(created_by)
    .bind(&input.model)
    .fetch_one(pool)
    .await?;

    Ok(project_agent)
}

pub async fn list_by_project(
    pool: &PgPool,
    project_id: Uuid,
) -> Result<Vec<ProjectAgent>, AppError> {
    let agents = sqlx::query_as::<_, ProjectAgent>(
        "SELECT * FROM project_agents WHERE project_id = $1 ORDER BY created_at",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;

    Ok(agents)
}

pub async fn get(pool: &PgPool, id: Uuid) -> Result<ProjectAgent, AppError> {
    sqlx::query_as::<_, ProjectAgent>("SELECT * FROM project_agents WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Project agent not found".into()))
}

pub async fn update_status(
    pool: &PgPool,
    id: Uuid,
    input: &UpdateProjectAgentStatusRequest,
) -> Result<ProjectAgent, AppError> {
    if !VALID_STATUSES.contains(&input.status.as_str()) {
        return Err(AppError::BadRequest(format!(
            "Invalid status: '{}'. Must be one of: {}",
            input.status,
            VALID_STATUSES.join(", ")
        )));
    }

    sqlx::query_as::<_, ProjectAgent>(
        r#"
        UPDATE project_agents SET
            status = $2,
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&input.status)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Project agent not found".into()))
}

pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM project_agents WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Project agent not found".into()));
    }

    Ok(())
}
