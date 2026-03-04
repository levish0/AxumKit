# Architecture

## Workspace Overview

AxumKit is a Cargo workspace with focused crates:

| Crate | Role |
|-------|------|
| `axumkit-config` | Environment configuration (`ServerConfig`, `WorkerConfig`) |
| `axumkit-constants` | Shared constants (cache keys, NATS subjects, storage keys) |
| `axumkit-entity` | SeaORM database entities |
| `axumkit-errors` | Centralized error model and HTTP mapping |
| `axumkit-dto` | Request/response DTOs and validators |
| `axumkit-server` | API server (routes, services, repositories, middleware) |
| `axumkit-worker` | Background worker (NATS consumers, cron jobs) |

## Layered Server Design

The server crate uses strict layers:

```
API (routes/extractors)
  -> Service (business logic)
     -> Repository (database access)
        -> Entity (SeaORM models)
```

Rules:

- API only calls Service.
- Service calls Repository and bridge clients.
- Repository owns DB query details.

## Request Flow

```
Client
  -> Tower middleware stack
  -> Router (/health-check, /docs, /v0/...)
  -> Extractors (session, validation)
  -> Service + Repository
  -> JSON response
```

Core middleware includes request tracing, CORS, stability protection, cookies, and anonymous user context.

## Shared AppState

Server requests share:

- PostgreSQL write/read pools
- R2 client (assets bucket)
- Redis session/cache clients
- NATS JetStream client (worker jobs)
- NATS core client + broadcast channel (SSE)
- Reqwest HTTP client
- MeiliSearch client

## Authentication Model

AxumKit uses session-based auth with Redis-backed sessions.

- Email/password login supports optional TOTP second step.
- OAuth (Google/GitHub) supports login and account linking.
- Sessions use sliding TTL with an absolute max lifetime.

## Worker Model

The worker is a separate binary that consumes JetStream jobs and runs cron tasks.

Consumers:

- `axumkit.jobs.email`
- `axumkit.jobs.index.user`
- `axumkit.jobs.reindex.users`

Cron tasks:

- Cleanup
- Sitemap generation to R2 assets bucket

## Server to Worker Bridge

The server publishes jobs via JetStream through `bridge::worker_client::*` helpers.

Examples:

- Send verification/reset emails
- Queue user indexing
- Trigger user reindex

## Error Handling

All domain errors are represented by `Errors` and converted to a standard JSON format:

```json
{
  "status": 400,
  "code": "domain:reason",
  "details": "..."
}
```

`details` is only returned in development mode.
