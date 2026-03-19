use sqlx::PgPool;
use uuid::Uuid;

use aura_storage_core::AppError;

use crate::models::{CreateTaskRequest, Task, TransitionRequest, UpdateTaskRequest};

pub async fn create(
    pool: &PgPool,
    project_id: Uuid,
    created_by: Uuid,
    input: &CreateTaskRequest,
) -> Result<Task, AppError> {
    if input.title.trim().is_empty() {
        return Err(AppError::BadRequest("Task title must not be empty".into()));
    }

    let dep_ids = serde_json::to_value(
        input.dependency_task_ids.as_deref().unwrap_or(&[]),
    )
    .unwrap_or_default();

    let task = sqlx::query_as::<_, Task>(
        r#"
        INSERT INTO tasks (project_id, org_id, spec_id, created_by, title, description, order_index,
                          dependency_task_ids, parent_task_id, assigned_project_agent_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING *
        "#,
    )
    .bind(project_id)
    .bind(input.org_id)
    .bind(input.spec_id)
    .bind(created_by)
    .bind(input.title.trim())
    .bind(&input.description)
    .bind(input.order_index)
    .bind(dep_ids)
    .bind(input.parent_task_id)
    .bind(input.assigned_project_agent_id)
    .fetch_one(pool)
    .await?;

    Ok(task)
}

pub async fn list_by_project(
    pool: &PgPool,
    project_id: Uuid,
    status_filter: Option<&str>,
) -> Result<Vec<Task>, AppError> {
    let tasks = match status_filter {
        Some(status) => {
            sqlx::query_as::<_, Task>(
                "SELECT * FROM tasks WHERE project_id = $1 AND status = $2 ORDER BY order_index",
            )
            .bind(project_id)
            .bind(status)
            .fetch_all(pool)
            .await?
        }
        None => {
            sqlx::query_as::<_, Task>(
                "SELECT * FROM tasks WHERE project_id = $1 ORDER BY order_index",
            )
            .bind(project_id)
            .fetch_all(pool)
            .await?
        }
    };

    Ok(tasks)
}

pub async fn get(pool: &PgPool, id: Uuid) -> Result<Task, AppError> {
    sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Task not found".into()))
}

pub async fn update(
    pool: &PgPool,
    id: Uuid,
    input: &UpdateTaskRequest,
) -> Result<Task, AppError> {
    sqlx::query_as::<_, Task>(
        r#"
        UPDATE tasks SET
            title = COALESCE($2, title),
            description = COALESCE($3, description),
            execution_notes = COALESCE($4, execution_notes),
            files_changed = COALESCE($5, files_changed),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&input.title)
    .bind(&input.description)
    .bind(&input.execution_notes)
    .bind(&input.files_changed)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Task not found".into()))
}

pub async fn transition(
    pool: &PgPool,
    id: Uuid,
    input: &TransitionRequest,
) -> Result<Task, AppError> {
    let task = get(pool, id).await?;

    validate_transition(&task.status, &input.status)?;

    sqlx::query_as::<_, Task>(
        r#"
        UPDATE tasks SET
            status = $2,
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&input.status)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM tasks WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Task not found".into()));
    }

    Ok(())
}

fn validate_transition(current: &str, next: &str) -> Result<(), AppError> {
    let valid = match current {
        "pending" => next == "ready",
        "ready" => next == "in_progress",
        "in_progress" => matches!(next, "done" | "failed" | "blocked"),
        "failed" => next == "ready",
        "blocked" => next == "ready",
        _ => false,
    };

    if !valid {
        return Err(AppError::BadRequest(format!(
            "Invalid status transition: '{current}' → '{next}'"
        )));
    }

    Ok(())
}
