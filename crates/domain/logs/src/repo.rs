use sqlx::PgPool;
use uuid::Uuid;

use aura_storage_core::AppError;

use crate::models::{CreateLogEntryRequest, LogEntry};

const VALID_LEVELS: &[&str] = &["info", "warn", "error", "debug"];

pub async fn create(
    pool: &PgPool,
    project_id: Uuid,
    input: &CreateLogEntryRequest,
) -> Result<LogEntry, AppError> {
    if !VALID_LEVELS.contains(&input.level.as_str()) {
        return Err(AppError::BadRequest(format!(
            "Invalid log level: '{}'. Must be one of: {}",
            input.level,
            VALID_LEVELS.join(", ")
        )));
    }

    let entry = sqlx::query_as::<_, LogEntry>(
        r#"
        INSERT INTO log_entries (project_id, org_id, project_agent_id, created_by, level, message, metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(project_id)
    .bind(input.org_id)
    .bind(input.project_agent_id)
    .bind(input.created_by)
    .bind(&input.level)
    .bind(&input.message)
    .bind(&input.metadata)
    .fetch_one(pool)
    .await?;

    Ok(entry)
}

pub async fn list_by_project(
    pool: &PgPool,
    project_id: Uuid,
    level_filter: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<LogEntry>, AppError> {
    let entries = match level_filter {
        Some(level) => {
            sqlx::query_as::<_, LogEntry>(
                "SELECT * FROM log_entries WHERE project_id = $1 AND level = $2 ORDER BY created_at DESC LIMIT $3 OFFSET $4",
            )
            .bind(project_id)
            .bind(level)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        }
        None => {
            sqlx::query_as::<_, LogEntry>(
                "SELECT * FROM log_entries WHERE project_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
            )
            .bind(project_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?
        }
    };

    Ok(entries)
}
