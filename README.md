<h1 align="center">aura-storage</h1>

<p align="center">
  <b>The execution data layer for autonomous agent workflows.</b>
</p>

## Overview

aura-storage stores all project execution data for the AURA platform — specs, tasks, sessions, events, project agents, and log entries. All AURA clients (desktop, web, mobile) and aura-swarm (cloud agent orchestration) connect to this service for execution state.

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
| `AURA_NETWORK_URL` | No | aura-network base URL for cost data in stats |
| `AURA_NETWORK_TOKEN` | No | Internal service token for aura-network |
| `CORS_ORIGINS` | No | Comma-separated allowed origins. Omit for permissive (dev mode) |
| `RUST_LOG` | No | Tracing filter (default: `aura_storage=debug,tower_http=debug,info`) |

---

## Authentication

All API endpoints require a JWT in the `Authorization: Bearer <token>` header. Tokens are obtained by logging in via zOS API. Same tokens as aura-network — both RS256 (Auth0 JWKS) and HS256 (shared secret) are accepted.

Internal (service-to-service) endpoints use `X-Internal-Token` header instead.

Unlike aura-network, this service does **not** auto-create users. The `created_by` field stores the user's UUID from the JWT but does not validate it against a users table — that lives in aura-network.

---

## API Reference

See [docs/api.md](docs/api.md) for the full API reference.

---

## Integration Guide

### From aura-code (Desktop)

```
Auth:       zOS API (login) -> gets JWT
Network:    aura-network (profiles, orgs, agents, feed, projects)
Storage:    aura-storage (specs, tasks, sessions, events, project agents, logs)
Billing:    zero-payments-server (credit balance, debit via JWT)
Local:      RocksDB (terminal, filesystem, settings)
```

On app load: fetch projects from aura-network, then fetch execution data from aura-storage for active projects.

### From aura-swarm (Cloud Agents)

```
1. Update agent status:    POST /internal/project-agents/:id/status
2. Create session:         POST /internal/sessions
3. Write events:           POST /internal/events (per LLM call)
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
| **aura-storage-events** | Session events (typed event stream) |
| **aura-storage-logs** | Structured log entries |

## License

MIT
