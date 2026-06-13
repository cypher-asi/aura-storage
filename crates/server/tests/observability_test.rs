//! Integration tests for the private observability ledger.
//!
//! Requires PostgreSQL. Set DATABASE_URL or use the default
//! `postgres://localhost/aura_storage_test`.

use std::net::SocketAddr;

use axum::Router;
use reqwest::Client;
use serde_json::{json, Value};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use aura_storage_auth::{InternalToken, TokenValidator};
use aura_storage_server::router;
use aura_storage_server::state::AppState;

const TEST_INTERNAL_TOKEN: &str = "test-internal-token";
const TEST_COOKIE_SECRET: &str = "test-cookie-secret-for-observability-tests";

async fn spawn_storage_server() -> (SocketAddr, sqlx::PgPool) {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/aura_storage_test".into());

    let pool = aura_storage_db::create_pool(&database_url)
        .await
        .expect("Failed to connect to test database");

    let (events_tx, _) = tokio::sync::broadcast::channel::<String>(256);

    let state = AppState {
        pool: pool.clone(),
        validator: TokenValidator::new(
            "test.auth0.com".into(),
            "test-audience".into(),
            TEST_COOKIE_SECRET.into(),
        ),
        internal_token: InternalToken(TEST_INTERNAL_TOKEN.into()),
        events_tx,
        http_client: reqwest::Client::new(),
        aura_network_url: None,
        aura_network_token: None,
    };

    let app: Router = router::create_router()
        .with_state(state)
        .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024))
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind");
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (addr, pool)
}

fn snapshot(source: &str, environment: &str, status: &str, check_status: &str) -> Value {
    json!({
        "schemaVersion": 1,
        "title": "AURA Observability",
        "generatedAt": "2026-06-12T22:41:55.355Z",
        "environment": environment,
        "source": source,
        "overall": status,
        "totals": {
            "features": 1,
            "operational": if status == "operational" { 1 } else { 0 },
            "degraded": 0,
            "partialOutage": 0,
            "majorOutage": if status == "major_outage" { 1 } else { 0 },
            "unknown": 0,
            "maintenance": 0
        },
        "features": [{
            "id": "remote-agents",
            "label": "Remote Agents",
            "category": "agents",
            "priority": 30,
            "status": status,
            "lastCheckedAt": "2026-06-12T22:40:12.000Z",
            "checksPassed": if check_status == "pass" { 1 } else { 0 },
            "checksTotal": 1,
            "checksFailed": if check_status == "fail" { 1 } else { 0 },
            "checksUnknown": 0,
            "latencyP95Ms": 151454,
            "message": "pod stuck unscheduled",
            "checks": [{
                "id": "remote-agent-runtime",
                "required": false,
                "status": check_status,
                "featureStatus": status,
                "message": "pod stuck unscheduled",
                "checkedAt": "2026-06-12T22:40:12.000Z",
                "latencyMs": 151454,
                "evidence": {
                    "remoteState": "unschedulable"
                }
            }]
        }]
    })
}

#[tokio::test]
async fn observability_internal_endpoints_require_token() {
    let (addr, _pool) = spawn_storage_server().await;
    let client = Client::new();

    let resp = client
        .get(format!("http://{addr}/internal/observability/latest"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn observability_ingest_is_idempotent_and_queryable() {
    let (addr, _pool) = spawn_storage_server().await;
    let client = Client::new();
    let base = format!("http://{addr}");
    let source = format!("test-source-{}", Uuid::new_v4());
    let environment = "test-observability";
    let external_run_id = Uuid::new_v4().to_string();

    let first = json!({
        "source": source,
        "environment": environment,
        "externalRunId": external_run_id,
        "externalRunAttempt": 1,
        "gitSha": "abc123",
        "gitBranch": "main",
        "workflowName": "AURA Observability",
        "snapshot": snapshot(&source, environment, "major_outage", "fail")
    });

    let resp = client
        .post(format!("{base}/internal/observability/runs"))
        .header("X-Internal-Token", TEST_INTERNAL_TOKEN)
        .json(&first)
        .send()
        .await
        .unwrap();
    let status = resp.status();
    let body = resp.text().await.unwrap();
    assert_eq!(status, 200, "{body}");
    let first_run: Value = serde_json::from_str(&body).unwrap();
    let run_id = first_run["id"].as_str().unwrap().to_string();
    assert_eq!(first_run["overallStatus"], "major_outage");

    let second = json!({
        "source": source,
        "environment": environment,
        "externalRunId": external_run_id,
        "externalRunAttempt": 1,
        "gitSha": "def456",
        "gitBranch": "main",
        "workflowName": "AURA Observability",
        "snapshot": snapshot(&source, environment, "operational", "pass")
    });

    let resp = client
        .post(format!("{base}/internal/observability/runs"))
        .header("X-Internal-Token", TEST_INTERNAL_TOKEN)
        .json(&second)
        .send()
        .await
        .unwrap();
    let status = resp.status();
    let body = resp.text().await.unwrap();
    assert_eq!(status, 200, "{body}");
    let second_run: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(second_run["id"], run_id);
    assert_eq!(second_run["overallStatus"], "operational");

    let latest: Value = client
        .get(format!(
            "{base}/internal/observability/latest?source={source}&environment={environment}"
        ))
        .header("X-Internal-Token", TEST_INTERNAL_TOKEN)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(latest["id"], run_id);
    assert_eq!(latest["overallStatus"], "operational");

    let feature_history: Value = client
        .get(format!(
            "{base}/internal/observability/features/remote-agents/history?source={source}&environment={environment}"
        ))
        .header("X-Internal-Token", TEST_INTERNAL_TOKEN)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(feature_history.as_array().unwrap().len(), 1);
    assert_eq!(feature_history[0]["status"], "operational");

    let check_history: Value = client
        .get(format!(
            "{base}/internal/observability/checks/remote-agent-runtime/history?source={source}&environment={environment}"
        ))
        .header("X-Internal-Token", TEST_INTERNAL_TOKEN)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(check_history.as_array().unwrap().len(), 1);
    assert_eq!(check_history[0]["status"], "pass");
    assert_eq!(check_history[0]["startedAt"], Value::Null);
    assert_eq!(check_history[0]["endedAt"], "2026-06-12T22:40:12Z");
    assert_eq!(check_history[0]["evidence"]["remoteState"], "unschedulable");

    let failures: Value = client
        .get(format!(
            "{base}/internal/observability/failures?source={source}&environment={environment}"
        ))
        .header("X-Internal-Token", TEST_INTERNAL_TOKEN)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(failures.as_array().unwrap().is_empty());

    let third = json!({
        "source": source,
        "environment": environment,
        "externalRunId": Uuid::new_v4().to_string(),
        "externalRunAttempt": 1,
        "gitSha": "fed789",
        "gitBranch": "main",
        "workflowName": "AURA Observability",
        "snapshot": snapshot(&source, environment, "major_outage", "fail")
    });
    let resp = client
        .post(format!("{base}/internal/observability/runs"))
        .header("X-Internal-Token", TEST_INTERNAL_TOKEN)
        .json(&third)
        .send()
        .await
        .unwrap();
    let status = resp.status();
    let body = resp.text().await.unwrap();
    assert_eq!(status, 200, "{body}");
    let third_run: Value = serde_json::from_str(&body).unwrap();
    assert_ne!(third_run["id"], run_id);

    let latest: Value = client
        .get(format!(
            "{base}/internal/observability/latest?source={source}&environment={environment}"
        ))
        .header("X-Internal-Token", TEST_INTERNAL_TOKEN)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(latest["id"], third_run["id"]);
    assert_eq!(latest["overallStatus"], "major_outage");

    let top_failures: Value = client
        .get(format!(
            "{base}/internal/observability/failures/top?source={source}&environment={environment}"
        ))
        .header("X-Internal-Token", TEST_INTERNAL_TOKEN)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(top_failures.as_array().unwrap().len(), 1);
    assert_eq!(top_failures[0]["checkId"], "remote-agent-runtime");
    assert_eq!(top_failures[0]["featureId"], "remote-agents");
    assert_eq!(top_failures[0]["failureCount"], 1);
    assert_eq!(top_failures[0]["latestStatus"], "fail");
}
