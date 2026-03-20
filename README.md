<h1 align="center">aura-storage</h1>

<p align="center">
  <b>The execution data layer for autonomous agent workflows.</b>
</p>

## Overview

aura-storage stores all project execution data for the AURA platform — specs, tasks, sessions, messages, project agents, and log entries. All AURA clients (desktop, web, mobile) and aura-swarm (cloud agent orchestration) connect to this service for execution state.

Projects themselves live in [aura-network](https://github.com/cypher-asi/aura-network) (the social layer). This service references project UUIDs from there. Together: aura-network owns "what exists", aura-storage owns "what happened".

---

## Quick Start

### Prerequisites

- Rust 1.85+
- PostgreSQL 15+

### Setup

```
cp .env.example .env
# Edit .env with your database URL and auth config

cargo run -p aura-storage-server
```

The server starts on `http://0.0.0.0:3000` by default. Migrations run automatically on startup.

### Health Check

```
curl http://localhost:3000/health
```

Returns:
```json
{"status": "ok", "timestamp": "2026-03-18T22:00:00Z"}
```

### Environment Variables

| Variable | Required | Description |
|---|---|---|
| `DATABASE_URL` | Yes | PostgreSQL connection string |
| `AUTH0_DOMAIN` | Yes | Auth0 domain for JWKS |
| `AUTH0_AUDIENCE` | Yes | Auth0 audience identifier |
| `AUTH_COOKIE_SECRET` | Yes | Shared secret for HS256 token validation (same as aura-network) |
| `INTERNAL_SERVICE_TOKEN` | Yes | Token for service-to-service auth (aura-swarm -> aura-storage) |
| `PORT` | No | Server port (default: 3000) |
| `CORS_ORIGINS` | No | Comma-separated allowed origins. Omit for permissive (dev mode) |
| `RUST_LOG` | No | Tracing filter (default: `aura_storage=debug,tower_http=debug,info`) |

---

## Authentication

All API endpoints require a JWT in the `Authorization: Bearer <token>` header. Tokens are obtained by logging in via zOS API. Same tokens as aura-network — both RS256 (Auth0 JWKS) and HS256 (shared secret) are accepted.

Internal (service-to-service) endpoints use `X-Internal-Token` header instead.

Unlike aura-network, this service does **not** auto-create users. The `created_by` field stores the user's UUID from the JWT but does not validate it against a users table — that lives in aura-network.

---

## Types

### ProjectAgentStatus

`idle` | `working` | `blocked` | `stopped` | `error`

### TaskStatus

`pending` | `ready` | `in_progress` | `done` | `failed` | `blocked`

### SessionStatus

`active` | `completed` | `failed` | `rolled_over`

### MessageRole

`user` | `assistant` | `system`

### LogLevel

`info` | `warn` | `error` | `debug`

---

## API Reference

### Project Agents

Project agents bridge agents (from aura-network) to projects. Each represents an agent assigned to work on a specific project.

| Method | Path | Description | Auth |
|---|---|---|---|
| POST | `/api/projects/:projectId/agents` | Assign agent to project. Body: `{"agentId": "...", "model": "..."}` | JWT |
| GET | `/api/projects/:projectId/agents` | List agents for project | JWT |
| GET | `/api/project-agents/:id` | Get project agent | JWT |
| PUT | `/api/project-agents/:id` | Update status. Body: `{"status": "working"}` | JWT |
| DELETE | `/api/project-agents/:id` | Remove agent from project | JWT |

### Specs

Specs define the requirements for a project. Ordered by `orderIndex`.

| Method | Path | Description | Auth |
|---|---|---|---|
| POST | `/api/projects/:projectId/specs` | Create spec. Body: `{"title": "...", "orderIndex": 0, "markdownContents": "..."}` | JWT |
| GET | `/api/projects/:projectId/specs` | List specs (ordered by orderIndex) | JWT |
| GET | `/api/specs/:id` | Get spec | JWT |
| PUT | `/api/specs/:id` | Update spec (partial — only send fields to change) | JWT |
| DELETE | `/api/specs/:id` | Delete spec | JWT |

### Tasks

Tasks are units of work derived from specs. Each task has a status governed by a state machine.

| Method | Path | Description | Auth |
|---|---|---|---|
| POST | `/api/projects/:projectId/tasks` | Create task. Body: `{"specId": "...", "title": "...", "orderIndex": 0, ...}` | JWT |
| GET | `/api/projects/:projectId/tasks` | List tasks. Filter: `?status=pending` | JWT |
| GET | `/api/tasks/:id` | Get task | JWT |
| PUT | `/api/tasks/:id` | Update task (partial — title, description, executionNotes, filesChanged) | JWT |
| DELETE | `/api/tasks/:id` | Delete task | JWT |
| POST | `/api/tasks/:id/transition` | Transition status. Body: `{"status": "ready"}` | JWT |

#### Task Status Transitions

```
pending -> ready
ready -> in_progress
in_progress -> done | failed | blocked
failed -> ready       (retry)
blocked -> ready      (unblock)
```

Invalid transitions return 400 Bad Request.

#### Create Task Body

```json
{
  "specId": "uuid",
  "title": "Implement login",
  "description": "Optional description",
  "orderIndex": 0,
  "dependencyTaskIds": ["uuid", "uuid"],
  "parentTaskId": "uuid",
  "assignedProjectAgentId": "uuid"
}
```

#### Update Task Body (partial)

```json
{
  "executionNotes": "Completed successfully",
  "filesChanged": [
    {"op": "add", "path": "src/auth.rs", "linesAdded": 45, "linesRemoved": 0}
  ]
}
```

### Sessions

Sessions represent a continuous agent execution context. Created per project agent.

| Method | Path | Description | Auth |
|---|---|---|---|
| POST | `/api/project-agents/:projectAgentId/sessions` | Start session. Body: `{"projectId": "...", "model": "..."}` | JWT |
| GET | `/api/project-agents/:projectAgentId/sessions` | List sessions for agent | JWT |
| GET | `/api/sessions/:id` | Get session | JWT |
| PUT | `/api/sessions/:id` | Update session (partial — status, tokens, contextUsage, summary, endedAt) | JWT |

#### Update Session Body (partial)

```json
{
  "status": "completed",
  "totalInputTokens": 15000,
  "totalOutputTokens": 3000,
  "contextUsage": 0.45,
  "summary": "Implemented auth endpoints",
  "endedAt": "2026-03-18T22:30:00Z"
}
```

### Messages

Messages are the LLM conversation history within a session.

| Method | Path | Description | Auth |
|---|---|---|---|
| POST | `/api/sessions/:sessionId/messages` | Create message | JWT |
| GET | `/api/sessions/:sessionId/messages` | List messages. Params: `?limit=&offset=` | JWT |

#### Create Message Body

```json
{
  "projectAgentId": "uuid",
  "projectId": "uuid",
  "createdBy": "uuid or null",
  "role": "user | assistant | system",
  "content": "Message text",
  "contentBlocks": [{"type": "text", "text": "..."}],
  "inputTokens": 500,
  "outputTokens": 200,
  "thinking": "Let me think about this...",
  "thinkingDurationMs": 1500
}
```

`createdBy` is nullable — omit for system messages. `role` determines the message type (user prompt, assistant response, or system context). `thinking` and `thinkingDurationMs` store Claude's extended thinking content and duration.

### Log Entries

Structured logs for project agent activity.

| Method | Path | Description | Auth |
|---|---|---|---|
| POST | `/api/projects/:projectId/logs` | Create log entry | JWT |
| GET | `/api/projects/:projectId/logs` | List logs. Params: `?level=error&limit=&offset=` | JWT |

#### Create Log Entry Body

```json
{
  "projectAgentId": "uuid or null",
  "createdBy": "uuid or null",
  "level": "info | warn | error | debug",
  "message": "Log message text",
  "metadata": {"key": "value"}
}
```

### Stats

Aggregated execution stats at project, org, or network level.

| Method | Path | Description | Auth |
|---|---|---|---|
| GET | `/api/stats?scope=project&projectId=...` | Project-level stats | JWT |
| GET | `/api/stats?scope=org&orgId=...` | Org-level stats | JWT |
| GET | `/api/stats?scope=network` | Network-wide stats | JWT |

Returns:
```json
{
  "totalTasks": 25,
  "pendingTasks": 3,
  "readyTasks": 2,
  "inProgressTasks": 5,
  "blockedTasks": 1,
  "doneTasks": 12,
  "failedTasks": 2,
  "completionPercentage": 48.0,
  "totalTokens": 150000,
  "totalMessages": 340,
  "totalAgents": 3,
  "totalSessions": 8,
  "totalTimeSeconds": 3600,
  "linesChanged": 450,
  "totalSpecs": 4
}
```

Same response shape at all scope levels. Token cost, commits, and PRs come from aura-network and orbit respectively.

### Internal Endpoints

Authenticated via `X-Internal-Token` header. Called by aura-swarm and other backend services.

| Method | Path | Description |
|---|---|---|
| POST | `/internal/sessions` | Create session (with createdBy in body) |
| POST | `/internal/messages` | Write message |
| POST | `/internal/logs` | Write log entry |
| POST | `/internal/project-agents/:id/status` | Update agent status |

Internal endpoints include fields that the public endpoints derive from path params and auth:

**Internal create session:**
```json
{
  "projectAgentId": "uuid",
  "projectId": "uuid",
  "createdBy": "uuid",
  "model": "claude-opus-4-6"
}
```

**Internal create message:**
```json
{
  "sessionId": "uuid",
  "projectAgentId": "uuid",
  "projectId": "uuid",
  "role": "assistant",
  "content": "Message text",
  "inputTokens": 500,
  "outputTokens": 200,
  "thinking": "Extended thinking content",
  "thinkingDurationMs": 1500
}
```

**Internal create log:**
```json
{
  "projectId": "uuid",
  "projectAgentId": "uuid",
  "level": "info",
  "message": "Log message",
  "metadata": {"key": "value"}
}
```

### Real-Time

| Protocol | Path | Description | Auth |
|---|---|---|---|
| WebSocket | `/ws/events` | Real-time event stream | JWT (query param `?token=`) |

Events broadcast when:
- Project agent status changes (`project_agent.status_changed`)
- Task status transitions (`task.status_changed`)
- Session starts (`session.started`) or status changes (`session.status_changed`)

Ping/pong keepalive every 30 seconds.

---

## Request/Response Format

All request and response bodies use JSON with **camelCase** field names.

**Successful responses:** 200 with JSON body, or 204 No Content for DELETE operations.

**Error responses:**
```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "Not found: Task not found"
  }
}
```

Error codes: `NOT_FOUND` (404), `UNAUTHORIZED` (401), `FORBIDDEN` (403), `BAD_REQUEST` (400), `CONFLICT` (409), `INTERNAL` (500).

**Pagination:** Messages and log entries support `?limit=` (default 50, max 100) and `?offset=` (default 0). Other list endpoints return all results.

---

## Cross-Service References

This service stores UUIDs that reference entities in aura-network. These are **not** foreign key constrained (different databases).

| Field | References |
|---|---|
| `project_id` | Project in aura-network |
| `agent_id` | Agent in aura-network |
| `created_by` | User in aura-network |

---

## Entity Hierarchy

```
project (aura-network)
  +-- project_agent -> agent (aura-network)
  |     +-- session
  |     |     +-- messages
  |     +-- log_entries
  +-- specs
  +-- tasks -> spec, project_agent
```

---

## Integration Guide

### From aura-code (Desktop)

```
Auth:       zOS API (login) -> gets JWT
Network:    aura-network (profiles, orgs, agents, feed, projects)
Storage:    aura-storage (specs, tasks, sessions, messages, project agents, logs)
Billing:    zero-payments-server (credit balance, debit via JWT)
Local:      RocksDB (terminal, filesystem, settings)
```

On app load: fetch projects from aura-network, then fetch execution data from aura-storage for active projects.

### From aura-swarm (Cloud Agents)

```
1. Update agent status:    POST /internal/project-agents/:id/status
2. Create session:         POST /internal/sessions
3. Write messages:         POST /internal/messages (per LLM call)
4. Write logs:             POST /internal/logs
5. Post to feed:            POST aura-network /internal/posts
```

Use `X-Internal-Token` for aura-storage internal endpoints. Use the user's JWT for aura-network activity posts and credit debits.

### From Mobile

Same API as desktop — all endpoints are API-first. Authenticate via zOS, then call aura-storage directly.

---

## Architecture

| Crate | Description |
|---|---|
| **aura-storage-core** | Shared types, error handling, pagination |
| **aura-storage-db** | PostgreSQL connection pool and migrations (9 migrations) |
| **aura-storage-auth** | JWT validation (Auth0 JWKS + HS256) and auth extractors |
| **aura-storage-server** | Axum HTTP server, router, handlers, WebSocket |
| **aura-storage-project-agents** | Project agent assignment and status tracking |
| **aura-storage-specs** | Spec management (requirements documents) |
| **aura-storage-tasks** | Task management with status state machine |
| **aura-storage-sessions** | Agent execution sessions |
| **aura-storage-messages** | LLM conversation history |
| **aura-storage-logs** | Structured log entries |

## License

MIT
