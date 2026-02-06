# Architecture

## Crate Overview

AxumKit is organized as a Cargo workspace with 7 internal crates:

| Crate | Role |
|-------|------|
| `axumkit-config` | Environment configuration (`ServerConfig`, `WorkerConfig`) |
| `axumkit-constants` | Shared constants (action log actions, NATS subjects, storage keys) |
| `axumkit-entity` | SeaORM database models |
| `axumkit-errors` | Centralized error types with HTTP response mapping |
| `axumkit-dto` | Request/response data transfer objects |
| `axumkit-server` | API server (routes, services, middleware, repository) |
| `axumkit-worker` | Background job processor (NATS consumers, cron jobs) |

## Layered Architecture

The server crate follows a strict layered architecture:

```
┌─────────────────────────────────────┐
│          API Layer (routes)         │  HTTP handlers, extractors
├─────────────────────────────────────┤
│        Service Layer (service)      │  Business logic
├─────────────────────────────────────┤
│      Repository Layer (repository)  │  Database queries
├─────────────────────────────────────┤
│        Entity Layer (entity)        │  SeaORM models
└─────────────────────────────────────┘
```

**Rules:**
- API layer calls Service layer only
- Service layer calls Repository layer and Bridge (worker client)
- Repository layer calls Entity layer (SeaORM queries)
- Never skip a layer (e.g., API must not call Repository directly)

## Request Flow

```
Client Request
    │
    ▼
┌─────────────────────────────────┐
│  Tower Middleware Stack         │
│  1. SetRequestId (X-Request-ID) │
│  2. TraceLayer (structured log) │
│  3. CORS                       │
│  4. Stability (concurrency,    │
│     buffer, timeout)           │
│  5. CookieManager              │
│  6. AnonymousUser (cookie)     │
└─────────────┬───────────────────┘
              │
              ▼
┌─────────────────────────────────┐
│  Router                        │
│  ├─ /health-check              │
│  ├─ /docs (Swagger UI, debug)  │
│  └─ /v0/...                    │
│     ├─ Extractor: RequiredSession│
│     │  (cookie → Redis lookup)  │
│     └─ Handler                  │
│        ├─ Service logic         │
│        ├─ Repository queries    │
│        └─ Bridge (→ NATS jobs)  │
└─────────────────────────────────┘
              │
              ▼
    JSON Response (or Errors → auto-converted)
```

## AppState

All shared resources live in `AppState`, cloned into each request via Axum's state system:

```rust
pub struct AppState {
    pub write_db: PostgresqlClient,      // Primary PostgreSQL
    pub read_db: PostgresqlClient,       // Replica PostgreSQL
    pub r2_client: R2Client,             // Cloudflare R2 (S3)
    pub seaweedfs_client: SeaweedFsClient, // SeaweedFS blob storage
    pub redis_session: RedisClient,      // Sessions, tokens, rate-limit
    pub redis_cache: RedisClient,        // Document cache (LRU)
    pub worker: WorkerClient,            // NATS JetStream (job queue)
    pub nats_client: NatsClient,         // NATS Core (pub/sub)
    pub eventstream_tx: EventStreamSender, // SSE broadcast channel
    pub http_client: HttpClient,         // reqwest (OAuth, external APIs)
    pub meilisearch_client: MeilisearchClient,
}
```

## Database Architecture

AxumKit uses a **write/read split**:

- **Write DB (Primary):** All INSERT, UPDATE, DELETE operations
- **Read DB (Replica):** All SELECT queries

Both are configured with independent connection pool sizes.

### Entities

```
users
├── id (UUID v7, PK)
├── handle (unique, max 20)
├── display_name
├── email (unique)
├── password (nullable — OAuth users)
├── totp_secret, totp_enabled_at, totp_backup_codes
├── profile_image, banner_image
└── created_at

posts
├── id (UUID v7, PK)
├── author_id (FK → users)
├── title
├── storage_key (SeaweedFS path)
├── created_at, updated_at

user_oauth_connections
├── id (UUID, PK)
├── user_id (FK → users, CASCADE)
├── provider (enum: google, github)
├── provider_user_id
└── created_at

action_logs
├── id (UUID, PK)
├── action (e.g., "post:create")
├── actor_id (FK → users, SET NULL)
├── actor_ip (inet)
├── resource_type (enum: user, post)
├── resource_id
├── summary, metadata (JSONB)
└── created_at
```

## Authentication Architecture

AxumKit uses **session-based authentication** with Redis:

```
Login Request
    │
    ├─ Email/Password → Argon2 verify
    │   └─ TOTP enabled? → Return temp_token, require /totp/verify
    │
    ├─ OAuth → Provider code exchange → Find/create user
    │
    └─ Session created in Redis
       Key: "session:{uuid}"
       TTL: sliding (default 168h), max lifetime (default 720h)
       │
       └─ Set-Cookie: session_id={uuid}
```

Session extraction in handlers:

```rust
// Required — returns 401 if not authenticated
pub async fn handler(RequiredSession(session): RequiredSession) { ... }

// Optional — returns None for anonymous users
pub async fn handler(OptionalSession(session): OptionalSession) { ... }
```

## Worker Architecture

The worker is a separate binary that processes background jobs:

```
┌──────────────────────────────────────────────┐
│              axumkit-worker                   │
│                                              │
│  ┌─────────────────────────────────────────┐ │
│  │         NATS JetStream Consumers        │ │
│  │  ┌─────────┐ ┌───────────┐ ┌─────────┐ │ │
│  │  │  Email  │ │Index Post │ │Index User│ │ │
│  │  └─────────┘ └───────────┘ └─────────┘ │ │
│  │  ┌───────────┐ ┌───────────┐ ┌───────┐ │ │
│  │  │Reindex    │ │Reindex    │ │Delete  │ │ │
│  │  │Posts      │ │Users      │ │Content │ │ │
│  │  └───────────┘ └───────────┘ └───────┘ │ │
│  └─────────────────────────────────────────┘ │
│                                              │
│  ┌─────────────────────────────────────────┐ │
│  │           Cron Scheduler                │ │
│  │  Cleanup (Sat 4AM)                      │ │
│  │  Sitemap (Sun 3AM)                      │ │
│  │  Orphan Blob Cleanup (Fri 5AM)          │ │
│  └─────────────────────────────────────────┘ │
└──────────────────────────────────────────────┘
```

### Server → Worker Communication

The server publishes jobs to NATS JetStream via the **Bridge** layer:

```
Server Handler
    │
    └─ bridge::worker_client::email::send_verification_email()
       │
       └─ jetstream.publish("axumkit.jobs.email", payload)
          │
          └─ Worker email consumer picks up the message
             │
             └─ Render template → Send via SMTP
```

### NATS Streams

| Stream | Subject | Consumer | Purpose |
|--------|---------|----------|---------|
| `axumkit_jobs_email` | `axumkit.jobs.email` | `email-consumer` | Email delivery |
| `axumkit_jobs_index_post` | `axumkit.jobs.index.post` | `post-index-consumer` | Post indexing |
| `axumkit_jobs_index_user` | `axumkit.jobs.index.user` | `user-index-consumer` | User indexing |
| `axumkit_jobs_reindex_posts` | `axumkit.jobs.reindex.posts` | `reindex-posts-consumer` | Bulk post reindex |
| `axumkit_jobs_reindex_users` | `axumkit.jobs.reindex.users` | `reindex-users-consumer` | Bulk user reindex |
| `axumkit_jobs_delete_content` | `axumkit.jobs.storage.delete_content` | `delete-content-consumer` | Storage cleanup |

All streams use **WorkQueue retention** — messages are removed after acknowledgment.

## EventStream (SSE)

Real-time action log updates use NATS Core Pub/Sub + Tokio broadcast:

```
Action occurs (e.g., post created)
    │
    └─ Server publishes to NATS subject: axumkit.realtime.events
       │
       └─ EventStream subscriber (background task) receives
          │
          └─ Broadcasts to all connected SSE clients via tokio::broadcast
```

This enables horizontal scaling — multiple server instances share events through NATS.

## Error Handling

All errors flow through a centralized system:

```rust
// Service returns Errors
fn create_post(...) -> Result<Post, Errors> {
    let user = repo::get_by_id(&db, id).await?;  // auto-converts DbErr
    if !user.verified { return Err(Errors::UserNotVerified); }
    ...
}

// Errors auto-convert to HTTP response
impl IntoResponse for Errors {
    fn into_response(self) -> Response {
        // Chain of domain handlers: user → oauth → session → ...
        let (status, code, details) = user_handler::map_response(&self)
            .or_else(|| oauth_handler::map_response(&self))
            .or_else(|| ...)
            .unwrap_or((500, "UNKNOWN_ERROR", None));

        // details only in dev mode
        Json(ErrorResponse { status, code, details })
    }
}
```

See [Error Codes](/reference/error-codes) for the complete list.
