use sqlx::PgPool;
use uuid::Uuid;

use aura_storage_core::AppError;

use crate::models::{CreateMessageRequest, Message};

const VALID_ROLES: &[&str] = &["user", "assistant", "system"];

pub async fn create(
    pool: &PgPool,
    session_id: Uuid,
    input: &CreateMessageRequest,
) -> Result<Message, AppError> {
    if !VALID_ROLES.contains(&input.role.as_str()) {
        return Err(AppError::BadRequest(format!(
            "Invalid message role: '{}'. Must be one of: {}",
            input.role,
            VALID_ROLES.join(", ")
        )));
    }

    let message = sqlx::query_as::<_, Message>(
        r#"
        INSERT INTO messages (session_id, project_agent_id, project_id, created_by, role, content,
                            content_blocks, input_tokens, output_tokens)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
    )
    .bind(session_id)
    .bind(input.project_agent_id)
    .bind(input.project_id)
    .bind(input.created_by)
    .bind(&input.role)
    .bind(&input.content)
    .bind(&input.content_blocks)
    .bind(input.input_tokens)
    .bind(input.output_tokens)
    .fetch_one(pool)
    .await?;

    Ok(message)
}

pub async fn list_by_session(
    pool: &PgPool,
    session_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<Message>, AppError> {
    let messages = sqlx::query_as::<_, Message>(
        "SELECT * FROM messages WHERE session_id = $1 ORDER BY created_at LIMIT $2 OFFSET $3",
    )
    .bind(session_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(messages)
}
