pub mod project_agents;
pub mod sessions;
pub mod specs;
pub mod tasks;

use axum::Json;

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
