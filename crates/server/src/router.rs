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
        // Specs
        .route("/api/projects/:projectId/specs", post(handlers::specs::create_spec).get(handlers::specs::list_specs))
        .route("/api/specs/:id", get(handlers::specs::get_spec).put(handlers::specs::update_spec).delete(handlers::specs::delete_spec))
        // Tasks
        .route("/api/projects/:projectId/tasks", post(handlers::tasks::create_task).get(handlers::tasks::list_tasks))
        .route("/api/tasks/:id", get(handlers::tasks::get_task).put(handlers::tasks::update_task).delete(handlers::tasks::delete_task))
        .route("/api/tasks/:id/transition", post(handlers::tasks::transition_task))
}
