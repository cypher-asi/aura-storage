use sqlx::PgPool;
use uuid::Uuid;

use aura_storage_core::AppError;

use crate::models::{CreateSpecRequest, Spec, UpdateSpecRequest};

pub async fn create(
    pool: &PgPool,
    project_id: Uuid,
    created_by: Uuid,
    input: &CreateSpecRequest,
) -> Result<Spec, AppError> {
    if input.title.trim().is_empty() {
        return Err(AppError::BadRequest("Spec title must not be empty".into()));
    }

    let spec = sqlx::query_as::<_, Spec>(
        r#"
        INSERT INTO specs (project_id, created_by, title, order_index, markdown_contents)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(project_id)
    .bind(created_by)
    .bind(input.title.trim())
    .bind(input.order_index)
    .bind(&input.markdown_contents)
    .fetch_one(pool)
    .await?;

    Ok(spec)
}

pub async fn list_by_project(pool: &PgPool, project_id: Uuid) -> Result<Vec<Spec>, AppError> {
    let specs = sqlx::query_as::<_, Spec>(
        "SELECT * FROM specs WHERE project_id = $1 ORDER BY order_index",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;

    Ok(specs)
}

pub async fn get(pool: &PgPool, id: Uuid) -> Result<Spec, AppError> {
    sqlx::query_as::<_, Spec>("SELECT * FROM specs WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Spec not found".into()))
}

pub async fn update(
    pool: &PgPool,
    id: Uuid,
    input: &UpdateSpecRequest,
) -> Result<Spec, AppError> {
    sqlx::query_as::<_, Spec>(
        r#"
        UPDATE specs SET
            title = COALESCE($2, title),
            order_index = COALESCE($3, order_index),
            markdown_contents = COALESCE($4, markdown_contents),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&input.title)
    .bind(input.order_index)
    .bind(&input.markdown_contents)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Spec not found".into()))
}

pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM specs WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Spec not found".into()));
    }

    Ok(())
}
