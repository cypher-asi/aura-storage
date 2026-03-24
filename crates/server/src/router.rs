use axum::routing::{get, post};
use axum::Router;

use crate::handlers;
use crate::state::AppState;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health))
        // Project Agents
        .route(
            "/api/projects/:projectId/agents",
            post(handlers::project_agents::create_project_agent)
                .get(handlers::project_agents::list_project_agents),
        )
        .route(
            "/api/project-agents/:id",
            get(handlers::project_agents::get_project_agent)
                .put(handlers::project_agents::update_project_agent)
                .delete(handlers::project_agents::delete_project_agent),
        )
        // Specs
        .route(
            "/api/projects/:projectId/specs",
            post(handlers::specs::create_spec).get(handlers::specs::list_specs),
        )
        .route(
            "/api/specs/:id",
            get(handlers::specs::get_spec)
                .put(handlers::specs::update_spec)
                .delete(handlers::specs::delete_spec),
        )
        // Tasks
        .route(
            "/api/projects/:projectId/tasks",
            post(handlers::tasks::create_task).get(handlers::tasks::list_tasks),
        )
        .route(
            "/api/tasks/:id",
            get(handlers::tasks::get_task)
                .put(handlers::tasks::update_task)
                .delete(handlers::tasks::delete_task),
        )
        .route(
            "/api/tasks/:id/transition",
            post(handlers::tasks::transition_task),
        )
        // Sessions
        .route(
            "/api/project-agents/:projectAgentId/sessions",
            post(handlers::sessions::create_session).get(handlers::sessions::list_sessions),
        )
        .route(
            "/api/sessions/:id",
            get(handlers::sessions::get_session).put(handlers::sessions::update_session),
        )
        // Events
        .route(
            "/api/sessions/:sessionId/events",
            post(handlers::events::create_event).get(handlers::events::list_events),
        )
        // Stats
        .route("/api/stats", get(handlers::stats::get_stats))
        // Log Entries
        .route(
            "/api/projects/:projectId/logs",
            post(handlers::logs::create_log_entry).get(handlers::logs::list_log_entries),
        )
        // Internal (X-Internal-Token auth) — full CRUD parity with /api/
        // Sessions
        .route(
            "/internal/sessions",
            post(handlers::internal::create_session),
        )
        .route(
            "/internal/sessions/:id",
            get(handlers::internal::get_session).put(handlers::internal::update_session),
        )
        .route(
            "/internal/project-agents/:projectAgentId/sessions",
            get(handlers::internal::list_sessions),
        )
        // Events
        .route("/internal/events", post(handlers::internal::create_event))
        .route(
            "/internal/sessions/:sessionId/events",
            get(handlers::internal::list_events),
        )
        // Logs
        .route("/internal/logs", post(handlers::internal::create_log))
        .route(
            "/internal/projects/:projectId/logs",
            get(handlers::internal::list_logs),
        )
        // Project Agents
        .route(
            "/internal/projects/:projectId/agents",
            post(handlers::internal::create_project_agent)
                .get(handlers::internal::list_project_agents),
        )
        .route(
            "/internal/project-agents/:id",
            get(handlers::internal::get_project_agent)
                .delete(handlers::internal::delete_project_agent),
        )
        .route(
            "/internal/project-agents/:id/status",
            post(handlers::internal::update_agent_status),
        )
        .route(
            "/internal/projects/:projectId/agents/count",
            get(handlers::internal::get_project_agent_count),
        )
        // Specs
        .route(
            "/internal/specs",
            post(handlers::internal::create_spec),
        )
        .route(
            "/internal/projects/:projectId/specs",
            get(handlers::internal::list_specs),
        )
        .route(
            "/internal/specs/:id",
            get(handlers::internal::get_spec)
                .put(handlers::internal::update_spec)
                .delete(handlers::internal::delete_spec),
        )
        // Tasks
        .route(
            "/internal/tasks",
            post(handlers::internal::create_task),
        )
        .route(
            "/internal/projects/:projectId/tasks",
            get(handlers::internal::list_tasks),
        )
        .route(
            "/internal/tasks/:id",
            get(handlers::internal::get_task)
                .put(handlers::internal::update_task)
                .delete(handlers::internal::delete_task),
        )
        .route(
            "/internal/tasks/:id/transition",
            post(handlers::internal::transition_task),
        )
        // Stats
        .route("/internal/stats", get(handlers::internal::get_stats))
        // WebSocket
        .route("/ws/events", get(handlers::ws::ws_events))
}
