//! Integration tests for the stats-data-integrity fixes:
//!   * `sessions.increment_tokens` (atomic per-call accounting)
//!   * `tasks.transition` populating `started_at` / `ended_at`
//!   * `session_cleanup::close_orphans` background sweep
//!   * Stats response exposing `total_task_time_seconds`
//!
//! Requires PostgreSQL. Set `DATABASE_URL` (default
//! `postgres://localhost/aura_storage_test`) and `createdb aura_storage_test`
//! before running.

use chrono::{Duration as ChronoDuration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use aura_storage_sessions::{models as session_models, repo as session_repo};
use aura_storage_server::jobs::session_cleanup;
use aura_storage_tasks::{models as task_models, repo as task_repo};

async fn pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/aura_storage_test".into());
    aura_storage_db::create_pool(&database_url)
        .await
        .expect("Failed to connect to test database")
}

/// Inserts a project_agent with random ids so test rows don't collide between
/// runs. Returns (project_agent_id, project_id, created_by).
async fn seed_project_agent(pool: &PgPool) -> (Uuid, Uuid, Uuid) {
    let project_id = Uuid::new_v4();
    let created_by = Uuid::new_v4();
    let project_agent_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO project_agents (id, project_id, created_by, agent_id, status)
        VALUES ($1, $2, $3, $4, 'idle')
        "#,
    )
    .bind(project_agent_id)
    .bind(project_id)
    .bind(created_by)
    .bind(Uuid::new_v4())
    .execute(pool)
    .await
    .expect("Failed to seed project agent");

    (project_agent_id, project_id, created_by)
}

async fn seed_spec(pool: &PgPool, project_id: Uuid, created_by: Uuid) -> Uuid {
    let spec_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO specs (id, project_id, created_by, title, order_index, markdown_contents)
        VALUES ($1, $2, $3, 'test-spec', 0, 'spec content')
        "#,
    )
    .bind(spec_id)
    .bind(project_id)
    .bind(created_by)
    .execute(pool)
    .await
    .expect("Failed to seed spec");
    spec_id
}

#[tokio::test]
async fn increment_tokens_accumulates() {
    let pool = pool().await;
    let (project_agent_id, project_id, created_by) = seed_project_agent(&pool).await;

    let req = session_models::CreateSessionRequest {
        project_id,
        org_id: None,
        model: Some("test-model".into()),
    };
    let session = session_repo::create(&pool, project_agent_id, created_by, &req)
        .await
        .expect("create session");

    assert_eq!(session.total_input_tokens, 0);
    assert_eq!(session.total_output_tokens, 0);

    let after_one = session_repo::increment_tokens(&pool, session.id, 100, 50)
        .await
        .expect("increment 1");
    assert_eq!(after_one.total_input_tokens, 100);
    assert_eq!(after_one.total_output_tokens, 50);

    let after_two = session_repo::increment_tokens(&pool, session.id, 25, 75)
        .await
        .expect("increment 2");
    assert_eq!(after_two.total_input_tokens, 125);
    assert_eq!(after_two.total_output_tokens, 125);
}

#[tokio::test]
async fn increment_tokens_concurrent_no_lost_updates() {
    let pool = pool().await;
    let (project_agent_id, project_id, created_by) = seed_project_agent(&pool).await;

    let req = session_models::CreateSessionRequest {
        project_id,
        org_id: None,
        model: None,
    };
    let session = session_repo::create(&pool, project_agent_id, created_by, &req)
        .await
        .expect("create session");

    let id = session.id;
    let mut handles = Vec::new();
    for _ in 0..20 {
        let p = pool.clone();
        handles.push(tokio::spawn(async move {
            session_repo::increment_tokens(&p, id, 10, 5).await
        }));
    }
    for h in handles {
        h.await.expect("join").expect("increment");
    }

    let final_state = session_repo::get(&pool, id).await.expect("get");
    assert_eq!(final_state.total_input_tokens, 200);
    assert_eq!(final_state.total_output_tokens, 100);
}

#[tokio::test]
async fn task_transition_stamps_timestamps() {
    let pool = pool().await;
    let (_, project_id, created_by) = seed_project_agent(&pool).await;
    let spec_id = seed_spec(&pool, project_id, created_by).await;

    let create_req = task_models::CreateTaskRequest {
        org_id: None,
        spec_id,
        title: "task time test".into(),
        description: None,
        order_index: 0,
        dependency_task_ids: None,
        parent_task_id: None,
        assigned_project_agent_id: None,
    };
    let task = task_repo::create(&pool, project_id, created_by, &create_req)
        .await
        .expect("create task");
    assert!(task.started_at.is_none());
    assert!(task.ended_at.is_none());

    // pending → ready: neither timestamp set
    let task = task_repo::transition(
        &pool,
        task.id,
        &task_models::TransitionRequest {
            status: "ready".into(),
        },
    )
    .await
    .expect("ready");
    assert!(task.started_at.is_none());
    assert!(task.ended_at.is_none());

    // ready → in_progress: started_at set
    let task = task_repo::transition(
        &pool,
        task.id,
        &task_models::TransitionRequest {
            status: "in_progress".into(),
        },
    )
    .await
    .expect("in_progress");
    let started_at_first = task.started_at.expect("started_at populated");
    assert!(task.ended_at.is_none());

    // in_progress → done: ended_at set, started_at preserved
    let task = task_repo::transition(
        &pool,
        task.id,
        &task_models::TransitionRequest {
            status: "done".into(),
        },
    )
    .await
    .expect("done");
    assert_eq!(task.started_at, Some(started_at_first));
    assert!(task.ended_at.is_some());
    assert!(task.ended_at.unwrap() >= started_at_first);
}

#[tokio::test]
async fn task_transition_failed_path_stamps_ended_at() {
    let pool = pool().await;
    let (_, project_id, created_by) = seed_project_agent(&pool).await;
    let spec_id = seed_spec(&pool, project_id, created_by).await;

    let task = task_repo::create(
        &pool,
        project_id,
        created_by,
        &task_models::CreateTaskRequest {
            org_id: None,
            spec_id,
            title: "failure path".into(),
            description: None,
            order_index: 1,
            dependency_task_ids: None,
            parent_task_id: None,
            assigned_project_agent_id: None,
        },
    )
    .await
    .expect("create");

    for status in ["ready", "in_progress", "failed"] {
        task_repo::transition(
            &pool,
            task.id,
            &task_models::TransitionRequest {
                status: status.into(),
            },
        )
        .await
        .expect(status);
    }

    let final_state = task_repo::get(&pool, task.id).await.expect("get");
    assert!(final_state.started_at.is_some());
    assert!(final_state.ended_at.is_some());
}

#[tokio::test]
async fn close_orphans_only_closes_old_active_sessions() {
    let pool = pool().await;
    let (project_agent_id, project_id, created_by) = seed_project_agent(&pool).await;

    // Stale session: started 7 hours ago, still active.
    let stale_id = Uuid::new_v4();
    let stale_started = Utc::now() - ChronoDuration::hours(7);
    sqlx::query(
        r#"
        INSERT INTO sessions (id, project_agent_id, project_id, created_by, status, started_at)
        VALUES ($1, $2, $3, $4, 'active', $5)
        "#,
    )
    .bind(stale_id)
    .bind(project_agent_id)
    .bind(project_id)
    .bind(created_by)
    .bind(stale_started)
    .execute(&pool)
    .await
    .expect("insert stale");

    // Recent session: started 1 hour ago, still active. Must not be closed.
    let recent_id = Uuid::new_v4();
    let recent_started = Utc::now() - ChronoDuration::hours(1);
    sqlx::query(
        r#"
        INSERT INTO sessions (id, project_agent_id, project_id, created_by, status, started_at)
        VALUES ($1, $2, $3, $4, 'active', $5)
        "#,
    )
    .bind(recent_id)
    .bind(project_agent_id)
    .bind(project_id)
    .bind(created_by)
    .bind(recent_started)
    .execute(&pool)
    .await
    .expect("insert recent");

    // Already-closed session: must not be touched.
    let closed_id = Uuid::new_v4();
    let closed_started = Utc::now() - ChronoDuration::hours(8);
    let closed_ended = Utc::now() - ChronoDuration::hours(7);
    sqlx::query(
        r#"
        INSERT INTO sessions (id, project_agent_id, project_id, created_by, status, started_at, ended_at)
        VALUES ($1, $2, $3, $4, 'completed', $5, $6)
        "#,
    )
    .bind(closed_id)
    .bind(project_agent_id)
    .bind(project_id)
    .bind(created_by)
    .bind(closed_started)
    .bind(closed_ended)
    .execute(&pool)
    .await
    .expect("insert closed");

    let n = session_cleanup::close_orphans(&pool, 6)
        .await
        .expect("cleanup");
    assert!(n >= 1, "expected at least the stale session to close");

    let stale = session_repo::get(&pool, stale_id).await.expect("get stale");
    assert_eq!(stale.status, "failed");
    assert!(stale.ended_at.is_some());

    let recent = session_repo::get(&pool, recent_id).await.expect("get recent");
    assert_eq!(recent.status, "active");
    assert!(recent.ended_at.is_none());

    let closed = session_repo::get(&pool, closed_id).await.expect("get closed");
    assert_eq!(closed.status, "completed");
}

#[tokio::test]
async fn stats_tokens_come_from_tasks_not_sessions() {
    // The dashboard "Tokens" stat must reflect tokens written to
    // `tasks.total_input_tokens` / `total_output_tokens` (where
    // aura-os-server's `persist_task_output` lands them under the
    // current dev-loop architecture). Tokens written to the
    // `sessions` table must NOT contribute to this stat under the new
    // architecture; nothing in production writes to sessions tokens
    // today, but historical rows could remain and shouldn't double-
    // count if this query is ever later changed.
    use aura_storage_server::handlers::stats::{get_stats_inner, StatsQuery};

    let pool = pool().await;
    let (project_agent_id, project_id, created_by) = seed_project_agent(&pool).await;
    let spec_id = seed_spec(&pool, project_id, created_by).await;

    // Seed a session with token totals that MUST NOT show up in the
    // stats response (these would be the historical pre-refactor data).
    sqlx::query(
        r#"
        INSERT INTO sessions (id, project_agent_id, project_id, created_by, status,
                              total_input_tokens, total_output_tokens, started_at, ended_at)
        VALUES (gen_random_uuid(), $1, $2, $3, 'completed', 99999, 99999, NOW(), NOW())
        "#,
    )
    .bind(project_agent_id)
    .bind(project_id)
    .bind(created_by)
    .execute(&pool)
    .await
    .expect("seed session");

    // Seed two completed tasks with token totals — these ARE what the
    // stats query should sum.
    let task_a = task_repo::create(
        &pool,
        project_id,
        created_by,
        &task_models::CreateTaskRequest {
            org_id: None,
            spec_id,
            title: "task A".into(),
            description: None,
            order_index: 0,
            dependency_task_ids: None,
            parent_task_id: None,
            assigned_project_agent_id: Some(project_agent_id),
        },
    )
    .await
    .expect("create task A");
    sqlx::query(
        "UPDATE tasks SET total_input_tokens = 1000, total_output_tokens = 500 WHERE id = $1",
    )
    .bind(task_a.id)
    .execute(&pool)
    .await
    .expect("set tokens A");

    let task_b = task_repo::create(
        &pool,
        project_id,
        created_by,
        &task_models::CreateTaskRequest {
            org_id: None,
            spec_id,
            title: "task B".into(),
            description: None,
            order_index: 1,
            dependency_task_ids: None,
            parent_task_id: None,
            assigned_project_agent_id: Some(project_agent_id),
        },
    )
    .await
    .expect("create task B");
    sqlx::query(
        "UPDATE tasks SET total_input_tokens = 200, total_output_tokens = 100 WHERE id = $1",
    )
    .bind(task_b.id)
    .execute(&pool)
    .await
    .expect("set tokens B");

    // Run the stats endpoint as the API surface does.
    let query = StatsQuery {
        scope: "project".into(),
        project_id: Some(project_id),
        org_id: None,
        agent_id: None,
    };
    let response = get_stats_inner(&pool, &reqwest::Client::new(), None, None, query)
        .await
        .expect("stats query");
    let stats = response.0;

    // Tasks-only sum: 1000 + 200 = 1200 input, 500 + 100 = 600 output.
    // The session row's 99999 tokens MUST be ignored.
    assert_eq!(
        stats.total_input_tokens, 1200,
        "input tokens must come from tasks (1000 + 200), NOT sessions (99999)"
    );
    assert_eq!(
        stats.total_output_tokens, 600,
        "output tokens must come from tasks (500 + 100), NOT sessions (99999)"
    );
    assert_eq!(stats.total_tokens, 1800);

    // Cleanup so this test doesn't pollute other tests' aggregations.
    sqlx::query("DELETE FROM tasks WHERE project_id = $1")
        .bind(project_id)
        .execute(&pool)
        .await
        .expect("cleanup tasks");
    sqlx::query("DELETE FROM sessions WHERE project_id = $1")
        .bind(project_id)
        .execute(&pool)
        .await
        .expect("cleanup sessions");
    sqlx::query("DELETE FROM specs WHERE project_id = $1")
        .bind(project_id)
        .execute(&pool)
        .await
        .expect("cleanup specs");
    sqlx::query("DELETE FROM project_agents WHERE project_id = $1")
        .bind(project_id)
        .execute(&pool)
        .await
        .expect("cleanup project_agents");
}
