/// Integration tests for process endpoints.
///
/// Requires PostgreSQL. Set DATABASE_URL or uses default.
/// Create test DB: `createdb aura_storage_test`
use std::net::SocketAddr;

use axum::{extract::Path, http::StatusCode, routing::get, Router};
use reqwest::Client;
use serde_json::{json, Value};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use aura_storage_auth::{InternalToken, TokenValidator};
use aura_storage_processes::{models as process_models, repo as process_repo};
use aura_storage_server::router;
use aura_storage_server::state::AppState;

const TEST_INTERNAL_TOKEN: &str = "test-internal-token";
const TEST_COOKIE_SECRET: &str = "test-cookie-secret-for-process-tests";

const SELF_SIGNED_KID: &str = "jFNXMnFjGrSoDafnLQBohoCNalWcFcTjnKEbkRzWFBHyYJFikdLMHP";

#[derive(Clone)]
struct MockNetworkState {
    allow: bool,
}

fn generate_test_jwt() -> String {
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use serde::Serialize;

    #[derive(Serialize)]
    struct Claims {
        sub: String,
        id: String,
        iat: i64,
        exp: i64,
    }

    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some(SELF_SIGNED_KID.to_string());

    let claims = Claims {
        sub: "auth0|testuser".into(),
        id: "f3f83d3a-d7b5-4320-a1ec-ff27495e7292".into(),
        iat: chrono::Utc::now().timestamp(),
        exp: chrono::Utc::now().timestamp() + 3600,
    };

    encode(
        &header,
        &claims,
        &EncodingKey::from_secret(TEST_COOKIE_SECRET.as_bytes()),
    )
    .unwrap()
}

async fn spawn_network_mock(allow: bool) -> String {
    async fn get_org(
        Path(_org_id): Path<Uuid>,
        axum::extract::State(state): axum::extract::State<MockNetworkState>,
        headers: axum::http::HeaderMap,
    ) -> StatusCode {
        let has_bearer = headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.starts_with("Bearer "))
            .unwrap_or(false);

        if !has_bearer {
            return StatusCode::UNAUTHORIZED;
        }

        if state.allow {
            StatusCode::OK
        } else {
            StatusCode::FORBIDDEN
        }
    }

    let app = Router::new()
        .route("/api/orgs/:orgId", get(get_org))
        .with_state(MockNetworkState { allow });

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind network mock");
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    format!("http://{addr}")
}

async fn spawn_storage_server(network_url: Option<String>) -> (SocketAddr, sqlx::PgPool) {
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
        aura_network_url: network_url,
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

async fn spawn_test_server() -> (SocketAddr, sqlx::PgPool) {
    spawn_storage_server(None).await
}

async fn spawn_authed_test_server(allow: bool) -> (SocketAddr, sqlx::PgPool) {
    let network_url = spawn_network_mock(allow).await;
    spawn_storage_server(Some(network_url)).await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn health_check() {
    let (addr, _pool) = spawn_test_server().await;
    let client = Client::new();

    let resp = client
        .get(format!("http://{addr}/health"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn public_endpoints_reject_without_jwt() {
    let (addr, _pool) = spawn_test_server().await;
    let client = Client::new();
    let org_id = Uuid::new_v4();

    let resp = client
        .get(format!("http://{addr}/api/processes?orgId={org_id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn internal_endpoints_reject_without_token() {
    let (addr, _pool) = spawn_test_server().await;
    let client = Client::new();
    let id = Uuid::new_v4();

    let resp = client
        .get(format!("http://{addr}/internal/processes/{id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn internal_endpoints_reject_wrong_token() {
    let (addr, _pool) = spawn_test_server().await;
    let client = Client::new();
    let id = Uuid::new_v4();

    let resp = client
        .get(format!("http://{addr}/internal/processes/{id}"))
        .header("X-Internal-Token", "wrong-token")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn full_process_lifecycle_via_public_writes() {
    let (addr, _pool) = spawn_authed_test_server(true).await;
    let client = Client::new();
    let base = format!("http://{addr}");
    let jwt = generate_test_jwt();
    let org_id = Uuid::new_v4();
    let project_id = Uuid::new_v4();

    // Create process via public API
    let resp = client
        .post(format!("{base}/api/processes"))
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&json!({
            "orgId": org_id,
            "projectId": project_id,
            "name": "Test Process",
            "description": "A test workflow"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let process: Value = resp.json().await.unwrap();
    let process_id = process["id"].as_str().unwrap();
    assert_eq!(process["name"], "Test Process");
    assert_eq!(process["enabled"], true);

    // Get process via public API
    let resp = client
        .get(format!("{base}/api/processes/{process_id}"))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let got: Value = resp.json().await.unwrap();
    assert_eq!(got["name"], "Test Process");

    // Update process via public API
    let resp = client
        .put(format!("{base}/api/processes/{process_id}"))
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&json!({"name": "Updated Process", "description": "Updated desc"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let updated: Value = resp.json().await.unwrap();
    assert_eq!(updated["name"], "Updated Process");

    // List processes (should have 1)
    let resp = client
        .get(format!("{base}/api/processes?orgId={org_id}"))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list: Vec<Value> = resp.json().await.unwrap();
    assert_eq!(list.len(), 1);

    // Get process via internal
    let resp = client
        .get(format!("{base}/internal/processes/{process_id}"))
        .header("X-Internal-Token", TEST_INTERNAL_TOKEN)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let got: Value = resp.json().await.unwrap();
    assert_eq!(got["name"], "Updated Process");

    // Update process via internal (e.g. next_run_at)
    let resp = client
        .put(format!("{base}/internal/processes/{process_id}"))
        .header("X-Internal-Token", TEST_INTERNAL_TOKEN)
        .json(&json!({"schedule": "0 */10 * * * *"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let got: Value = resp.json().await.unwrap();
    assert_eq!(got["schedule"], "0 */10 * * * *");

    // Create node via public
    let resp = client
        .post(format!("{base}/api/processes/{process_id}/nodes"))
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&json!({
            "nodeType": "ignition",
            "label": "Start",
            "positionX": 100.0,
            "positionY": 50.0
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let node: Value = resp.json().await.unwrap();
    let node_id = node["id"].as_str().unwrap();
    assert_eq!(node["nodeType"], "ignition");

    // Create second node
    let resp = client
        .post(format!("{base}/api/processes/{process_id}/nodes"))
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&json!({
            "nodeType": "action",
            "label": "Do Something",
            "prompt": "Run the task"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let node2: Value = resp.json().await.unwrap();
    let node2_id = node2["id"].as_str().unwrap();

    // List nodes via public
    let resp = client
        .get(format!("{base}/api/processes/{process_id}/nodes"))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let nodes: Vec<Value> = resp.json().await.unwrap();
    assert_eq!(nodes.len(), 2);

    // Update node via public
    let resp = client
        .put(format!(
            "{base}/api/processes/{process_id}/nodes/{node2_id}"
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&json!({"label": "Renamed Action"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let updated_node: Value = resp.json().await.unwrap();
    assert_eq!(updated_node["label"], "Renamed Action");

    // List nodes via internal
    let resp = client
        .get(format!("{base}/internal/processes/{process_id}/nodes"))
        .header("X-Internal-Token", TEST_INTERNAL_TOKEN)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let nodes: Vec<Value> = resp.json().await.unwrap();
    assert_eq!(nodes.len(), 2);

    // Create connection
    let resp = client
        .post(format!("{base}/api/processes/{process_id}/connections"))
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&json!({
            "sourceNodeId": node_id,
            "targetNodeId": node2_id
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // List connections via public
    let resp = client
        .get(format!("{base}/api/processes/{process_id}/connections"))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let conns: Vec<Value> = resp.json().await.unwrap();
    assert_eq!(conns.len(), 1);
    let conn_id = conns[0]["id"].as_str().unwrap();

    // List connections via internal
    let resp = client
        .get(format!(
            "{base}/internal/processes/{process_id}/connections"
        ))
        .header("X-Internal-Token", TEST_INTERNAL_TOKEN)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let conns: Vec<Value> = resp.json().await.unwrap();
    assert_eq!(conns.len(), 1);

    // Delete connection via public
    let resp = client
        .delete(format!(
            "{base}/api/processes/{process_id}/connections/{conn_id}"
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    // Recreate connection for rest of lifecycle
    let resp = client
        .post(format!("{base}/api/processes/{process_id}/connections"))
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&json!({
            "sourceNodeId": node_id,
            "targetNodeId": node2_id
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Delete node via public (delete node2, recreate for rest of lifecycle)
    let resp = client
        .delete(format!(
            "{base}/api/processes/{process_id}/nodes/{node2_id}"
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    // Recreate second node for rest of lifecycle
    let resp = client
        .post(format!("{base}/api/processes/{process_id}/nodes"))
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&json!({
            "nodeType": "action",
            "label": "Do Something",
            "prompt": "Run the task"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Create run via public JWT route
    let resp = client
        .post(format!("{base}/api/processes/{process_id}/runs"))
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&json!({
            "trigger": "manual"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let run: Value = resp.json().await.unwrap();
    let run_id = run["id"].as_str().unwrap();
    assert_eq!(run["status"], "pending");

    // Update run status via public JWT route
    let resp = client
        .put(format!("{base}/api/processes/{process_id}/runs/{run_id}"))
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&json!({"status": "running"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let updated_run: Value = resp.json().await.unwrap();
    assert_eq!(updated_run["status"], "running");

    // List runs via public
    let resp = client
        .get(format!("{base}/api/processes/{process_id}/runs"))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let runs: Vec<Value> = resp.json().await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["status"], "running");

    // Get run via public
    let resp = client
        .get(format!("{base}/api/processes/{process_id}/runs/{run_id}"))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let single_run: Value = resp.json().await.unwrap();
    assert_eq!(single_run["id"], run_id);

    // Create event via public JWT route
    let resp = client
        .post(format!(
            "{base}/api/processes/{process_id}/runs/{run_id}/events"
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&json!({
            "nodeId": node_id,
            "status": "running"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let event: Value = resp.json().await.unwrap();
    let event_id = event["id"].as_str().unwrap();

    // Update event via public JWT route
    let resp = client
        .put(format!(
            "{base}/api/processes/{process_id}/runs/{run_id}/events/{event_id}"
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&json!({
            "status": "completed",
            "output": "Task done",
            "inputTokens": 100,
            "outputTokens": 50
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // List events
    let resp = client
        .get(format!(
            "{base}/api/processes/{process_id}/runs/{run_id}/events"
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let events: Vec<Value> = resp.json().await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["status"], "completed");

    // Create artifact via public JWT route
    let resp = client
        .post(format!(
            "{base}/api/processes/{process_id}/runs/{run_id}/artifacts"
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&json!({
            "nodeId": node_id,
            "artifactType": "report",
            "name": "output.md",
            "filePath": "process-workspaces/test/output.md",
            "sizeBytes": 1024
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // List run artifacts
    let resp = client
        .get(format!(
            "{base}/api/processes/{process_id}/runs/{run_id}/artifacts"
        ))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let artifacts: Vec<Value> = resp.json().await.unwrap();
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0]["artifactType"], "report");
    let artifact_id = artifacts[0]["id"].as_str().unwrap();

    // Get artifact metadata via public
    let resp = client
        .get(format!("{base}/api/process-artifacts/{artifact_id}"))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let artifact: Value = resp.json().await.unwrap();
    assert_eq!(artifact["name"], "output.md");

    // Delete process (CASCADE should clean up everything)
    let resp = client
        .delete(format!("{base}/api/processes/{process_id}"))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    // Verify deleted
    let resp = client
        .get(format!("{base}/internal/processes/{process_id}"))
        .header("X-Internal-Token", TEST_INTERNAL_TOKEN)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn cross_org_isolation() {
    let (addr, _pool) = spawn_authed_test_server(true).await;
    let client = Client::new();
    let base = format!("http://{addr}");
    let jwt = generate_test_jwt();

    let org_a = Uuid::new_v4();
    let org_b = Uuid::new_v4();

    // Create process in org A
    let resp = client
        .post(format!("{base}/api/processes"))
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&json!({"orgId": org_a, "name": "Org A Process"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // List org B — should be empty
    let resp = client
        .get(format!("{base}/api/processes?orgId={org_b}"))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .unwrap();
    let list: Vec<Value> = resp.json().await.unwrap();
    assert!(list.is_empty(), "Org B should see no processes");
}

#[tokio::test]
async fn folder_lifecycle() {
    let (addr, _pool) = spawn_authed_test_server(true).await;
    let client = Client::new();
    let base = format!("http://{addr}");
    let jwt = generate_test_jwt();
    let org_id = Uuid::new_v4();

    // Create folder
    let resp = client
        .post(format!("{base}/api/process-folders"))
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&json!({"orgId": org_id, "name": "My Folder"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let folder: Value = resp.json().await.unwrap();
    let folder_id = folder["id"].as_str().unwrap();

    // List folders
    let resp = client
        .get(format!("{base}/api/process-folders?orgId={org_id}"))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .unwrap();
    let folders: Vec<Value> = resp.json().await.unwrap();
    assert_eq!(folders.len(), 1);

    // Update folder
    let resp = client
        .put(format!("{base}/api/process-folders/{folder_id}"))
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&json!({"name": "Renamed Folder"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let updated: Value = resp.json().await.unwrap();
    assert_eq!(updated["name"], "Renamed Folder");

    // Delete folder
    let resp = client
        .delete(format!("{base}/api/process-folders/{folder_id}"))
        .header("Authorization", format!("Bearer {jwt}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);
}

#[tokio::test]
async fn public_process_writes_are_rejected_without_org_membership() {
    let jwt = generate_test_jwt();
    let (addr, pool) = spawn_authed_test_server(false).await;
    let client = Client::new();
    let base = format!("http://{addr}");
    let org_id = Uuid::new_v4();
    let project_id = Uuid::new_v4();

    let resp = client
        .post(format!("{base}/api/processes"))
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&json!({"orgId": org_id, "name": "Denied Process"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);

    let resp = client
        .post(format!("{base}/api/process-folders"))
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&json!({"orgId": org_id, "name": "Denied Folder"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);

    let process = process_repo::create_process(
        &pool,
        Uuid::new_v4(),
        &process_models::CreateProcessRequest {
            org_id,
            project_id: Some(project_id),
            folder_id: None,
            name: "Direct Insert".into(),
            description: Some("Direct DB row".into()),
            enabled: Some(true),
            schedule: None,
            tags: None,
        },
    )
    .await
    .unwrap();

    let resp = client
        .post(format!("{base}/api/processes/{}/runs", process.id))
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&json!({"trigger": "manual"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn scheduled_processes_internal() {
    let (addr, _pool) = spawn_authed_test_server(true).await;
    let client = Client::new();
    let base = format!("http://{addr}");
    let jwt = generate_test_jwt();
    let org_id = Uuid::new_v4();

    // Create enabled + scheduled process
    let resp = client
        .post(format!("{base}/api/processes"))
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&json!({
            "orgId": org_id,
            "name": "Scheduled Process",
            "schedule": "0 */5 * * * *",
            "enabled": true
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Create disabled process with schedule
    client
        .post(format!("{base}/api/processes"))
        .header("Authorization", format!("Bearer {jwt}"))
        .json(&json!({
            "orgId": org_id,
            "name": "Disabled Scheduled",
            "schedule": "0 0 * * * *",
            "enabled": false
        }))
        .send()
        .await
        .unwrap();

    // List scheduled — only enabled ones
    let resp = client
        .get(format!("{base}/internal/processes/scheduled"))
        .header("X-Internal-Token", TEST_INTERNAL_TOKEN)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let scheduled: Vec<Value> = resp.json().await.unwrap();
    // Should include our enabled+scheduled one (might include others from parallel tests)
    let has_ours = scheduled.iter().any(|p| p["name"] == "Scheduled Process");
    assert!(has_ours, "Should find our scheduled process");
    let has_disabled = scheduled.iter().any(|p| p["name"] == "Disabled Scheduled");
    assert!(!has_disabled, "Should NOT find disabled process");
}

#[tokio::test]
async fn nonexistent_returns_404() {
    let (addr, _pool) = spawn_test_server().await;
    let client = Client::new();
    let base = format!("http://{addr}");
    let fake_id = Uuid::new_v4();

    let resp = client
        .get(format!("{base}/internal/processes/{fake_id}"))
        .header("X-Internal-Token", TEST_INTERNAL_TOKEN)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}
