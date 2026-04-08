use sqlx::PgPool;
use uuid::Uuid;

use aura_storage_core::AppError;

use crate::models::*;

// ---------------------------------------------------------------------------
// Process folders
// ---------------------------------------------------------------------------

pub async fn create_folder(
    pool: &PgPool,
    created_by: Uuid,
    input: &CreateProcessFolderRequest,
) -> Result<ProcessFolder, AppError> {
    if input.name.trim().is_empty() {
        return Err(AppError::BadRequest("Folder name must not be empty".into()));
    }

    sqlx::query_as::<_, ProcessFolder>(
        r#"
        INSERT INTO process_folders (org_id, created_by, name)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(input.org_id)
    .bind(created_by)
    .bind(input.name.trim())
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn list_folders(pool: &PgPool, org_id: Uuid) -> Result<Vec<ProcessFolder>, AppError> {
    sqlx::query_as::<_, ProcessFolder>(
        "SELECT * FROM process_folders WHERE org_id = $1 ORDER BY LOWER(name)",
    )
    .bind(org_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn get_folder(pool: &PgPool, id: Uuid) -> Result<ProcessFolder, AppError> {
    sqlx::query_as::<_, ProcessFolder>("SELECT * FROM process_folders WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Process folder not found".into()))
}

pub async fn update_folder(
    pool: &PgPool,
    id: Uuid,
    input: &UpdateProcessFolderRequest,
) -> Result<ProcessFolder, AppError> {
    sqlx::query_as::<_, ProcessFolder>(
        r#"
        UPDATE process_folders SET
            name = COALESCE($2, name),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&input.name)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Process folder not found".into()))
}

pub async fn delete_folder(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM process_folders WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Process folder not found".into()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Processes
// ---------------------------------------------------------------------------

pub async fn create_process(
    pool: &PgPool,
    created_by: Uuid,
    input: &CreateProcessRequest,
) -> Result<Process, AppError> {
    if input.name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Process name must not be empty".into(),
        ));
    }

    let tags = serde_json::to_value(input.tags.as_deref().unwrap_or(&[])).unwrap_or_default();

    sqlx::query_as::<_, Process>(
        r#"
        INSERT INTO processes (org_id, created_by, project_id, folder_id, name, description,
                               enabled, schedule, tags)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
    )
    .bind(input.org_id)
    .bind(created_by)
    .bind(input.project_id)
    .bind(input.folder_id)
    .bind(input.name.trim())
    .bind(input.description.as_deref().unwrap_or(""))
    .bind(input.enabled.unwrap_or(true))
    .bind(&input.schedule)
    .bind(tags)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn list_processes(pool: &PgPool, org_id: Uuid) -> Result<Vec<Process>, AppError> {
    sqlx::query_as::<_, Process>(
        "SELECT * FROM processes WHERE org_id = $1 ORDER BY created_at DESC",
    )
    .bind(org_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn list_scheduled_processes(pool: &PgPool) -> Result<Vec<Process>, AppError> {
    sqlx::query_as::<_, Process>(
        "SELECT * FROM processes WHERE enabled = TRUE AND schedule IS NOT NULL",
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn get_process(pool: &PgPool, id: Uuid) -> Result<Process, AppError> {
    sqlx::query_as::<_, Process>("SELECT * FROM processes WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Process not found".into()))
}

pub async fn update_process(
    pool: &PgPool,
    id: Uuid,
    input: &UpdateProcessRequest,
) -> Result<Process, AppError> {
    let tags = input.tags.as_ref().map(|t| serde_json::to_value(t).unwrap_or_default());

    sqlx::query_as::<_, Process>(
        r#"
        UPDATE processes SET
            name = COALESCE($2, name),
            description = COALESCE($3, description),
            project_id = CASE WHEN $4 THEN $5 ELSE project_id END,
            folder_id = CASE WHEN $6 THEN $7 ELSE folder_id END,
            enabled = COALESCE($8, enabled),
            schedule = CASE WHEN $9 THEN $10 ELSE schedule END,
            tags = COALESCE($11, tags),
            last_run_at = CASE WHEN $12 THEN $13 ELSE last_run_at END,
            next_run_at = CASE WHEN $14 THEN $15 ELSE next_run_at END,
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(input.project_id.is_some())
    .bind(input.project_id.flatten())
    .bind(input.folder_id.is_some())
    .bind(input.folder_id.flatten())
    .bind(input.enabled)
    .bind(input.schedule.is_some())
    .bind(input.schedule.as_ref().and_then(|s| s.as_ref()))
    .bind(tags)
    .bind(input.last_run_at.is_some())
    .bind(input.last_run_at.flatten())
    .bind(input.next_run_at.is_some())
    .bind(input.next_run_at.flatten())
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Process not found".into()))
}

pub async fn delete_process(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM processes WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Process not found".into()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Process nodes
// ---------------------------------------------------------------------------

pub async fn create_node(
    pool: &PgPool,
    process_id: Uuid,
    input: &CreateProcessNodeRequest,
) -> Result<ProcessNode, AppError> {
    sqlx::query_as::<_, ProcessNode>(
        r#"
        INSERT INTO process_nodes (process_id, node_type, label, agent_id, prompt, config,
                                   position_x, position_y)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(process_id)
    .bind(&input.node_type)
    .bind(input.label.as_deref().unwrap_or(""))
    .bind(input.agent_id)
    .bind(input.prompt.as_deref().unwrap_or(""))
    .bind(input.config.as_ref().unwrap_or(&serde_json::json!({})))
    .bind(input.position_x.unwrap_or(0.0))
    .bind(input.position_y.unwrap_or(0.0))
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn list_nodes(pool: &PgPool, process_id: Uuid) -> Result<Vec<ProcessNode>, AppError> {
    sqlx::query_as::<_, ProcessNode>(
        "SELECT * FROM process_nodes WHERE process_id = $1 ORDER BY created_at",
    )
    .bind(process_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn get_node(pool: &PgPool, id: Uuid) -> Result<ProcessNode, AppError> {
    sqlx::query_as::<_, ProcessNode>("SELECT * FROM process_nodes WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Process node not found".into()))
}

pub async fn update_node(
    pool: &PgPool,
    id: Uuid,
    input: &UpdateProcessNodeRequest,
) -> Result<ProcessNode, AppError> {
    sqlx::query_as::<_, ProcessNode>(
        r#"
        UPDATE process_nodes SET
            label = COALESCE($2, label),
            agent_id = CASE WHEN $3 THEN $4 ELSE agent_id END,
            prompt = COALESCE($5, prompt),
            config = COALESCE($6, config),
            position_x = COALESCE($7, position_x),
            position_y = COALESCE($8, position_y),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&input.label)
    .bind(input.agent_id.is_some())
    .bind(input.agent_id.flatten())
    .bind(&input.prompt)
    .bind(&input.config)
    .bind(input.position_x)
    .bind(input.position_y)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Process node not found".into()))
}

pub async fn delete_node(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM process_nodes WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Process node not found".into()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Process node connections
// ---------------------------------------------------------------------------

pub async fn create_connection(
    pool: &PgPool,
    process_id: Uuid,
    input: &CreateProcessNodeConnectionRequest,
) -> Result<ProcessNodeConnection, AppError> {
    sqlx::query_as::<_, ProcessNodeConnection>(
        r#"
        INSERT INTO process_node_connections (process_id, source_node_id, source_handle,
                                              target_node_id, target_handle)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(process_id)
    .bind(input.source_node_id)
    .bind(&input.source_handle)
    .bind(input.target_node_id)
    .bind(&input.target_handle)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn list_connections(
    pool: &PgPool,
    process_id: Uuid,
) -> Result<Vec<ProcessNodeConnection>, AppError> {
    sqlx::query_as::<_, ProcessNodeConnection>(
        "SELECT * FROM process_node_connections WHERE process_id = $1",
    )
    .bind(process_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn delete_connection(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM process_node_connections WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Connection not found".into()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Process runs
// ---------------------------------------------------------------------------

pub async fn create_run(
    pool: &PgPool,
    input: &CreateProcessRunRequest,
) -> Result<ProcessRun, AppError> {
    sqlx::query_as::<_, ProcessRun>(
        r#"
        INSERT INTO process_runs (id, process_id, trigger, parent_run_id, input_override)
        VALUES (COALESCE($1, gen_random_uuid()), $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(input.id)
    .bind(input.process_id)
    .bind(input.trigger.as_deref().unwrap_or("manual"))
    .bind(input.parent_run_id)
    .bind(&input.input_override)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn list_runs(pool: &PgPool, process_id: Uuid) -> Result<Vec<ProcessRun>, AppError> {
    sqlx::query_as::<_, ProcessRun>(
        "SELECT * FROM process_runs WHERE process_id = $1 ORDER BY started_at DESC",
    )
    .bind(process_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn get_run(pool: &PgPool, id: Uuid) -> Result<ProcessRun, AppError> {
    sqlx::query_as::<_, ProcessRun>("SELECT * FROM process_runs WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Process run not found".into()))
}

pub async fn update_run(
    pool: &PgPool,
    id: Uuid,
    input: &UpdateProcessRunRequest,
) -> Result<ProcessRun, AppError> {
    sqlx::query_as::<_, ProcessRun>(
        r#"
        UPDATE process_runs SET
            status = COALESCE($2, status),
            error = CASE WHEN $3 THEN $4 ELSE error END,
            completed_at = CASE WHEN $5 THEN $6 ELSE completed_at END,
            total_input_tokens = COALESCE($7, total_input_tokens),
            total_output_tokens = COALESCE($8, total_output_tokens),
            cost_usd = COALESCE($9, cost_usd),
            output = CASE WHEN $10 THEN $11 ELSE output END
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&input.status)
    .bind(input.error.is_some())
    .bind(input.error.as_ref().and_then(|e| e.as_ref()))
    .bind(input.completed_at.is_some())
    .bind(input.completed_at.flatten())
    .bind(input.total_input_tokens)
    .bind(input.total_output_tokens)
    .bind(input.cost_usd)
    .bind(input.output.is_some())
    .bind(input.output.as_ref().and_then(|o| o.as_ref()))
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Process run not found".into()))
}

// ---------------------------------------------------------------------------
// Process events
// ---------------------------------------------------------------------------

pub async fn create_event(
    pool: &PgPool,
    input: &CreateProcessEventRequest,
) -> Result<ProcessEvent, AppError> {
    sqlx::query_as::<_, ProcessEvent>(
        r#"
        INSERT INTO process_events (id, run_id, node_id, process_id, status, input_snapshot, output)
        VALUES (COALESCE($1, gen_random_uuid()), $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(input.id)
    .bind(input.run_id)
    .bind(input.node_id)
    .bind(input.process_id)
    .bind(input.status.as_deref().unwrap_or("pending"))
    .bind(input.input_snapshot.as_deref().unwrap_or(""))
    .bind(input.output.as_deref().unwrap_or(""))
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn list_events(
    pool: &PgPool,
    run_id: Uuid,
) -> Result<Vec<ProcessEvent>, AppError> {
    sqlx::query_as::<_, ProcessEvent>(
        "SELECT * FROM process_events WHERE run_id = $1 ORDER BY started_at ASC",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn update_event(
    pool: &PgPool,
    id: Uuid,
    input: &UpdateProcessEventRequest,
) -> Result<ProcessEvent, AppError> {
    sqlx::query_as::<_, ProcessEvent>(
        r#"
        UPDATE process_events SET
            status = COALESCE($2, status),
            output = COALESCE($3, output),
            completed_at = CASE WHEN $4 THEN $5 ELSE completed_at END,
            input_tokens = COALESCE($6, input_tokens),
            output_tokens = COALESCE($7, output_tokens),
            model = COALESCE($8, model),
            content_blocks = COALESCE($9, content_blocks)
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&input.status)
    .bind(&input.output)
    .bind(input.completed_at.is_some())
    .bind(input.completed_at.flatten())
    .bind(input.input_tokens)
    .bind(input.output_tokens)
    .bind(&input.model)
    .bind(&input.content_blocks)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Process event not found".into()))
}

// ---------------------------------------------------------------------------
// Process artifacts
// ---------------------------------------------------------------------------

pub async fn create_artifact(
    pool: &PgPool,
    input: &CreateProcessArtifactRequest,
) -> Result<ProcessArtifact, AppError> {
    sqlx::query_as::<_, ProcessArtifact>(
        r#"
        INSERT INTO process_artifacts (id, process_id, run_id, node_id, artifact_type, name,
                                       file_path, size_bytes, metadata)
        VALUES (COALESCE($1, gen_random_uuid()), $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
    )
    .bind(input.id)
    .bind(input.process_id)
    .bind(input.run_id)
    .bind(input.node_id)
    .bind(&input.artifact_type)
    .bind(&input.name)
    .bind(&input.file_path)
    .bind(input.size_bytes.unwrap_or(0))
    .bind(input.metadata.as_ref().unwrap_or(&serde_json::json!({})))
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn list_artifacts_for_run(
    pool: &PgPool,
    run_id: Uuid,
) -> Result<Vec<ProcessArtifact>, AppError> {
    sqlx::query_as::<_, ProcessArtifact>(
        "SELECT * FROM process_artifacts WHERE run_id = $1 ORDER BY created_at",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn list_artifacts_for_process(
    pool: &PgPool,
    process_id: Uuid,
) -> Result<Vec<ProcessArtifact>, AppError> {
    sqlx::query_as::<_, ProcessArtifact>(
        "SELECT * FROM process_artifacts WHERE process_id = $1 ORDER BY created_at",
    )
    .bind(process_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn get_artifact(pool: &PgPool, id: Uuid) -> Result<ProcessArtifact, AppError> {
    sqlx::query_as::<_, ProcessArtifact>("SELECT * FROM process_artifacts WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Process artifact not found".into()))
}
