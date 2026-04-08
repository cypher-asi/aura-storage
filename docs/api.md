# Aura Storage API Reference

## Overview

Aura Storage provides persistent storage for project agents, specs, tasks, sessions, events, logs, artifacts, processes, and real-time WebSocket notifications. All request and response bodies use **camelCase** JSON.

---

## Authentication

### Public Endpoints

All `/api/*` endpoints require a JSON Web Token:

```
Authorization: Bearer <token>
```

Both **RS256** (Auth0 JWKS) and **HS256** (shared secret) tokens are accepted.

### Internal Endpoints

All `/internal/*` endpoints require a shared secret:

```
X-Internal-Token: <secret>
```

---

## Response Format

**Success:** `200` with a JSON body, or `204 No Content` for DELETE operations.

**Errors:** All errors return a JSON body:

```json
{
  "error": {
    "code": "NOT_FOUND | UNAUTHORIZED | FORBIDDEN | BAD_REQUEST | CONFLICT | INTERNAL",
    "message": "Human-readable description"
  }
}
```

**Pagination:** Endpoints that support pagination accept `?limit=` and `?offset=` query parameters.

---

## Project Agents

### POST /api/projects/:projectId/agents

**Auth:** JWT

Create a new project agent.

**Path Parameters:**

| Parameter   | Type | Required | Description        |
|-------------|------|----------|--------------------|
| `projectId` | UUID | Yes      | The project's UUID |

**Request Body:**

| Field     | Type   | Required | Description                |
|-----------|--------|----------|----------------------------|
| `agentId` | UUID   | Yes      | The agent to attach         |
| `orgId`   | UUID   | No       | Organization scope          |
| `model`   | string | No       | LLM model identifier        |

```json
{
  "agentId": "uuid",
  "orgId": "uuid",
  "model": "string"
}
```

**Response:** `200` — ProjectAgent

```json
{
  "id": "uuid",
  "projectId": "uuid",
  "orgId": "uuid | null",
  "agentId": "uuid",
  "createdBy": "uuid",
  "status": "idle | working | blocked | stopped | error",
  "model": "string | null",
  "totalInputTokens": 0,
  "totalOutputTokens": 0,
  "createdAt": "datetime",
  "updatedAt": "datetime"
}
```

---

### GET /api/projects/:projectId/agents

**Auth:** JWT

List all agents for a project.

**Path Parameters:**

| Parameter   | Type | Required | Description        |
|-------------|------|----------|--------------------|
| `projectId` | UUID | Yes      | The project's UUID |

**Response:** `200` — Array of ProjectAgent objects.

---

### GET /api/project-agents/:id

**Auth:** JWT

Get a single project agent by ID.

**Path Parameters:**

| Parameter | Type | Required | Description              |
|-----------|------|----------|--------------------------|
| `id`      | UUID | Yes      | The project agent's UUID |

**Response:** `200` — ProjectAgent

---

### PUT /api/project-agents/:id

**Auth:** JWT

Update a project agent's status.

**Path Parameters:**

| Parameter | Type | Required | Description              |
|-----------|------|----------|--------------------------|
| `id`      | UUID | Yes      | The project agent's UUID |

**Request Body:**

| Field    | Type   | Required | Description                                   |
|----------|--------|----------|-----------------------------------------------|
| `status` | string | Yes      | One of: `idle`, `working`, `blocked`, `stopped`, `error` |

```json
{
  "status": "working"
}
```

**Response:** `200` — ProjectAgent

**Side Effect:** Broadcasts `project_agent.status_changed` via WebSocket.

---

### DELETE /api/project-agents/:id

**Auth:** JWT

Delete a project agent.

**Path Parameters:**

| Parameter | Type | Required | Description              |
|-----------|------|----------|--------------------------|
| `id`      | UUID | Yes      | The project agent's UUID |

**Response:** `204 No Content`

---

## Specs

### POST /api/projects/:projectId/specs

**Auth:** JWT

Create a new spec.

**Path Parameters:**

| Parameter   | Type | Required | Description        |
|-------------|------|----------|--------------------|
| `projectId` | UUID | Yes      | The project's UUID |

**Request Body:**

| Field              | Type    | Required | Description                |
|--------------------|---------|----------|----------------------------|
| `orgId`            | UUID    | No       | Organization scope          |
| `title`            | string  | Yes      | Spec title                  |
| `orderIndex`       | integer | Yes      | Display/sort order          |
| `markdownContents` | string  | Yes      | Full markdown body          |

```json
{
  "orgId": "uuid",
  "title": "string",
  "orderIndex": 0,
  "markdownContents": "string"
}
```

**Response:** `200` — Spec

```json
{
  "id": "uuid",
  "projectId": "uuid",
  "orgId": "uuid | null",
  "createdBy": "uuid",
  "title": "string",
  "orderIndex": 0,
  "markdownContents": "string",
  "createdAt": "datetime",
  "updatedAt": "datetime"
}
```

---

### GET /api/projects/:projectId/specs

**Auth:** JWT

List all specs for a project, ordered by `orderIndex`.

**Path Parameters:**

| Parameter   | Type | Required | Description        |
|-------------|------|----------|--------------------|
| `projectId` | UUID | Yes      | The project's UUID |

**Response:** `200` — Array of Spec objects (ordered by `orderIndex`).

---

### GET /api/specs/:id

**Auth:** JWT

Get a single spec by ID.

**Path Parameters:**

| Parameter | Type | Required | Description    |
|-----------|------|----------|----------------|
| `id`      | UUID | Yes      | The spec's UUID |

**Response:** `200` — Spec

---

### PUT /api/specs/:id

**Auth:** JWT

Update a spec. All fields are optional.

**Path Parameters:**

| Parameter | Type | Required | Description    |
|-----------|------|----------|----------------|
| `id`      | UUID | Yes      | The spec's UUID |

**Request Body:**

| Field              | Type    | Required | Description                |
|--------------------|---------|----------|----------------------------|
| `title`            | string  | No       | New title                   |
| `orderIndex`       | integer | No       | New sort order              |
| `markdownContents` | string  | No       | New markdown body           |

```json
{
  "title": "string",
  "orderIndex": 1,
  "markdownContents": "string"
}
```

**Response:** `200` — Spec

---

### DELETE /api/specs/:id

**Auth:** JWT

Delete a spec.

**Path Parameters:**

| Parameter | Type | Required | Description    |
|-----------|------|----------|----------------|
| `id`      | UUID | Yes      | The spec's UUID |

**Response:** `204 No Content`

---

## Tasks

### POST /api/projects/:projectId/tasks

**Auth:** JWT

Create a new task.

**Path Parameters:**

| Parameter   | Type | Required | Description        |
|-------------|------|----------|--------------------|
| `projectId` | UUID | Yes      | The project's UUID |

**Request Body:**

| Field                    | Type     | Required | Description                          |
|--------------------------|----------|----------|--------------------------------------|
| `orgId`                  | UUID     | No       | Organization scope                    |
| `specId`                 | UUID     | Yes      | Parent spec                           |
| `title`                  | string   | Yes      | Task title                            |
| `description`            | string   | No       | Task description                      |
| `orderIndex`             | integer  | Yes      | Display/sort order                    |
| `dependencyTaskIds`      | UUID[]   | No       | Tasks that must complete first        |
| `parentTaskId`           | UUID     | No       | Parent task for sub-task hierarchy     |
| `assignedProjectAgentId` | UUID     | No       | Agent assigned to this task           |

```json
{
  "orgId": "uuid",
  "specId": "uuid",
  "title": "string",
  "description": "string",
  "orderIndex": 0,
  "dependencyTaskIds": ["uuid"],
  "parentTaskId": "uuid",
  "assignedProjectAgentId": "uuid"
}
```

**Response:** `200` — Task

```json
{
  "id": "uuid",
  "projectId": "uuid",
  "orgId": "uuid | null",
  "specId": "uuid",
  "createdBy": "uuid",
  "title": "string",
  "description": "string | null",
  "status": "pending | ready | in_progress | blocked | done | failed",
  "orderIndex": 0,
  "dependencyTaskIds": ["uuid"],
  "parentTaskId": "uuid | null",
  "assignedProjectAgentId": "uuid | null",
  "sessionId": "uuid | null",
  "executionNotes": "string | null",
  "filesChanged": null,
  "model": "string | null",
  "totalInputTokens": 0,
  "totalOutputTokens": 0,
  "createdAt": "datetime",
  "updatedAt": "datetime"
}
```

---

### GET /api/projects/:projectId/tasks

**Auth:** JWT

List all tasks for a project, ordered by `orderIndex`.

**Path Parameters:**

| Parameter   | Type | Required | Description        |
|-------------|------|----------|--------------------|
| `projectId` | UUID | Yes      | The project's UUID |

**Query Parameters:**

| Parameter | Type   | Required | Description                                                  |
|-----------|--------|----------|--------------------------------------------------------------|
| `status`  | string | No       | Filter by status: `pending`, `ready`, `in_progress`, `blocked`, `done`, `failed` |

**Response:** `200` — Array of Task objects (ordered by `orderIndex`).

---

### GET /api/tasks/:id

**Auth:** JWT

Get a single task by ID.

**Path Parameters:**

| Parameter | Type | Required | Description    |
|-----------|------|----------|----------------|
| `id`      | UUID | Yes      | The task's UUID |

**Response:** `200` — Task

---

### PUT /api/tasks/:id

**Auth:** JWT

Update task fields. All fields are optional. Does **not** change task status (use the transition endpoint instead).

**Path Parameters:**

| Parameter | Type | Required | Description    |
|-----------|------|----------|----------------|
| `id`      | UUID | Yes      | The task's UUID |

**Request Body:**

| Field            | Type   | Required | Description                          |
|------------------|--------|----------|--------------------------------------|
| `title`          | string | No       | New title                            |
| `description`    | string | No       | New description                      |
| `executionNotes` | string | No       | Agent-written execution notes        |
| `filesChanged`   | array  | No       | Array of file operation objects       |

`filesChanged` entry format:

| Field          | Type    | Description                         |
|----------------|---------|-------------------------------------|
| `op`           | string  | Operation: `add`, `modify`, `delete` |
| `path`         | string  | File path relative to project root   |
| `linesAdded`   | integer | Number of lines added                |
| `linesRemoved` | integer | Number of lines removed              |

```json
{
  "title": "string",
  "description": "string",
  "executionNotes": "string",
  "filesChanged": [
    {
      "op": "add",
      "path": "src/auth.rs",
      "linesAdded": 45,
      "linesRemoved": 0
    }
  ]
}
```

**Response:** `200` — Task

---

### DELETE /api/tasks/:id

**Auth:** JWT

Delete a task.

**Path Parameters:**

| Parameter | Type | Required | Description    |
|-----------|------|----------|----------------|
| `id`      | UUID | Yes      | The task's UUID |

**Response:** `204 No Content`

---

### POST /api/tasks/:id/transition

**Auth:** JWT

Transition a task to a new status. Enforces a state machine; invalid transitions return `400`.

**Path Parameters:**

| Parameter | Type | Required | Description    |
|-----------|------|----------|----------------|
| `id`      | UUID | Yes      | The task's UUID |

**Request Body:**

| Field    | Type   | Required | Description    |
|----------|--------|----------|----------------|
| `status` | string | Yes      | Target status  |

```json
{
  "status": "in_progress"
}
```

**Response:** `200` — Task

**Side Effect:** Broadcasts `task.status_changed` via WebSocket.

**Valid Transitions:**

| From          | To                              |
|---------------|---------------------------------|
| `pending`     | `ready`                         |
| `ready`       | `in_progress`                   |
| `in_progress` | `done`, `failed`, `blocked`     |
| `failed`      | `ready` (retry)                 |
| `blocked`     | `ready` (unblock)               |

All other transitions return `400 BAD_REQUEST`.

---

## Sessions

### POST /api/project-agents/:projectAgentId/sessions

**Auth:** JWT

Create a new session for a project agent.

**Path Parameters:**

| Parameter        | Type | Required | Description              |
|------------------|------|----------|--------------------------|
| `projectAgentId` | UUID | Yes      | The project agent's UUID |

**Request Body:**

| Field       | Type   | Required | Description             |
|-------------|--------|----------|-------------------------|
| `projectId` | UUID   | Yes      | The project's UUID       |
| `orgId`     | UUID   | No       | Organization scope       |
| `model`     | string | No       | LLM model identifier     |

```json
{
  "projectId": "uuid",
  "orgId": "uuid",
  "model": "string"
}
```

**Response:** `200` — Session

```json
{
  "id": "uuid",
  "projectAgentId": "uuid",
  "projectId": "uuid",
  "orgId": "uuid | null",
  "createdBy": "uuid",
  "model": "string | null",
  "status": "active | completed | failed | rolled_over",
  "totalInputTokens": 0,
  "totalOutputTokens": 0,
  "contextUsage": 0.0,
  "summary": "string | null",
  "startedAt": "datetime",
  "endedAt": "datetime | null"
}
```

**Side Effect:** Broadcasts `session.started` via WebSocket.

---

### GET /api/project-agents/:projectAgentId/sessions

**Auth:** JWT

List all sessions for a project agent.

**Path Parameters:**

| Parameter        | Type | Required | Description              |
|------------------|------|----------|--------------------------|
| `projectAgentId` | UUID | Yes      | The project agent's UUID |

**Response:** `200` — Array of Session objects.

---

### GET /api/sessions/:id

**Auth:** JWT

Get a single session by ID.

**Path Parameters:**

| Parameter | Type | Required | Description       |
|-----------|------|----------|-------------------|
| `id`      | UUID | Yes      | The session's UUID |

**Response:** `200` — Session

---

### PUT /api/sessions/:id

**Auth:** JWT

Update a session. All fields are optional.

**Path Parameters:**

| Parameter | Type | Required | Description       |
|-----------|------|----------|-------------------|
| `id`      | UUID | Yes      | The session's UUID |

**Request Body:**

| Field              | Type     | Required | Description                                       |
|--------------------|----------|----------|---------------------------------------------------|
| `status`           | string   | No       | One of: `active`, `completed`, `failed`, `rolled_over` |
| `totalInputTokens` | integer  | No       | Cumulative input tokens                            |
| `totalOutputTokens`| integer  | No       | Cumulative output tokens                           |
| `contextUsage`     | float    | No       | Context window utilization (0.0 - 1.0)             |
| `summary`          | string   | No       | Session summary text                               |
| `endedAt`          | datetime | No       | When the session ended                             |

```json
{
  "status": "completed",
  "totalInputTokens": 15000,
  "totalOutputTokens": 3200,
  "contextUsage": 0.45,
  "summary": "Implemented auth module",
  "endedAt": "2026-03-24T12:00:00Z"
}
```

**Response:** `200` — Session

**Side Effect:** If `status` is updated, broadcasts `session.status_changed` via WebSocket.

---

## Session Events

### POST /api/sessions/:sessionId/events

**Auth:** JWT

Create a new event in a session.

**Path Parameters:**

| Parameter   | Type | Required | Description       |
|-------------|------|----------|-------------------|
| `sessionId` | UUID | Yes      | The session's UUID |

**Request Body:**

| Field       | Type   | Required | Description                                       |
|-------------|--------|----------|---------------------------------------------------|
| `sessionId` | UUID   | Yes      | The session's UUID (must match path)               |
| `userId`    | UUID   | No       | User who triggered the event                       |
| `agentId`   | UUID   | No       | Agent that generated the event                     |
| `sender`    | string | No       | `user` or `agent` (validated if present)           |
| `projectId` | UUID   | No       | Associated project                                 |
| `orgId`     | UUID   | No       | Organization scope                                 |
| `type`      | string | Yes      | Event type identifier                              |
| `content`   | object | No       | Freeform JSONB payload                             |

```json
{
  "sessionId": "uuid",
  "userId": "uuid",
  "agentId": "uuid",
  "sender": "agent",
  "projectId": "uuid",
  "orgId": "uuid",
  "type": "tool_call",
  "content": {}
}
```

**Response:** `200` — SessionEvent

```json
{
  "eventId": "uuid",
  "sessionId": "uuid",
  "userId": "uuid | null",
  "agentId": "uuid | null",
  "sender": "string | null",
  "projectId": "uuid | null",
  "orgId": "uuid | null",
  "type": "string",
  "content": {},
  "timestamp": "datetime"
}
```

**Reference Event Types:**

| Category       | Types                                                                                                                                                        |
|----------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Chat / LLM     | `delta`, `thinking_delta`, `progress`, `tool_call_started`, `tool_call_snapshot`, `tool_call`, `tool_result`, `message_saved`, `agent_instance_updated`, `token_usage`, `done` |
| Spec generation| `spec_saved`, `specs_title`, `specs_summary`, `spec_gen_started`, `spec_gen_progress`, `spec_gen_completed`, `spec_gen_failed`                                |
| Task lifecycle | `task_saved`, `task_started`, `task_completed`, `task_failed`, `task_retrying`, `task_became_ready`, `tasks_became_ready`, `task_output_delta`, `follow_up_task_created`, `file_ops_applied` |
| Loop lifecycle | `loop_started`, `loop_paused`, `loop_stopped`, `loop_finished`, `loop_iteration_summary`, `session_rolled_over`                                              |
| Build / Test   | `build_verification_skipped`, `build_verification_started`, `build_verification_passed`, `build_verification_failed`, `build_fix_attempt`, `test_verification_started`, `test_verification_passed`, `test_verification_failed`, `test_fix_attempt` |
| Git            | `git_committed`, `git_pushed`                                                                                                                                 |
| Other          | `log_line`, `network_event`, `error`                                                                                                                          |

---

### GET /api/sessions/:sessionId/events

**Auth:** JWT

List events for a session, ordered by timestamp ascending.

**Path Parameters:**

| Parameter   | Type | Required | Description       |
|-------------|------|----------|-------------------|
| `sessionId` | UUID | Yes      | The session's UUID |

**Query Parameters:**

| Parameter | Type    | Required | Default | Description                |
|-----------|---------|----------|---------|----------------------------|
| `limit`   | integer | No       | 100     | Max results (capped at 500) |
| `offset`  | integer | No       | 0       | Number of events to skip    |

**Response:** `200` — Array of SessionEvent objects (ordered by `timestamp` ASC).

---

## Artifacts

Generated images and 3D models with iteration tracking.

### POST /api/projects/:projectId/artifacts

Create an artifact.

**Auth:** JWT

**Path params:** `projectId` (UUID)

**Request body:**

```json
{
  "type": "image | model (required)",
  "name": "string (optional)",
  "description": "string (optional)",
  "assetUrl": "string (required — S3 URL or GLB URL)",
  "thumbnailUrl": "string (optional)",
  "originalUrl": "string (optional — unwatermarked version)",
  "parentId": "uuid (optional — parent artifact for iterations)",
  "isIteration": "boolean (default: false)",
  "prompt": "string (optional)",
  "promptMode": "new | remix | edit (optional)",
  "model": "string (optional — e.g., gpt-image-1, tripo-v2)",
  "provider": "string (optional — e.g., openai, google, tripo)",
  "orgId": "uuid (optional)",
  "meta": "{} (optional — freeform JSONB)"
}
```

**Response:** 200

```json
{
  "id": "uuid",
  "projectId": "uuid",
  "orgId": "uuid | null",
  "createdBy": "uuid",
  "type": "image | model",
  "name": "string | null",
  "description": "string | null",
  "assetUrl": "string",
  "thumbnailUrl": "string | null",
  "originalUrl": "string | null",
  "parentId": "uuid | null",
  "isIteration": "boolean",
  "prompt": "string | null",
  "promptMode": "string | null",
  "model": "string | null",
  "provider": "string | null",
  "meta": "{} | null",
  "createdAt": "datetime"
}
```

---

### GET /api/projects/:projectId/artifacts

List artifacts for a project.

**Auth:** JWT

**Query params:** `type` (optional — filter by "image" or "model"), `limit` (default 50, max 100), `offset` (default 0)

**Response:** 200 — Array of Artifact objects (ordered by createdAt DESC)

---

### GET /api/artifacts/:id

Get a single artifact.

**Auth:** JWT

**Response:** 200 — Artifact object

---

### GET /api/artifacts/:id/children

Get iteration children of an artifact.

**Auth:** JWT

**Response:** 200 — Array of Artifact objects (ordered by createdAt ASC)

---

### DELETE /api/artifacts/:id

Delete an artifact.

**Auth:** JWT

**Response:** 204 No Content

---

## Stats

### GET /api/stats

**Auth:** JWT

Get aggregate statistics scoped to a project, organization, or the entire network.

**Query Parameters:**

| Parameter   | Type   | Required                      | Description                                                    |
|-------------|--------|-------------------------------|----------------------------------------------------------------|
| `scope`     | string | Yes                           | One of: `project`, `org`, `network`                            |
| `projectId` | UUID   | Yes (if `scope` = `project`)  | The project to get stats for                                   |
| `orgId`     | UUID   | Yes (if `scope` = `org`)      | The organization to get stats for                              |
| `agentId`   | UUID   | No                            | Filter stats to a specific agent                               |

**Response:** `200`

```json
{
  "totalTasks": 42,
  "pendingTasks": 5,
  "readyTasks": 8,
  "inProgressTasks": 3,
  "blockedTasks": 1,
  "doneTasks": 22,
  "failedTasks": 3,
  "completionPercentage": 52.38,
  "totalTokens": 1250000,
  "totalInputTokens": 8500,
  "totalOutputTokens": 2300,
  "totalEvents": 980,
  "totalAgents": 4,
  "totalSessions": 12,
  "totalTimeSeconds": 3600.5,
  "linesChanged": 1450,
  "totalSpecs": 6,
  "contributors": 3,
  "estimatedCostUsd": 0.085
}
```

| Field                  | Type    | Description                                      |
|------------------------|---------|--------------------------------------------------|
| `totalTasks`           | integer | Total number of tasks                             |
| `pendingTasks`         | integer | Tasks in `pending` status                         |
| `readyTasks`           | integer | Tasks in `ready` status                           |
| `inProgressTasks`      | integer | Tasks in `in_progress` status                     |
| `blockedTasks`         | integer | Tasks in `blocked` status                         |
| `doneTasks`            | integer | Tasks in `done` status                            |
| `failedTasks`          | integer | Tasks in `failed` status                          |
| `completionPercentage` | float   | Percentage of tasks completed                     |
| `totalTokens`          | integer | Total LLM tokens consumed                         |
| `totalInputTokens`    | integer | Total input tokens across all sessions             |
| `totalOutputTokens`   | integer | Total output tokens across all sessions            |
| `totalEvents`          | integer | Total session events recorded                     |
| `totalAgents`          | integer | Number of distinct agents                         |
| `totalSessions`        | integer | Number of sessions                                |
| `totalTimeSeconds`     | float   | Total wall-clock time across sessions             |
| `linesChanged`         | integer | Total lines added and removed                     |
| `totalSpecs`           | integer | Number of specs                                   |
| `contributors`         | integer | Number of distinct contributors                   |
| `estimatedCostUsd`    | float   | Estimated cost in USD, sourced from aura-network   |

---

## Log Entries

### POST /api/projects/:projectId/logs

**Auth:** JWT

Create a log entry.

**Path Parameters:**

| Parameter   | Type | Required | Description        |
|-------------|------|----------|--------------------|
| `projectId` | UUID | Yes      | The project's UUID |

**Request Body:**

| Field            | Type   | Required | Description                                 |
|------------------|--------|----------|---------------------------------------------|
| `orgId`          | UUID   | No       | Organization scope                           |
| `projectAgentId` | UUID   | No       | Agent that produced the log                  |
| `createdBy`      | UUID   | No       | User or agent ID                             |
| `level`          | string | Yes      | One of: `info`, `warn`, `error`, `debug`     |
| `message`        | string | Yes      | Log message text                             |
| `metadata`       | object | No       | Freeform JSONB metadata                      |

```json
{
  "orgId": "uuid",
  "projectAgentId": "uuid",
  "createdBy": "uuid",
  "level": "info",
  "message": "Build succeeded",
  "metadata": { "duration_ms": 1200 }
}
```

**Response:** `200` — LogEntry

```json
{
  "id": "uuid",
  "projectId": "uuid",
  "orgId": "uuid | null",
  "projectAgentId": "uuid | null",
  "createdBy": "uuid | null",
  "level": "info",
  "message": "Build succeeded",
  "metadata": { "duration_ms": 1200 },
  "createdAt": "datetime"
}
```

---

### GET /api/projects/:projectId/logs

**Auth:** JWT

List log entries for a project.

**Path Parameters:**

| Parameter   | Type | Required | Description        |
|-------------|------|----------|--------------------|
| `projectId` | UUID | Yes      | The project's UUID |

**Query Parameters:**

| Parameter | Type    | Required | Default | Description                              |
|-----------|---------|----------|---------|------------------------------------------|
| `level`   | string  | No       | —       | Filter by level: `info`, `warn`, `error`, `debug` |
| `limit`   | integer | No       | 100     | Max results to return                     |
| `offset`  | integer | No       | 0       | Number of entries to skip                 |

**Response:** `200` — Array of LogEntry objects.

---

## Processes

Workflow definitions with visual node graphs, execution runs, and artifacts.

### POST /api/processes

**Auth:** JWT

Create a new process.

**Request Body:**

| Field         | Type     | Required | Description                                              |
|---------------|----------|----------|----------------------------------------------------------|
| `orgId`       | UUID     | Yes      | Organization scope                                       |
| `projectId`   | UUID     | No       | Associated project                                       |
| `folderId`    | UUID     | No       | Parent folder                                            |
| `name`        | string   | Yes      | Process name                                             |
| `description` | string   | No       | Process description                                      |
| `enabled`     | boolean  | No       | Whether the process is enabled (default: true)           |
| `schedule`    | string   | No       | Cron expression for scheduled execution                  |
| `tags`        | string[] | No       | Tags for categorization                                  |

```json
{
  "orgId": "uuid",
  "name": "Daily PR Summary",
  "description": "Fetches open PRs and writes a summary",
  "tags": ["github", "daily"]
}
```

**Response:** `200` — Process

```json
{
  "id": "uuid",
  "orgId": "uuid",
  "createdBy": "uuid",
  "projectId": "uuid | null",
  "folderId": "uuid | null",
  "name": "string",
  "description": "string",
  "enabled": true,
  "schedule": "string | null",
  "tags": ["string"],
  "lastRunAt": "datetime | null",
  "nextRunAt": "datetime | null",
  "createdAt": "datetime",
  "updatedAt": "datetime"
}
```

---

### GET /api/processes

**Auth:** JWT

List processes for an organization.

**Query Parameters:**

| Parameter | Type | Required | Description            |
|-----------|------|----------|------------------------|
| `org_id`  | UUID | Yes      | Filter by organization |

**Response:** `200` — Array of Process objects.

---

### GET /api/processes/:id

**Auth:** JWT

Get a single process by ID.

**Path Parameters:**

| Parameter | Type | Required | Description        |
|-----------|------|----------|--------------------|
| `id`      | UUID | Yes      | The process's UUID |

**Response:** `200` — Process

---

### PUT /api/processes/:id

**Auth:** JWT

Update a process. All fields are optional.

**Path Parameters:**

| Parameter | Type | Required | Description        |
|-----------|------|----------|--------------------|
| `id`      | UUID | Yes      | The process's UUID |

**Request Body:**

| Field         | Type          | Required | Description                    |
|---------------|---------------|----------|--------------------------------|
| `name`        | string        | No       | New name                       |
| `description` | string        | No       | New description                |
| `projectId`   | UUID \| null  | No       | Associate/disassociate project |
| `folderId`    | UUID \| null  | No       | Move to/from folder            |
| `enabled`     | boolean       | No       | Enable/disable                 |
| `schedule`    | string \| null| No       | Update cron schedule           |
| `tags`        | string[]      | No       | Replace tags                   |
| `lastRunAt`   | datetime \| null | No    | Last run timestamp             |
| `nextRunAt`   | datetime \| null | No    | Next scheduled run             |

**Response:** `200` — Process

---

### DELETE /api/processes/:id

**Auth:** JWT

Delete a process. CASCADE deletes all nodes, connections, runs, events, and artifacts.

**Path Parameters:**

| Parameter | Type | Required | Description        |
|-----------|------|----------|--------------------|
| `id`      | UUID | Yes      | The process's UUID |

**Response:** `204 No Content`

---

### GET /api/processes/:id/nodes

**Auth:** JWT

List all nodes for a process.

**Path Parameters:**

| Parameter | Type | Required | Description        |
|-----------|------|----------|--------------------|
| `id`      | UUID | Yes      | The process's UUID |

**Response:** `200` — Array of ProcessNode objects.

```json
{
  "id": "uuid",
  "processId": "uuid",
  "nodeType": "ignition | action | condition | artifact | delay | merge | prompt | sub_process | for_each",
  "label": "string",
  "agentId": "uuid | null",
  "prompt": "string",
  "config": {},
  "positionX": 0.0,
  "positionY": 0.0,
  "createdAt": "datetime",
  "updatedAt": "datetime"
}
```

---

### POST /api/processes/:id/nodes

**Auth:** JWT

Create a node in a process.

**Path Parameters:**

| Parameter | Type | Required | Description        |
|-----------|------|----------|--------------------|
| `id`      | UUID | Yes      | The process's UUID |

**Request Body:**

| Field      | Type   | Required | Description                                                                           |
|------------|--------|----------|---------------------------------------------------------------------------------------|
| `nodeType` | string | Yes      | One of: `ignition`, `action`, `condition`, `artifact`, `delay`, `merge`, `prompt`, `sub_process`, `for_each` |
| `label`    | string | No       | Display label                                                                         |
| `agentId`  | UUID   | No       | Agent to execute this node                                                            |
| `prompt`   | string | No       | Prompt text for the node                                                              |
| `config`   | object | No       | Node-specific configuration                                                           |
| `positionX`| float  | No       | X position in visual editor                                                           |
| `positionY`| float  | No       | Y position in visual editor                                                           |

**Response:** `200` — ProcessNode

---

### PUT /api/processes/:id/nodes/:nodeId

**Auth:** JWT

Update a node. All fields are optional.

**Path Parameters:**

| Parameter | Type | Required | Description      |
|-----------|------|----------|------------------|
| `id`      | UUID | Yes      | Process UUID     |
| `nodeId`  | UUID | Yes      | Node UUID        |

**Request Body:**

| Field      | Type          | Required | Description            |
|------------|---------------|----------|------------------------|
| `label`    | string        | No       | New label              |
| `agentId`  | UUID \| null  | No       | Update/remove agent    |
| `prompt`   | string        | No       | New prompt text        |
| `config`   | object        | No       | New config             |
| `positionX`| float         | No       | New X position         |
| `positionY`| float         | No       | New Y position         |

**Response:** `200` — ProcessNode

---

### DELETE /api/processes/:id/nodes/:nodeId

**Auth:** JWT

Delete a node. CASCADE deletes connections referencing this node.

**Path Parameters:**

| Parameter | Type | Required | Description  |
|-----------|------|----------|--------------|
| `id`      | UUID | Yes      | Process UUID |
| `nodeId`  | UUID | Yes      | Node UUID    |

**Response:** `204 No Content`

---

### GET /api/processes/:id/connections

**Auth:** JWT

List all connections for a process.

**Path Parameters:**

| Parameter | Type | Required | Description        |
|-----------|------|----------|--------------------|
| `id`      | UUID | Yes      | The process's UUID |

**Response:** `200` — Array of ProcessNodeConnection objects.

```json
{
  "id": "uuid",
  "processId": "uuid",
  "sourceNodeId": "uuid",
  "sourceHandle": "string | null",
  "targetNodeId": "uuid",
  "targetHandle": "string | null"
}
```

---

### POST /api/processes/:id/connections

**Auth:** JWT

Create a connection between two nodes.

**Path Parameters:**

| Parameter | Type | Required | Description        |
|-----------|------|----------|--------------------|
| `id`      | UUID | Yes      | The process's UUID |

**Request Body:**

| Field          | Type   | Required | Description        |
|----------------|--------|----------|--------------------|
| `sourceNodeId` | UUID   | Yes      | Source node UUID   |
| `sourceHandle` | string | No       | Source handle name  |
| `targetNodeId` | UUID   | Yes      | Target node UUID   |
| `targetHandle` | string | No       | Target handle name  |

**Response:** `200` — ProcessNodeConnection

---

### DELETE /api/processes/:id/connections/:connectionId

**Auth:** JWT

Delete a connection.

**Path Parameters:**

| Parameter      | Type | Required | Description     |
|----------------|------|----------|-----------------|
| `id`           | UUID | Yes      | Process UUID    |
| `connectionId` | UUID | Yes      | Connection UUID |

**Response:** `204 No Content`

---

### GET /api/processes/:id/runs

**Auth:** JWT

List all runs for a process.

**Path Parameters:**

| Parameter | Type | Required | Description        |
|-----------|------|----------|--------------------|
| `id`      | UUID | Yes      | The process's UUID |

**Response:** `200` — Array of ProcessRun objects.

```json
{
  "id": "uuid",
  "processId": "uuid",
  "status": "pending | running | completed | failed | cancelled",
  "trigger": "scheduled | manual",
  "error": "string | null",
  "startedAt": "datetime",
  "completedAt": "datetime | null",
  "totalInputTokens": 0,
  "totalOutputTokens": 0,
  "costUsd": 0.0,
  "output": "string | null",
  "parentRunId": "uuid | null",
  "inputOverride": "string | null",
  "createdAt": "datetime"
}
```

---

### GET /api/processes/:id/runs/:runId

**Auth:** JWT

Get a single run by ID.

**Path Parameters:**

| Parameter | Type | Required | Description  |
|-----------|------|----------|--------------|
| `id`      | UUID | Yes      | Process UUID |
| `runId`   | UUID | Yes      | Run UUID     |

**Response:** `200` — ProcessRun

---

### GET /api/processes/:id/runs/:runId/events

**Auth:** JWT

List all events for a run.

**Path Parameters:**

| Parameter | Type | Required | Description  |
|-----------|------|----------|--------------|
| `id`      | UUID | Yes      | Process UUID |
| `runId`   | UUID | Yes      | Run UUID     |

**Response:** `200` — Array of ProcessEvent objects.

```json
{
  "id": "uuid",
  "runId": "uuid",
  "nodeId": "uuid",
  "processId": "uuid",
  "status": "pending | running | completed | failed | skipped",
  "inputSnapshot": "string",
  "output": "string",
  "startedAt": "datetime",
  "completedAt": "datetime | null",
  "inputTokens": 0,
  "outputTokens": 0,
  "model": "string | null",
  "contentBlocks": {}
}
```

---

### GET /api/processes/:id/runs/:runId/artifacts

**Auth:** JWT

List all artifacts for a run.

**Path Parameters:**

| Parameter | Type | Required | Description  |
|-----------|------|----------|--------------|
| `id`      | UUID | Yes      | Process UUID |
| `runId`   | UUID | Yes      | Run UUID     |

**Response:** `200` — Array of ProcessArtifact objects.

```json
{
  "id": "uuid",
  "processId": "uuid",
  "runId": "uuid",
  "nodeId": "uuid",
  "artifactType": "report | document | data | media | code | custom",
  "name": "string",
  "filePath": "string",
  "sizeBytes": 0,
  "metadata": {},
  "createdAt": "datetime"
}
```

---

### GET /api/process-artifacts/:id

**Auth:** JWT

Get a single process artifact by ID (metadata only).

**Path Parameters:**

| Parameter | Type | Required | Description    |
|-----------|------|----------|----------------|
| `id`      | UUID | Yes      | Artifact UUID  |

**Response:** `200` — ProcessArtifact

---

### GET /api/process-folders

**Auth:** JWT

List process folders for an organization.

**Query Parameters:**

| Parameter | Type | Required | Description            |
|-----------|------|----------|------------------------|
| `org_id`  | UUID | Yes      | Filter by organization |

**Response:** `200` — Array of ProcessFolder objects.

```json
{
  "id": "uuid",
  "orgId": "uuid",
  "createdBy": "uuid",
  "name": "string",
  "createdAt": "datetime",
  "updatedAt": "datetime"
}
```

---

### POST /api/process-folders

**Auth:** JWT

Create a process folder.

**Request Body:**

| Field   | Type   | Required | Description            |
|---------|--------|----------|------------------------|
| `orgId` | UUID   | Yes      | Organization scope     |
| `name`  | string | Yes      | Folder name            |

**Response:** `200` — ProcessFolder

---

### PUT /api/process-folders/:id

**Auth:** JWT

Update a process folder.

**Path Parameters:**

| Parameter | Type | Required | Description  |
|-----------|------|----------|--------------|
| `id`      | UUID | Yes      | Folder UUID  |

**Request Body:**

| Field  | Type   | Required | Description |
|--------|--------|----------|-------------|
| `name` | string | No       | New name    |

**Response:** `200` — ProcessFolder

---

### DELETE /api/process-folders/:id

**Auth:** JWT

Delete a process folder. Processes in this folder have their `folderId` set to null.

**Path Parameters:**

| Parameter | Type | Required | Description  |
|-----------|------|----------|--------------|
| `id`      | UUID | Yes      | Folder UUID  |

**Response:** `204 No Content`

---

## Internal Endpoints

These endpoints are used for service-to-service communication and require the `X-Internal-Token` header instead of a JWT. Create endpoints include `projectId` and `createdBy` in the request body (since there is no JWT to derive from).

---

### Sessions

#### POST /internal/sessions

**Auth:** Internal

Create a session on behalf of a user (used by aura-router).

**Request Body:**

| Field            | Type   | Required | Description              |
|------------------|--------|----------|--------------------------|
| `projectAgentId` | UUID   | Yes      | The project agent's UUID |
| `projectId`      | UUID   | Yes      | The project's UUID        |
| `orgId`          | UUID   | No       | Organization scope        |
| `createdBy`      | UUID   | Yes      | The originating user      |
| `model`          | string | No       | LLM model identifier      |

```json
{
  "projectAgentId": "uuid",
  "projectId": "uuid",
  "orgId": "uuid",
  "createdBy": "uuid",
  "model": "string"
}
```

**Response:** `200` — Session

---

#### GET /internal/sessions/:id

**Auth:** Internal

Get a single session by ID.

**Path Parameters:**

| Parameter | Type | Required | Description       |
|-----------|------|----------|-------------------|
| `id`      | UUID | Yes      | The session's UUID |

**Response:** `200` — Session

---

#### PUT /internal/sessions/:id

**Auth:** Internal

Update a session. Same request body as [PUT /api/sessions/:id](#put-apisessionsid).

**Path Parameters:**

| Parameter | Type | Required | Description       |
|-----------|------|----------|-------------------|
| `id`      | UUID | Yes      | The session's UUID |

**Response:** `200` — Session

---

#### GET /internal/project-agents/:projectAgentId/sessions

**Auth:** Internal

List all sessions for a project agent.

**Path Parameters:**

| Parameter        | Type | Required | Description              |
|------------------|------|----------|--------------------------|
| `projectAgentId` | UUID | Yes      | The project agent's UUID |

**Response:** `200` — Array of Session objects.

---

### Events

#### POST /internal/events

**Auth:** Internal

Create a session event from an internal service.

**Request Body:** Same schema as [POST /api/sessions/:sessionId/events](#post-apisessionssessionidevents).

**Response:** `200` — SessionEvent

---

#### GET /internal/sessions/:sessionId/events

**Auth:** Internal

List events for a session, ordered by timestamp ascending.

**Path Parameters:**

| Parameter   | Type | Required | Description       |
|-------------|------|----------|-------------------|
| `sessionId` | UUID | Yes      | The session's UUID |

**Query Parameters:**

| Parameter | Type    | Required | Default | Description                |
|-----------|---------|----------|---------|----------------------------|
| `limit`   | integer | No       | 100     | Max results (capped at 500) |
| `offset`  | integer | No       | 0       | Number of events to skip    |

**Response:** `200` — Array of SessionEvent objects (ordered by `timestamp` ASC).

---

### Logs

#### POST /internal/logs

**Auth:** Internal

Create a log entry from an internal service.

**Request Body:**

| Field            | Type   | Required | Description                                 |
|------------------|--------|----------|---------------------------------------------|
| `projectId`      | UUID   | Yes      | The project's UUID                           |
| `orgId`          | UUID   | No       | Organization scope                           |
| `projectAgentId` | UUID   | No       | Agent that produced the log                  |
| `createdBy`      | UUID   | No       | Originating user or agent                    |
| `level`          | string | Yes      | One of: `info`, `warn`, `error`, `debug`     |
| `message`        | string | Yes      | Log message text                             |
| `metadata`       | object | No       | Freeform JSONB metadata                      |

```json
{
  "projectId": "uuid",
  "orgId": "uuid",
  "projectAgentId": "uuid",
  "createdBy": "uuid",
  "level": "error",
  "message": "Task execution failed",
  "metadata": {}
}
```

**Response:** `200` — LogEntry

---

#### GET /internal/projects/:projectId/logs

**Auth:** Internal

List log entries for a project.

**Path Parameters:**

| Parameter   | Type | Required | Description        |
|-------------|------|----------|--------------------|
| `projectId` | UUID | Yes      | The project's UUID |

**Query Parameters:**

| Parameter | Type    | Required | Default | Description                              |
|-----------|---------|----------|---------|------------------------------------------|
| `level`   | string  | No       | —       | Filter by level: `info`, `warn`, `error`, `debug` |
| `limit`   | integer | No       | 100     | Max results to return                     |
| `offset`  | integer | No       | 0       | Number of entries to skip                 |

**Response:** `200` — Array of LogEntry objects.

---

### Project Agents

#### POST /internal/projects/:projectId/agents

**Auth:** Internal

Create a project agent from an internal service.

**Path Parameters:**

| Parameter   | Type | Required | Description        |
|-------------|------|----------|--------------------|
| `projectId` | UUID | Yes      | The project's UUID |

**Request Body:**

| Field       | Type   | Required | Description                |
|-------------|--------|----------|----------------------------|
| `projectId` | UUID   | Yes      | The project's UUID          |
| `createdBy` | UUID   | Yes      | The originating user        |
| `agentId`   | UUID   | Yes      | The agent to attach         |
| `orgId`     | UUID   | No       | Organization scope          |
| `model`     | string | No       | LLM model identifier        |

```json
{
  "projectId": "uuid",
  "createdBy": "uuid",
  "agentId": "uuid",
  "orgId": "uuid",
  "model": "string"
}
```

**Response:** `200` — ProjectAgent

---

#### GET /internal/projects/:projectId/agents

**Auth:** Internal

List all agents for a project.

**Path Parameters:**

| Parameter   | Type | Required | Description        |
|-------------|------|----------|--------------------|
| `projectId` | UUID | Yes      | The project's UUID |

**Response:** `200` — Array of ProjectAgent objects.

---

#### GET /internal/project-agents/:id

**Auth:** Internal

Get a single project agent by ID.

**Path Parameters:**

| Parameter | Type | Required | Description              |
|-----------|------|----------|--------------------------|
| `id`      | UUID | Yes      | The project agent's UUID |

**Response:** `200` — ProjectAgent

---

#### POST /internal/project-agents/:id/status

**Auth:** Internal

Update a project agent's status from an internal service.

**Path Parameters:**

| Parameter | Type | Required | Description              |
|-----------|------|----------|--------------------------|
| `id`      | UUID | Yes      | The project agent's UUID |

**Request Body:**

| Field    | Type   | Required | Description                                           |
|----------|--------|----------|-------------------------------------------------------|
| `status` | string | Yes      | One of: `idle`, `working`, `blocked`, `stopped`, `error` |

```json
{
  "status": "working"
}
```

**Response:** `200` — ProjectAgent

---

#### DELETE /internal/project-agents/:id

**Auth:** Internal

Delete a project agent.

**Path Parameters:**

| Parameter | Type | Required | Description              |
|-----------|------|----------|--------------------------|
| `id`      | UUID | Yes      | The project agent's UUID |

**Response:** `204 No Content`

---

#### GET /internal/projects/:projectId/agents/count

**Auth:** Internal

Get the number of agents attached to a project.

**Path Parameters:**

| Parameter   | Type | Required | Description        |
|-------------|------|----------|--------------------|
| `projectId` | UUID | Yes      | The project's UUID |

**Response:** `200`

```json
{
  "count": 3
}
```

---

### Specs

#### POST /internal/specs

**Auth:** Internal

Create a spec from an internal service.

**Request Body:**

| Field              | Type    | Required | Description                |
|--------------------|---------|----------|----------------------------|
| `projectId`        | UUID    | Yes      | The project's UUID          |
| `createdBy`        | UUID    | Yes      | The originating user        |
| `title`            | string  | Yes      | Spec title                  |
| `orderIndex`       | integer | Yes      | Display/sort order          |
| `markdownContents` | string  | Yes      | Full markdown body          |
| `orgId`            | UUID    | No       | Organization scope          |

```json
{
  "projectId": "uuid",
  "createdBy": "uuid",
  "title": "string",
  "orderIndex": 0,
  "markdownContents": "string",
  "orgId": "uuid"
}
```

**Response:** `200` — Spec

---

#### GET /internal/projects/:projectId/specs

**Auth:** Internal

List all specs for a project, ordered by `orderIndex`.

**Path Parameters:**

| Parameter   | Type | Required | Description        |
|-------------|------|----------|--------------------|
| `projectId` | UUID | Yes      | The project's UUID |

**Response:** `200` — Array of Spec objects (ordered by `orderIndex`).

---

#### GET /internal/specs/:id

**Auth:** Internal

Get a single spec by ID.

**Path Parameters:**

| Parameter | Type | Required | Description    |
|-----------|------|----------|----------------|
| `id`      | UUID | Yes      | The spec's UUID |

**Response:** `200` — Spec

---

#### PUT /internal/specs/:id

**Auth:** Internal

Update a spec. Same request body as [PUT /api/specs/:id](#put-apispecsid).

**Path Parameters:**

| Parameter | Type | Required | Description    |
|-----------|------|----------|----------------|
| `id`      | UUID | Yes      | The spec's UUID |

**Response:** `200` — Spec

---

#### DELETE /internal/specs/:id

**Auth:** Internal

Delete a spec.

**Path Parameters:**

| Parameter | Type | Required | Description    |
|-----------|------|----------|----------------|
| `id`      | UUID | Yes      | The spec's UUID |

**Response:** `204 No Content`

---

### Tasks

#### POST /internal/tasks

**Auth:** Internal

Create a task from an internal service.

**Request Body:**

| Field                    | Type     | Required | Description                          |
|--------------------------|----------|----------|--------------------------------------|
| `projectId`              | UUID     | Yes      | The project's UUID                    |
| `createdBy`              | UUID     | Yes      | The originating user                  |
| `specId`                 | UUID     | Yes      | Parent spec                           |
| `title`                  | string   | Yes      | Task title                            |
| `orderIndex`             | integer  | Yes      | Display/sort order                    |
| `description`            | string   | No       | Task description                      |
| `dependencyTaskIds`      | UUID[]   | No       | Tasks that must complete first        |
| `parentTaskId`           | UUID     | No       | Parent task for sub-task hierarchy     |
| `assignedProjectAgentId` | UUID     | No       | Agent assigned to this task           |
| `orgId`                  | UUID     | No       | Organization scope                    |

```json
{
  "projectId": "uuid",
  "createdBy": "uuid",
  "specId": "uuid",
  "title": "string",
  "orderIndex": 0,
  "description": "string",
  "dependencyTaskIds": ["uuid"],
  "parentTaskId": "uuid",
  "assignedProjectAgentId": "uuid",
  "orgId": "uuid"
}
```

**Response:** `200` — Task

---

#### GET /internal/projects/:projectId/tasks

**Auth:** Internal

List all tasks for a project, ordered by `orderIndex`.

**Path Parameters:**

| Parameter   | Type | Required | Description        |
|-------------|------|----------|--------------------|
| `projectId` | UUID | Yes      | The project's UUID |

**Query Parameters:**

| Parameter | Type   | Required | Description                                                  |
|-----------|--------|----------|--------------------------------------------------------------|
| `status`  | string | No       | Filter by status: `pending`, `ready`, `in_progress`, `blocked`, `done`, `failed` |

**Response:** `200` — Array of Task objects (ordered by `orderIndex`).

---

#### GET /internal/tasks/:id

**Auth:** Internal

Get a single task by ID.

**Path Parameters:**

| Parameter | Type | Required | Description    |
|-----------|------|----------|----------------|
| `id`      | UUID | Yes      | The task's UUID |

**Response:** `200` — Task

---

#### PUT /internal/tasks/:id

**Auth:** Internal

Update task fields. Same request body as [PUT /api/tasks/:id](#put-apitasksid).

**Path Parameters:**

| Parameter | Type | Required | Description    |
|-----------|------|----------|----------------|
| `id`      | UUID | Yes      | The task's UUID |

**Response:** `200` — Task

---

#### DELETE /internal/tasks/:id

**Auth:** Internal

Delete a task.

**Path Parameters:**

| Parameter | Type | Required | Description    |
|-----------|------|----------|----------------|
| `id`      | UUID | Yes      | The task's UUID |

**Response:** `204 No Content`

---

#### POST /internal/tasks/:id/transition

**Auth:** Internal

Transition a task to a new status. Enforces the same state machine as the public endpoint.

**Path Parameters:**

| Parameter | Type | Required | Description    |
|-----------|------|----------|----------------|
| `id`      | UUID | Yes      | The task's UUID |

**Request Body:**

| Field    | Type   | Required | Description    |
|----------|--------|----------|----------------|
| `status` | string | Yes      | Target status  |

```json
{
  "status": "in_progress"
}
```

**Response:** `200` — Task

---

### Artifacts

#### POST /internal/artifacts

Create an artifact. Body includes `projectId` and `createdBy`.

**Auth:** Internal

**Request body:** Same fields as public endpoint, plus `projectId` (UUID) and `createdBy` (UUID).

#### GET /internal/projects/:projectId/artifacts

List artifacts for a project.

**Auth:** Internal

**Query params:** `type`, `limit`, `offset`

#### GET /internal/artifacts/:id

Get an artifact.

**Auth:** Internal

#### DELETE /internal/artifacts/:id

Delete an artifact.

**Auth:** Internal

---

### Stats

#### GET /internal/stats

**Auth:** Internal

Get aggregate statistics. Same query parameters as [GET /api/stats](#get-apistats).

**Response:** `200` — Same response format as [GET /api/stats](#get-apistats).

---

### Processes

#### GET /internal/processes/:id

**Auth:** Internal

Get a single process by ID (used by executor).

**Path Parameters:**

| Parameter | Type | Required | Description        |
|-----------|------|----------|--------------------|
| `id`      | UUID | Yes      | The process's UUID |

**Response:** `200` — Process

---

#### GET /internal/processes/:id/nodes

**Auth:** Internal

List all nodes for a process (used by executor).

**Path Parameters:**

| Parameter | Type | Required | Description        |
|-----------|------|----------|--------------------|
| `id`      | UUID | Yes      | The process's UUID |

**Response:** `200` — Array of ProcessNode objects.

---

#### GET /internal/processes/:id/connections

**Auth:** Internal

List all connections for a process (used by executor).

**Path Parameters:**

| Parameter | Type | Required | Description        |
|-----------|------|----------|--------------------|
| `id`      | UUID | Yes      | The process's UUID |

**Response:** `200` — Array of ProcessNodeConnection objects.

---

#### GET /internal/processes/scheduled

**Auth:** Internal

List all enabled processes that have a schedule and a `nextRunAt` in the past (used by scheduler).

**Response:** `200` — Array of Process objects.

---

#### PUT /internal/processes/:id

**Auth:** Internal

Update a process (used by scheduler to update `nextRunAt`/`lastRunAt`).

**Path Parameters:**

| Parameter | Type | Required | Description        |
|-----------|------|----------|--------------------|
| `id`      | UUID | Yes      | The process's UUID |

**Request Body:** Same as [PUT /api/processes/:id](#put-apiprocessesid).

**Response:** `200` — Process

---

#### POST /internal/process-runs

**Auth:** Internal

Create a process run.

**Request Body:**

| Field           | Type   | Required | Description                              |
|-----------------|--------|----------|------------------------------------------|
| `id`            | UUID   | No       | Client-supplied ID (generated if omitted)|
| `processId`     | UUID   | Yes      | The process's UUID                       |
| `trigger`       | string | No       | `scheduled` or `manual` (default: manual)|
| `parentRunId`   | UUID   | No       | Parent run for sub-processes             |
| `inputOverride` | string | No       | Override input text                      |

**Response:** `200` — ProcessRun

---

#### PUT /internal/process-runs/:id

**Auth:** Internal

Update a process run's status and metrics.

**Path Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id`      | UUID | Yes      | Run UUID    |

**Request Body:**

| Field              | Type             | Required | Description              |
|--------------------|------------------|----------|--------------------------|
| `status`           | string           | No       | New status               |
| `error`            | string \| null   | No       | Error message            |
| `completedAt`      | datetime \| null | No       | Completion timestamp     |
| `totalInputTokens` | integer          | No       | Cumulative input tokens  |
| `totalOutputTokens`| integer          | No       | Cumulative output tokens |
| `costUsd`          | float            | No       | Estimated cost           |
| `output`           | string \| null   | No       | Run output text          |

**Response:** `200` — ProcessRun

---

#### POST /internal/process-events

**Auth:** Internal

Create a process event.

**Request Body:**

| Field           | Type   | Required | Description                              |
|-----------------|--------|----------|------------------------------------------|
| `id`            | UUID   | No       | Client-supplied ID (generated if omitted)|
| `runId`         | UUID   | Yes      | The run's UUID                           |
| `nodeId`        | UUID   | Yes      | The node's UUID                          |
| `processId`     | UUID   | Yes      | The process's UUID                       |
| `status`        | string | No       | Event status (default: pending)          |
| `inputSnapshot` | string | No       | Input data snapshot                      |
| `output`        | string | No       | Event output                             |

**Response:** `200` — ProcessEvent

---

#### PUT /internal/process-events/:id

**Auth:** Internal

Update a process event.

**Path Parameters:**

| Parameter | Type | Required | Description  |
|-----------|------|----------|--------------|
| `id`      | UUID | Yes      | Event UUID   |

**Request Body:**

| Field           | Type             | Required | Description            |
|-----------------|------------------|----------|------------------------|
| `status`        | string           | No       | New status             |
| `output`        | string           | No       | Updated output         |
| `completedAt`   | datetime \| null | No       | Completion timestamp   |
| `inputTokens`   | integer          | No       | Input tokens used      |
| `outputTokens`  | integer          | No       | Output tokens used     |
| `model`         | string           | No       | LLM model used         |
| `contentBlocks` | object           | No       | Content block data     |

**Response:** `200` — ProcessEvent

---

#### POST /internal/process-artifacts

**Auth:** Internal

Create a process artifact (metadata only — file content stays local).

**Request Body:**

| Field          | Type    | Required | Description                              |
|----------------|---------|----------|------------------------------------------|
| `id`           | UUID    | No       | Client-supplied ID (generated if omitted)|
| `processId`    | UUID    | Yes      | The process's UUID                       |
| `runId`        | UUID    | Yes      | The run's UUID                           |
| `nodeId`       | UUID    | Yes      | The node's UUID                          |
| `artifactType` | string  | Yes      | One of: `report`, `document`, `data`, `media`, `code`, `custom` |
| `name`         | string  | Yes      | Artifact name                            |
| `filePath`     | string  | Yes      | Local file path                          |
| `sizeBytes`    | integer | No       | File size in bytes (default: 0)          |
| `metadata`     | object  | No       | Freeform metadata                        |

**Response:** `200` — ProcessArtifact

---

## WebSocket

### WS /ws/events

**Auth:** JWT passed as a query parameter.

```
wss://host/ws/events?token=<JWT>
```

**Broadcast Events:**

| Event                          | Trigger                                  |
|--------------------------------|------------------------------------------|
| `project_agent.status_changed` | Project agent status updated             |
| `task.status_changed`          | Task transitioned to a new status        |
| `session.started`              | New session created                      |
| `session.status_changed`       | Session status updated                   |

**Keepalive:** Ping/pong frames every 30 seconds.
