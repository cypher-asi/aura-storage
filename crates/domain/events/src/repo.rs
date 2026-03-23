use sqlx::PgPool;
use uuid::Uuid;

use aura_storage_core::AppError;

use crate::models::{CreateEventRequest, SessionEvent};

pub async fn create(pool: &PgPool, input: &CreateEventRequest) -> Result<SessionEvent, AppError> {
    // Event type validation relaxed while system shape is being figured out.
    // Valid types are listed in models::VALID_EVENT_TYPES for reference.
    // crate::models::validate_event_type(&input.event_type)?;

    if let Some(ref sender) = input.sender {
        if sender != "user" && sender != "agent" {
            return Err(AppError::BadRequest(format!(
                "Invalid sender: '{sender}'. Must be user or agent"
            )));
        }
    }

    let event = sqlx::query_as::<_, SessionEvent>(
        r#"
        INSERT INTO session_events (session_id, user_id, agent_id, sender, project_id, org_id, type, content)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(input.session_id)
    .bind(input.user_id)
    .bind(input.agent_id)
    .bind(&input.sender)
    .bind(input.project_id)
    .bind(input.org_id)
    .bind(&input.event_type)
    .bind(&input.content)
    .fetch_one(pool)
    .await?;

    Ok(event)
}

pub async fn list_by_session(
    pool: &PgPool,
    session_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<SessionEvent>, AppError> {
    let events = sqlx::query_as::<_, SessionEvent>(
        r#"
        SELECT * FROM session_events
        WHERE session_id = $1
        ORDER BY timestamp ASC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(session_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(events)
}
