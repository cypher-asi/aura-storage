pub mod events;
pub mod internal;
pub mod logs;
pub mod project_agents;
pub mod sessions;
pub mod specs;
pub mod stats;
pub mod tasks;
pub mod ws;

use axum::Json;

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
