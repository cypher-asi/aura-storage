use aura_storage_core::AppError;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{AgentSkillAssignment, CreateSkillRequest, Skill, UpdateSkillRequest};

pub fn validate_skill_name(name: &str) -> Result<(), AppError> {
    let valid = !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && name
            .chars()
            .last()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !valid {
        return Err(AppError::BadRequest(
            "Skill name must contain only lowercase letters, numbers, and internal hyphens".into(),
        ));
    }
    Ok(())
}

fn validate_tools(tools: &[String]) -> Result<(), AppError> {
    if tools.len() > 128
        || tools
            .iter()
            .any(|tool| tool.trim().is_empty() || tool.len() > 200)
    {
        return Err(AppError::BadRequest("Invalid allowed tools list".into()));
    }
    Ok(())
}

pub async fn create(
    pool: &PgPool,
    created_by: Uuid,
    input: &CreateSkillRequest,
) -> Result<Skill, AppError> {
    validate_skill_name(&input.name)?;
    validate_tools(&input.allowed_tools)?;
    let allowed_tools = serde_json::to_value(&input.allowed_tools)
        .map_err(|error| AppError::BadRequest(format!("Invalid allowed tools: {error}")))?;

    sqlx::query_as::<_, Skill>(
        r#"
        INSERT INTO skills (
            org_id, created_by, name, description, body, allowed_tools,
            model, context, user_invocable, model_invocable, agent_target,
            content_hash
        )
        VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
            md5($3 || E'\n' || $4 || E'\n' || $5 || E'\n' || $6::text ||
                E'\n' || COALESCE($7, '') || E'\n' || COALESCE($8, '') ||
                E'\n' || $9::text || E'\n' || $10::text ||
                E'\n' || COALESCE($11::text, ''))
        )
        RETURNING *
        "#,
    )
    .bind(input.org_id)
    .bind(created_by)
    .bind(&input.name)
    .bind(input.description.trim())
    .bind(&input.body)
    .bind(allowed_tools)
    .bind(input.model.as_deref())
    .bind(input.context.as_deref())
    .bind(input.user_invocable.unwrap_or(true))
    .bind(input.model_invocable.unwrap_or(false))
    .bind(&input.agent_target)
    .fetch_one(pool)
    .await
    .map_err(|error| {
        if error
            .as_database_error()
            .is_some_and(|db| db.is_unique_violation())
        {
            AppError::Conflict(format!("A skill named '{}' already exists", input.name))
        } else {
            AppError::Database(error)
        }
    })
}

pub async fn list(
    pool: &PgPool,
    created_by: Uuid,
    org_id: Option<Uuid>,
) -> Result<Vec<Skill>, AppError> {
    sqlx::query_as::<_, Skill>(
        r#"
        SELECT * FROM skills
        WHERE deleted_at IS NULL
          AND (
            (org_id IS NULL AND created_by = $1)
            OR ($2::uuid IS NOT NULL AND org_id = $2)
          )
        ORDER BY LOWER(name), updated_at DESC
        "#,
    )
    .bind(created_by)
    .bind(org_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn list_for_sync(
    pool: &PgPool,
    created_by: Uuid,
    org_id: Option<Uuid>,
) -> Result<Vec<Skill>, AppError> {
    sqlx::query_as::<_, Skill>(
        r#"
        SELECT * FROM skills
        WHERE
            (org_id IS NULL AND created_by = $1)
            OR ($2::uuid IS NOT NULL AND org_id = $2)
        ORDER BY updated_at, id
        "#,
    )
    .bind(created_by)
    .bind(org_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn get(pool: &PgPool, id: Uuid) -> Result<Skill, AppError> {
    sqlx::query_as::<_, Skill>("SELECT * FROM skills WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Skill not found".into()))
}

pub async fn update(
    pool: &PgPool,
    id: Uuid,
    input: &UpdateSkillRequest,
) -> Result<Skill, AppError> {
    if input.expected_revision < 1 {
        return Err(AppError::BadRequest(
            "expectedRevision must be a positive integer".into(),
        ));
    }
    if let Some(tools) = input.allowed_tools.as_deref() {
        validate_tools(tools)?;
    }
    let allowed_tools = input
        .allowed_tools
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|error| AppError::BadRequest(format!("Invalid allowed tools: {error}")))?;

    let updated = sqlx::query_as::<_, Skill>(
        r#"
        UPDATE skills SET
            description = COALESCE($3, description),
            body = COALESCE($4, body),
            allowed_tools = COALESCE($5, allowed_tools),
            model = CASE WHEN $6 THEN $7 ELSE model END,
            context = CASE WHEN $8 THEN $9 ELSE context END,
            user_invocable = COALESCE($10, user_invocable),
            model_invocable = COALESCE($11, model_invocable),
            agent_target = CASE WHEN $12 THEN $13 ELSE agent_target END,
            revision = revision + 1,
            content_hash = md5(
                name || E'\n' || COALESCE($3, description) || E'\n' ||
                COALESCE($4, body) || E'\n' || COALESCE($5, allowed_tools)::text ||
                E'\n' || COALESCE(CASE WHEN $6 THEN $7 ELSE model END, '') ||
                E'\n' || COALESCE(CASE WHEN $8 THEN $9 ELSE context END, '') ||
                E'\n' || COALESCE($10, user_invocable)::text ||
                E'\n' || COALESCE($11, model_invocable)::text ||
                E'\n' || COALESCE(CASE WHEN $12 THEN $13 ELSE agent_target END::text, '')
            ),
            updated_at = NOW()
        WHERE id = $1 AND revision = $2 AND deleted_at IS NULL
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(input.expected_revision)
    .bind(input.description.as_deref().map(str::trim))
    .bind(input.body.as_deref())
    .bind(allowed_tools)
    .bind(input.model.is_some())
    .bind(input.model.as_ref().and_then(|value| value.as_deref()))
    .bind(input.context.is_some())
    .bind(input.context.as_ref().and_then(|value| value.as_deref()))
    .bind(input.user_invocable)
    .bind(input.model_invocable)
    .bind(input.agent_target.is_some())
    .bind(input.agent_target.as_ref().and_then(|value| value.as_ref()))
    .fetch_optional(pool)
    .await?;

    if let Some(skill) = updated {
        return Ok(skill);
    }
    if get(pool, id).await.is_ok() {
        Err(AppError::Conflict(
            "Skill changed in another Aura runtime; refresh and retry".into(),
        ))
    } else {
        Err(AppError::NotFound("Skill not found".into()))
    }
}

pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query(
        "UPDATE skills SET deleted_at = NOW(), updated_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Skill not found".into()));
    }
    Ok(())
}

pub async fn assign(
    pool: &PgPool,
    skill: &Skill,
    agent_id: Uuid,
    created_by: Uuid,
) -> Result<AgentSkillAssignment, AppError> {
    sqlx::query_as::<_, AgentSkillAssignment>(
        r#"
        INSERT INTO agent_skill_assignments (skill_id, agent_id, org_id, created_by)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (skill_id, agent_id) DO UPDATE
            SET created_by = agent_skill_assignments.created_by
        RETURNING *
        "#,
    )
    .bind(skill.id)
    .bind(agent_id)
    .bind(skill.org_id)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

pub async fn unassign(pool: &PgPool, skill_id: Uuid, agent_id: Uuid) -> Result<(), AppError> {
    let result =
        sqlx::query("DELETE FROM agent_skill_assignments WHERE skill_id = $1 AND agent_id = $2")
            .bind(skill_id)
            .bind(agent_id)
            .execute(pool)
            .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Skill assignment not found".into()));
    }
    Ok(())
}

pub async fn list_for_agent(
    pool: &PgPool,
    agent_id: Uuid,
    created_by: Uuid,
    org_id: Option<Uuid>,
) -> Result<Vec<Skill>, AppError> {
    sqlx::query_as::<_, Skill>(
        r#"
        SELECT skills.* FROM skills
        INNER JOIN agent_skill_assignments assignment ON assignment.skill_id = skills.id
        WHERE assignment.agent_id = $1
          AND skills.deleted_at IS NULL
          AND (
            (skills.org_id IS NULL AND skills.created_by = $2)
            OR ($3::uuid IS NOT NULL AND skills.org_id = $3)
          )
        ORDER BY LOWER(skills.name)
        "#,
    )
    .bind(agent_id)
    .bind(created_by)
    .bind(org_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use super::validate_skill_name;

    #[test]
    fn accepts_portable_skill_names() {
        assert!(validate_skill_name("release-check").is_ok());
        assert!(validate_skill_name("skill2").is_ok());
    }

    #[test]
    fn rejects_paths_and_ambiguous_names() {
        for name in ["", "-hidden", "HasCaps", "has space", "../escape", "ends-"] {
            assert!(validate_skill_name(name).is_err(), "{name} should fail");
        }
    }
}
