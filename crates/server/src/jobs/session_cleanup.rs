//! Periodic orphan-session cleanup.
//!
//! Sessions whose dev-loop crashed or disconnected before sending a close
//! event stay `status='active'` with `ended_at IS NULL` forever — they pollute
//! the "active" count and they never receive their final token totals via the
//! per-call increment path. This job force-closes any session that has been
//! idle past a configurable threshold.
//!
//! Tuning via env:
//!   SESSION_CLEANUP_INTERVAL_SECS  (default 1800 = 30 min)
//!   SESSION_CLEANUP_THRESHOLD_HOURS (default 6)

use sqlx::PgPool;
use std::time::Duration;

const DEFAULT_INTERVAL_SECS: u64 = 1800;
const DEFAULT_THRESHOLD_HOURS: i64 = 6;

pub fn spawn(pool: PgPool) {
    let interval_secs = std::env::var("SESSION_CLEANUP_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_INTERVAL_SECS);
    let threshold_hours = std::env::var("SESSION_CLEANUP_THRESHOLD_HOURS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_THRESHOLD_HOURS);

    tracing::info!(
        interval_secs,
        threshold_hours,
        "Spawning orphan-session cleanup task"
    );

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
        // First tick fires immediately; skip it so startup doesn't trigger a sweep.
        ticker.tick().await;

        loop {
            ticker.tick().await;
            match close_orphans(&pool, threshold_hours).await {
                Ok(0) => {}
                Ok(n) => tracing::info!(closed = n, "Orphan-session cleanup closed sessions"),
                Err(e) => tracing::error!(error = ?e, "Orphan-session cleanup failed"),
            }
        }
    });
}

/// Marks all `status='active'` sessions older than `threshold_hours` as
/// `failed` with `ended_at = NOW()`. Returns the number of rows updated.
pub async fn close_orphans(pool: &PgPool, threshold_hours: i64) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE sessions
           SET status = 'failed',
               ended_at = NOW()
         WHERE status = 'active'
           AND ended_at IS NULL
           AND started_at < NOW() - make_interval(hours => $1)
        "#,
    )
    .bind(threshold_hours as i32)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
