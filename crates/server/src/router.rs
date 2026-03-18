use axum::routing::{get, post};
use axum::Router;

use crate::handlers;
use crate::state::AppState;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health))
        // Project Agents
        .route("/api/projects/:projectId/agents", post(handlers::project_agents::create_project_agent).get(handlers::project_agents::list_project_agents))
        .route("/api/project-agents/:id", get(handlers::project_agents::get_project_agent).put(handlers::project_agents::update_project_agent).delete(handlers::project_agents::delete_project_agent))
}
