use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use aura_storage_core::AppError;

use crate::models::{
    IngestObservabilityRunRequest, ObservabilityCheckResult, ObservabilityFailureQuery,
    ObservabilityFailureSummary, ObservabilityFeatureResult, ObservabilityHistoryQuery,
    ObservabilityRun,
};

const FEATURE_STATUSES: &[&str] = &[
    "operational",
    "degraded",
    "partial_outage",
    "major_outage",
    "unknown",
    "maintenance",
];

const CHECK_STATUSES: &[&str] = &["pass", "warn", "fail", "skip", "unknown"];

struct NormalizedRun {
    source: String,
    environment: String,
    external_run_id: Option<String>,
    external_run_attempt: i32,
    git_sha: Option<String>,
    git_branch: Option<String>,
    workflow_name: Option<String>,
    release_channel: Option<String>,
    generated_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    overall_status: String,
    totals: Value,
    snapshot: Value,
}

pub async fn ingest(
    pool: &PgPool,
    input: &IngestObservabilityRunRequest,
) -> Result<ObservabilityRun, AppError> {
    let normalized = normalize_run(input)?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Internal(format!("failed to begin transaction: {e}")))?;

    let run = if normalized.external_run_id.is_some() {
        sqlx::query_as::<_, ObservabilityRun>(
            r#"
            INSERT INTO observability_runs (
                source, environment, external_run_id, external_run_attempt,
                git_sha, git_branch, workflow_name, release_channel,
                generated_at, started_at, completed_at, overall_status, totals, snapshot
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            ON CONFLICT (source, external_run_id, external_run_attempt)
                WHERE external_run_id IS NOT NULL
            DO UPDATE SET
                environment = EXCLUDED.environment,
                git_sha = EXCLUDED.git_sha,
                git_branch = EXCLUDED.git_branch,
                workflow_name = EXCLUDED.workflow_name,
                release_channel = EXCLUDED.release_channel,
                generated_at = EXCLUDED.generated_at,
                started_at = EXCLUDED.started_at,
                completed_at = EXCLUDED.completed_at,
                overall_status = EXCLUDED.overall_status,
                totals = EXCLUDED.totals,
                snapshot = EXCLUDED.snapshot
            RETURNING *
            "#,
        )
        .bind(&normalized.source)
        .bind(&normalized.environment)
        .bind(&normalized.external_run_id)
        .bind(normalized.external_run_attempt)
        .bind(&normalized.git_sha)
        .bind(&normalized.git_branch)
        .bind(&normalized.workflow_name)
        .bind(&normalized.release_channel)
        .bind(normalized.generated_at)
        .bind(normalized.started_at)
        .bind(normalized.completed_at)
        .bind(&normalized.overall_status)
        .bind(&normalized.totals)
        .bind(&normalized.snapshot)
        .fetch_one(&mut *tx)
        .await?
    } else {
        sqlx::query_as::<_, ObservabilityRun>(
            r#"
            INSERT INTO observability_runs (
                source, environment, external_run_attempt,
                git_sha, git_branch, workflow_name, release_channel,
                generated_at, started_at, completed_at, overall_status, totals, snapshot
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING *
            "#,
        )
        .bind(&normalized.source)
        .bind(&normalized.environment)
        .bind(normalized.external_run_attempt)
        .bind(&normalized.git_sha)
        .bind(&normalized.git_branch)
        .bind(&normalized.workflow_name)
        .bind(&normalized.release_channel)
        .bind(normalized.generated_at)
        .bind(normalized.started_at)
        .bind(normalized.completed_at)
        .bind(&normalized.overall_status)
        .bind(&normalized.totals)
        .bind(&normalized.snapshot)
        .fetch_one(&mut *tx)
        .await?
    };

    sqlx::query("DELETE FROM observability_check_results WHERE run_id = $1")
        .bind(run.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM observability_feature_results WHERE run_id = $1")
        .bind(run.id)
        .execute(&mut *tx)
        .await?;

    let features = normalized
        .snapshot
        .get("features")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::BadRequest("snapshot.features must be an array".into()))?;

    for feature in features {
        insert_feature(&mut tx, run.id, feature).await?;
        if let Some(checks) = feature.get("checks").and_then(Value::as_array) {
            let feature_id = required_string(feature, "id", "feature.id")?;
            for check in checks {
                insert_check(&mut tx, run.id, &feature_id, feature, check).await?;
            }
        }
    }

    tx.commit()
        .await
        .map_err(|e| AppError::Internal(format!("failed to commit transaction: {e}")))?;

    Ok(run)
}

async fn insert_feature(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    run_id: Uuid,
    feature: &Value,
) -> Result<(), AppError> {
    let feature_id = required_string(feature, "id", "feature.id")?;
    let label = optional_string(feature, "label").unwrap_or_else(|| feature_id.clone());
    let category = optional_string(feature, "category").unwrap_or_else(|| "uncategorized".into());
    let status = required_status(feature, "status", FEATURE_STATUSES, "feature.status")?;

    sqlx::query(
        r#"
        INSERT INTO observability_feature_results (
            run_id, feature_id, label, category, priority, status, last_checked_at,
            checks_passed, checks_total, checks_failed, checks_unknown,
            latency_p95_ms, message, feature
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        "#,
    )
    .bind(run_id)
    .bind(feature_id)
    .bind(label)
    .bind(category)
    .bind(optional_i32(feature, "priority").unwrap_or(999))
    .bind(status)
    .bind(optional_datetime(feature, "lastCheckedAt")?)
    .bind(optional_i32(feature, "checksPassed").unwrap_or(0))
    .bind(optional_i32(feature, "checksTotal").unwrap_or(0))
    .bind(optional_i32(feature, "checksFailed").unwrap_or(0))
    .bind(optional_i32(feature, "checksUnknown").unwrap_or(0))
    .bind(optional_i32(feature, "latencyP95Ms"))
    .bind(optional_string(feature, "message").unwrap_or_default())
    .bind(feature)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn insert_check(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    run_id: Uuid,
    feature_id: &str,
    feature: &Value,
    check: &Value,
) -> Result<(), AppError> {
    let check_id = required_string(check, "id", "check.id")?;
    let status = required_status(check, "status", CHECK_STATUSES, "check.status")?;
    let feature_status = optional_string(check, "featureStatus")
        .or_else(|| optional_string(feature, "status"))
        .ok_or_else(|| AppError::BadRequest("check.featureStatus is missing".into()))?;
    validate_status(&feature_status, FEATURE_STATUSES, "check.featureStatus")?;
    let started_at = optional_datetime(check, "startedAt")?;
    let ended_at = optional_datetime(check, "endedAt")?.or(optional_datetime(check, "checkedAt")?);

    sqlx::query(
        r#"
        INSERT INTO observability_check_results (
            run_id, feature_id, check_id, required, status, feature_status,
            message, started_at, ended_at, latency_ms, evidence, check_payload
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
    )
    .bind(run_id)
    .bind(feature_id)
    .bind(check_id)
    .bind(
        check
            .get("required")
            .and_then(Value::as_bool)
            .unwrap_or(true),
    )
    .bind(status)
    .bind(feature_status)
    .bind(optional_string(check, "message").unwrap_or_default())
    .bind(started_at)
    .bind(ended_at)
    .bind(optional_i32(check, "latencyMs"))
    .bind(
        check
            .get("evidence")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({})),
    )
    .bind(check)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn latest(
    pool: &PgPool,
    source: Option<&str>,
    environment: Option<&str>,
) -> Result<ObservabilityRun, AppError> {
    sqlx::query_as::<_, ObservabilityRun>(
        r#"
        SELECT * FROM observability_runs
        WHERE ($1::TEXT IS NULL OR source = $1)
          AND ($2::TEXT IS NULL OR environment = $2)
        ORDER BY generated_at DESC, created_at DESC
        LIMIT 1
        "#,
    )
    .bind(source)
    .bind(environment)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Observability run not found".into()))
}

pub async fn list_runs(
    pool: &PgPool,
    query: &ObservabilityHistoryQuery,
) -> Result<Vec<ObservabilityRun>, AppError> {
    sqlx::query_as::<_, ObservabilityRun>(
        r#"
        SELECT * FROM observability_runs
        WHERE ($1::TEXT IS NULL OR source = $1)
          AND ($2::TEXT IS NULL OR environment = $2)
        ORDER BY generated_at DESC, created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(query.source.as_deref())
    .bind(query.environment.as_deref())
    .bind(query.limit())
    .bind(query.offset())
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn feature_history(
    pool: &PgPool,
    feature_id: &str,
    query: &ObservabilityHistoryQuery,
) -> Result<Vec<ObservabilityFeatureResult>, AppError> {
    sqlx::query_as::<_, ObservabilityFeatureResult>(
        r#"
        SELECT f.* FROM observability_feature_results f
        JOIN observability_runs r ON r.id = f.run_id
        WHERE f.feature_id = $1
          AND ($2::TEXT IS NULL OR r.source = $2)
          AND ($3::TEXT IS NULL OR r.environment = $3)
        ORDER BY r.generated_at DESC, r.created_at DESC, f.created_at DESC
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(feature_id)
    .bind(query.source.as_deref())
    .bind(query.environment.as_deref())
    .bind(query.limit())
    .bind(query.offset())
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn check_history(
    pool: &PgPool,
    check_id: &str,
    query: &ObservabilityHistoryQuery,
) -> Result<Vec<ObservabilityCheckResult>, AppError> {
    sqlx::query_as::<_, ObservabilityCheckResult>(
        r#"
        SELECT c.* FROM observability_check_results c
        JOIN observability_runs r ON r.id = c.run_id
        WHERE c.check_id = $1
          AND ($2::TEXT IS NULL OR r.source = $2)
          AND ($3::TEXT IS NULL OR r.environment = $3)
        ORDER BY COALESCE(c.ended_at, r.generated_at) DESC, r.created_at DESC, c.created_at DESC
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(check_id)
    .bind(query.source.as_deref())
    .bind(query.environment.as_deref())
    .bind(query.limit())
    .bind(query.offset())
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn recent_failures(
    pool: &PgPool,
    query: &ObservabilityFailureQuery,
) -> Result<Vec<ObservabilityCheckResult>, AppError> {
    sqlx::query_as::<_, ObservabilityCheckResult>(
        r#"
        SELECT c.* FROM observability_check_results c
        JOIN observability_runs r ON r.id = c.run_id
        WHERE c.status IN ('warn','fail','unknown')
          AND ($1::TEXT IS NULL OR r.source = $1)
          AND ($2::TEXT IS NULL OR r.environment = $2)
        ORDER BY COALESCE(c.ended_at, r.generated_at) DESC, r.created_at DESC, c.created_at DESC
        LIMIT $3
        "#,
    )
    .bind(query.source.as_deref())
    .bind(query.environment.as_deref())
    .bind(query.limit())
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

pub async fn top_failures(
    pool: &PgPool,
    query: &ObservabilityFailureQuery,
) -> Result<Vec<ObservabilityFailureSummary>, AppError> {
    sqlx::query_as::<_, ObservabilityFailureSummary>(
        r#"
        SELECT
            c.check_id,
            (ARRAY_AGG(c.feature_id ORDER BY COALESCE(c.ended_at, r.generated_at) DESC, r.created_at DESC, c.created_at DESC))[1] AS feature_id,
            COUNT(*) AS failure_count,
            (ARRAY_AGG(c.status ORDER BY COALESCE(c.ended_at, r.generated_at) DESC, r.created_at DESC, c.created_at DESC))[1] AS latest_status,
            COALESCE((ARRAY_AGG(c.message ORDER BY COALESCE(c.ended_at, r.generated_at) DESC, r.created_at DESC, c.created_at DESC))[1], '') AS latest_message,
            MAX(COALESCE(c.ended_at, r.generated_at)) AS latest_seen_at,
            MAX(c.latency_ms) AS max_latency_ms
        FROM observability_check_results c
        JOIN observability_runs r ON r.id = c.run_id
        WHERE c.status IN ('warn','fail','unknown')
          AND ($1::TEXT IS NULL OR r.source = $1)
          AND ($2::TEXT IS NULL OR r.environment = $2)
        GROUP BY c.check_id
        ORDER BY failure_count DESC, latest_seen_at DESC
        LIMIT $3
        "#,
    )
    .bind(query.source.as_deref())
    .bind(query.environment.as_deref())
    .bind(query.limit())
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

fn normalize_run(input: &IngestObservabilityRunRequest) -> Result<NormalizedRun, AppError> {
    if !input.snapshot.is_object() {
        return Err(AppError::BadRequest(
            "snapshot must be a JSON object".into(),
        ));
    }
    if !input
        .snapshot
        .get("features")
        .and_then(Value::as_array)
        .map(|features| !features.is_empty())
        .unwrap_or(false)
    {
        return Err(AppError::BadRequest(
            "snapshot.features must be a non-empty array".into(),
        ));
    }

    let source = input
        .source
        .as_deref()
        .map(trimmed_string)
        .or_else(|| optional_string(&input.snapshot, "source"))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".into());
    let environment = input
        .environment
        .as_deref()
        .map(trimmed_string)
        .or_else(|| optional_string(&input.snapshot, "environment"))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".into());
    let generated_at = input
        .generated_at
        .or(optional_datetime(&input.snapshot, "generatedAt")?)
        .ok_or_else(|| AppError::BadRequest("generatedAt is required".into()))?;
    let overall_status = input
        .overall_status
        .as_deref()
        .map(trimmed_string)
        .or_else(|| optional_string(&input.snapshot, "overall"))
        .ok_or_else(|| AppError::BadRequest("overall status is required".into()))?;
    validate_status(&overall_status, FEATURE_STATUSES, "overallStatus")?;

    Ok(NormalizedRun {
        source,
        environment,
        external_run_id: input
            .external_run_id
            .as_deref()
            .map(trimmed_string)
            .filter(|value| !value.is_empty()),
        external_run_attempt: input.external_run_attempt.unwrap_or(1).max(1),
        git_sha: non_empty_input_string(input.git_sha.as_deref()),
        git_branch: non_empty_input_string(input.git_branch.as_deref()),
        workflow_name: non_empty_input_string(input.workflow_name.as_deref()),
        release_channel: non_empty_input_string(input.release_channel.as_deref()),
        generated_at,
        started_at: input.started_at,
        completed_at: input.completed_at,
        overall_status,
        totals: input
            .totals
            .clone()
            .or_else(|| input.snapshot.get("totals").cloned())
            .unwrap_or_else(|| serde_json::json!({})),
        snapshot: input.snapshot.clone(),
    })
}

fn required_string(value: &Value, key: &str, label: &str) -> Result<String, AppError> {
    optional_string(value, key)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::BadRequest(format!("{label} is required")))
}

fn optional_string(value: &Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(trimmed_string)
}

fn trimmed_string(value: &str) -> String {
    value.trim().to_string()
}

fn non_empty_input_string(value: Option<&str>) -> Option<String> {
    value.map(trimmed_string).filter(|value| !value.is_empty())
}

fn optional_i32(value: &Value, key: &str) -> Option<i32> {
    let number = value.get(key)?.as_i64()?;
    i32::try_from(number).ok()
}

fn optional_datetime(value: &Value, key: &str) -> Result<Option<DateTime<Utc>>, AppError> {
    let Some(raw) = value.get(key).and_then(Value::as_str) else {
        return Ok(None);
    };
    if raw.trim().is_empty() {
        return Ok(None);
    }
    DateTime::parse_from_rfc3339(raw)
        .map(|value| Some(value.with_timezone(&Utc)))
        .map_err(|_| AppError::BadRequest(format!("{key} must be an RFC3339 timestamp")))
}

fn required_status(
    value: &Value,
    key: &str,
    allowed: &[&str],
    label: &str,
) -> Result<String, AppError> {
    let status = required_string(value, key, label)?;
    validate_status(&status, allowed, label)?;
    Ok(status)
}

fn validate_status(status: &str, allowed: &[&str], label: &str) -> Result<(), AppError> {
    if allowed.contains(&status) {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!(
            "{label} must be one of: {}",
            allowed.join(", ")
        )))
    }
}
