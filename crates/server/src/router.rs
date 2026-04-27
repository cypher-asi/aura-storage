use axum::routing::{delete, get, post, put};
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
        // Artifacts
        .route(
            "/api/projects/:projectId/artifacts",
            post(handlers::artifacts::create_artifact).get(handlers::artifacts::list_artifacts),
        )
        .route(
            "/api/artifacts/:id",
            get(handlers::artifacts::get_artifact).delete(handlers::artifacts::delete_artifact),
        )
        .route(
            "/api/artifacts/:id/children",
            get(handlers::artifacts::get_artifact_children),
        )
        // Processes
        .route(
            "/api/processes",
            post(handlers::processes::create_process).get(handlers::processes::list_processes),
        )
        .route(
            "/api/processes/:id",
            get(handlers::processes::get_process)
                .put(handlers::processes::update_process)
                .delete(handlers::processes::delete_process),
        )
        .route(
            "/api/processes/:id/nodes",
            post(handlers::processes::create_node).get(handlers::processes::list_nodes),
        )
        .route(
            "/api/processes/:id/nodes/:nodeId",
            put(handlers::processes::update_node).delete(handlers::processes::delete_node),
        )
        .route(
            "/api/processes/:id/connections",
            post(handlers::processes::create_connection).get(handlers::processes::list_connections),
        )
        .route(
            "/api/processes/:id/connections/:connectionId",
            delete(handlers::processes::delete_connection),
        )
        .route(
            "/api/processes/:id/runs",
            post(handlers::processes::create_run).get(handlers::processes::list_runs),
        )
        .route(
            "/api/processes/:id/runs/:runId",
            get(handlers::processes::get_run).put(handlers::processes::update_run),
        )
        .route(
            "/api/processes/:id/runs/:runId/events",
            post(handlers::processes::create_run_event).get(handlers::processes::list_run_events),
        )
        .route(
            "/api/processes/:id/runs/:runId/events/:eventId",
            put(handlers::processes::update_run_event),
        )
        .route(
            "/api/processes/:id/runs/:runId/artifacts",
            post(handlers::processes::create_run_artifact)
                .get(handlers::processes::list_run_artifacts),
        )
        .route(
            "/api/process-artifacts/:id",
            get(handlers::processes::get_artifact),
        )
        .route(
            "/api/process-folders",
            post(handlers::processes::create_folder).get(handlers::processes::list_folders),
        )
        .route(
            "/api/process-folders/:id",
            put(handlers::processes::update_folder).delete(handlers::processes::delete_folder),
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
            "/internal/sessions/:id/tokens",
            post(handlers::internal::increment_session_tokens),
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
        // Project cascade delete
        .route(
            "/internal/projects/:projectId",
            delete(handlers::internal::delete_project_data),
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
        .route("/internal/specs", post(handlers::internal::create_spec))
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
        .route("/internal/tasks", post(handlers::internal::create_task))
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
        // Artifacts
        .route(
            "/internal/artifacts",
            post(handlers::internal::create_artifact),
        )
        .route(
            "/internal/projects/:projectId/artifacts",
            get(handlers::internal::list_artifacts),
        )
        .route(
            "/internal/artifacts/:id",
            get(handlers::internal::get_artifact).delete(handlers::internal::delete_artifact),
        )
        // Processes
        .route(
            "/internal/processes/scheduled",
            get(handlers::internal::list_scheduled_processes),
        )
        .route(
            "/internal/processes/:id",
            get(handlers::internal::get_process).put(handlers::internal::update_process),
        )
        .route(
            "/internal/processes/:id/nodes",
            get(handlers::internal::list_process_nodes),
        )
        .route(
            "/internal/processes/:id/connections",
            get(handlers::internal::list_process_connections),
        )
        .route(
            "/internal/process-runs",
            post(handlers::internal::create_process_run),
        )
        .route(
            "/internal/process-runs/:id",
            put(handlers::internal::update_process_run),
        )
        .route(
            "/internal/process-events",
            post(handlers::internal::create_process_event),
        )
        .route(
            "/internal/process-events/:id",
            put(handlers::internal::update_process_event),
        )
        .route(
            "/internal/process-artifacts",
            post(handlers::internal::create_process_artifact),
        )
        // Stats
        .route("/internal/stats", get(handlers::internal::get_stats))
        // WebSocket
        .route("/ws/events", get(handlers::ws::ws_events))
}
