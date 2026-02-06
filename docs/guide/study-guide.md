# Study Guide

This guide provides a recommended reading order for understanding the AxumKit codebase. Follow the sequence below — each step builds on the previous one.

## Overview

AxumKit is a Rust monorepo with 7 internal crates. The dependency graph flows like this:

```
axumkit-config ─────────────────────┐
axumkit-constants ──────────────────┤
axumkit-entity ─────────────────────┤
axumkit-errors ─────────────────────┼──▶ axumkit-server
axumkit-dto ────────────────────────┤       │
                                    │       ▼
                                    └──▶ axumkit-worker
```

## Step 1: Configuration (`axumkit-config`)

**Start here.** This crate is the simplest and shows how the entire app is configured.

| File | What to learn |
|------|---------------|
| `src/server_config.rs` | `LazyLock` pattern for static config, `require!` macro, env var loading |

Key takeaways:
- All config is loaded once via `ServerConfig::get()` (returns `&'static ServerConfig`)
- Missing required env vars are collected and panic'd at startup — fail-fast pattern
- Optional vars use `.unwrap_or()` with sensible defaults
- Two Redis instances: session (persistent, AOF) and cache (volatile, LRU)
- Write/Read database separation for PostgreSQL

## Step 2: Entities (`axumkit-entity`)

Database models. Read these to understand the data layer.

| File | What to learn |
|------|---------------|
| `src/users.rs` | User model with TOTP fields, optional password (OAuth users) |
| `src/posts.rs` | Post model with `storage_key` pointing to SeaweedFS |
| `src/user_oauth_connections.rs` | OAuth provider linking, `OAuthProvider` enum |
| `src/action_logs.rs` | Audit log with `IpNetwork` type, JSONB metadata |
| `src/common.rs` | Shared enums: `OAuthProvider`, `ActionResourceType` |

Key takeaways:
- UUIDs everywhere (v7 for time-ordering)
- `password` is `Option<String>` — OAuth-only users have no password
- `totp_backup_codes` uses PostgreSQL array type
- Relations defined via SeaORM's `DeriveRelation`

## Step 3: Error System (`axumkit-errors`)

The centralized error handling system. This is a critical architectural pattern.

| File | What to learn |
|------|---------------|
| `src/errors.rs` | `Errors` enum, `IntoResponse` impl, handler chain pattern |
| `src/protocol.rs` | Error code string constants (e.g., `"user:not_found"`) |
| `src/handlers/*.rs` | Domain-specific `map_response()` and `log_error()` functions |

Key takeaways:
- Every `Errors` variant auto-converts to an HTTP response
- Handler chain: each domain handler gets a chance to match the error
- Error codes follow `domain:operation` format
- `details` field only shows in dev mode (`ServerConfig::get().is_dev`)
- `ServiceResult<T>` is `Result<T, Errors>` — used everywhere

## Step 4: DTOs (`axumkit-dto`)

Request/response types organized by domain.

| Directory | What to learn |
|-----------|---------------|
| `src/auth/request/` | Login, TOTP verify, password reset request shapes |
| `src/auth/response/` | Login response with TOTP-required flag |
| `src/oauth/request/` | OAuth code exchange, link/unlink requests |
| `src/posts/request/` | Post creation, pagination queries |
| `src/search/request/` | Search query params |
| `src/validator/` | `ValidatedJson`, `ValidatedQuery` — request validation wrappers |

Key takeaways:
- All DTOs derive `Serialize`/`Deserialize` + `ToSchema` (for OpenAPI)
- Validation via `validator` crate with `#[validate]` attributes
- Custom extractors (`ValidatedJson`, `ValidatedQuery`) validate before handler runs

## Step 5: Constants (`axumkit-constants`)

Shared constants used by both server and worker.

| File | What to learn |
|------|---------------|
| `src/action_log_actions.rs` | `ActionLogAction` enum with `"resource:operation"` format |
| `src/nats_subjects.rs` | NATS subject for realtime events |
| `src/storage_keys.rs` | R2/SeaweedFS key prefixes, image size limits |

## Step 6: Server (`axumkit-server`)

The main API server. This is the largest crate. Read in this order:

### 6a. State & Connections

| File | What to learn |
|------|---------------|
| `src/state.rs` | `AppState` struct — all shared resources |
| `src/connection/*.rs` | How each external service is connected |
| `src/main.rs` | Server startup, middleware stack assembly |

### 6b. Middleware

| File | What to learn |
|------|---------------|
| `src/middleware/anonymous_user.rs` | Anonymous user cookie (UUIDv7, 365-day TTL) |
| `src/middleware/cors.rs` | CORS from env vars |
| `src/middleware/rate_limit.rs` | Sliding window with Redis Lua script |
| `src/middleware/stability.rs` | Concurrency limit, buffer, timeout (Tower layers) |
| `src/middleware/trace_layer_config.rs` | Request ID propagation |

Middleware stack order (outermost to innermost):
```
Request ID → Trace → CORS → Stability → Cookie → Anonymous User → Routes
```

### 6c. Extractors

| File | What to learn |
|------|---------------|
| `src/extractors/session.rs` | `RequiredSession` / `OptionalSession` — session from cookie → Redis |
| `src/extractors/turnstile.rs` | Cloudflare Turnstile bot protection |

### 6d. Routes (API Layer)

| Directory | What to learn |
|-----------|---------------|
| `src/api/routes.rs` | Top-level router: health + `/v0` + Swagger |
| `src/api/v0/routes/auth/routes.rs` | All auth endpoints |
| `src/api/v0/routes/posts/routes.rs` | CRUD endpoints |
| `src/api/v0/routes/user/routes.rs` | Profile management |
| `src/api/v0/routes/search/routes.rs` | MeiliSearch query endpoints |
| `src/api/v0/routes/stream/routes.rs` | SSE event stream |

### 6e. Services (Business Logic)

| Directory | What to learn |
|-----------|---------------|
| `src/service/auth/session.rs` | Redis session CRUD, sliding TTL refresh |
| `src/service/auth/login.rs` | Password verification, TOTP check, session creation |
| `src/service/oauth/` | OAuth2 flow: authorize URL → code exchange → find/create user |
| `src/service/posts/` | Post CRUD with SeaweedFS storage |

### 6f. Repository (Database Layer)

| Pattern | Example |
|---------|---------|
| `find_by_*` | Returns `Option<Model>` |
| `get_by_*` | Returns `Result<Model, Errors>` (errors if not found) |

### 6g. Bridge (Server → Worker Communication)

| File | What to learn |
|------|---------------|
| `src/bridge/worker_client/email.rs` | Publish email jobs to NATS |
| `src/bridge/worker_client/index.rs` | Publish search index jobs |
| `src/bridge/worker_client/storage.rs` | Publish delete content jobs |

## Step 7: Worker (`axumkit-worker`)

Background job processor. Read after understanding the server.

| File | What to learn |
|------|---------------|
| `src/main.rs` | Worker startup, consumer spawning |
| `src/nats/streams.rs` | JetStream stream definitions |
| `src/nats/consumer.rs` | Generic consumer with retry/backoff |
| `src/jobs/email/mod.rs` | Email rendering (MRML + MiniJinja) and sending |
| `src/jobs/index/post.rs` | MeiliSearch post indexing |
| `src/jobs/cron/mod.rs` | Cron scheduler setup |
| `src/jobs/cron/sitemap.rs` | Sitemap generation and R2 upload |

Key takeaways:
- `NatsConsumer` is a generic consumer: deserialize → handle → ack/nak
- Exponential backoff: 1s, 2s, 4s, 8s, 16s (5 retries)
- Concurrency controlled via `Semaphore`
- Cron jobs: cleanup (Sat 4AM), sitemap (Sun 3AM), orphan cleanup (Fri 5AM)

## Recommended Practice

After reading through the codebase:

1. **Add a new field to the User entity** — touch entity, migration, DTO, service, repository
2. **Add a new API endpoint** — follow the existing pattern in `api/v0/routes/`
3. **Add a new worker job** — define stream, publisher (bridge), consumer
4. **Write an e2e test** — see `crates/e2e/` for existing examples
