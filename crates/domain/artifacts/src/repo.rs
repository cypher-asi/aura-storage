use sqlx::PgPool;
use uuid::Uuid;

use aura_storage_core::AppError;

use crate::models::{Artifact, CreateArtifactRequest};

pub async fn create(
    pool: &PgPool,
    project_id: Uuid,
    created_by: Uuid,
    input: &CreateArtifactRequest,
) -> Result<Artifact, AppError> {
    if input.artifact_type != "image" && input.artifact_type != "model" && input.artifact_type != "video" {
        return Err(AppError::BadRequest(format!(
            "Invalid artifact type: '{}'. Must be image, model, or video",
            input.artifact_type
        )));
    }

    if input.asset_url.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Artifact asset_url must not be empty".into(),
        ));
    }

    if let Some(ref pm) = input.prompt_mode {
        if pm != "new" && pm != "remix" && pm != "edit" {
            return Err(AppError::BadRequest(format!(
                "Invalid prompt_mode: '{pm}'. Must be new, remix, or edit"
            )));
        }
    }

    let artifact = sqlx::query_as::<_, Artifact>(
        r#"
        INSERT INTO artifacts (project_id, org_id, created_by, type, name, description, asset_url, thumbnail_url, original_url, parent_id, is_iteration, prompt, prompt_mode, model, provider, meta)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
        RETURNING *
        "#,
    )
    .bind(project_id)
    .bind(input.org_id)
    .bind(created_by)
    .bind(&input.artifact_type)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.asset_url)
    .bind(&input.thumbnail_url)
    .bind(&input.original_url)
    .bind(input.parent_id)
    .bind(input.is_iteration)
    .bind(&input.prompt)
    .bind(&input.prompt_mode)
    .bind(&input.model)
    .bind(&input.provider)
    .bind(&input.meta)
    .fetch_one(pool)
    .await?;

    Ok(artifact)
}

pub async fn list_by_project(
    pool: &PgPool,
    project_id: Uuid,
    artifact_type: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<Artifact>, AppError> {
    let artifacts = if let Some(t) = artifact_type {
        sqlx::query_as::<_, Artifact>(
            "SELECT * FROM artifacts WHERE project_id = $1 AND type = $2 ORDER BY created_at DESC LIMIT $3 OFFSET $4",
        )
        .bind(project_id)
        .bind(t)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, Artifact>(
            "SELECT * FROM artifacts WHERE project_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(project_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    };

    Ok(artifacts)
}

pub async fn get(pool: &PgPool, id: Uuid) -> Result<Artifact, AppError> {
    sqlx::query_as::<_, Artifact>("SELECT * FROM artifacts WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Artifact not found".into()))
}

pub async fn get_children(pool: &PgPool, parent_id: Uuid) -> Result<Vec<Artifact>, AppError> {
    let artifacts = sqlx::query_as::<_, Artifact>(
        "SELECT * FROM artifacts WHERE parent_id = $1 ORDER BY created_at ASC",
    )
    .bind(parent_id)
    .fetch_all(pool)
    .await?;

    Ok(artifacts)
}

pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    sqlx::query("DELETE FROM artifacts WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
