# AxumKit

Production-ready Rust web API template.

## What's Included

- **Web Framework**: Axum with layered architecture (API → Service → Repository → Entity)
- **Database**: PostgreSQL + SeaORM with migrations
- **Auth**: Session-based auth (Redis), OAuth2 (Google, GitHub)
- **Background Jobs**: NATS JetStream worker (email, search indexing, cron)
- **Storage**: SeaweedFS (content), Cloudflare R2 (files)
- **Search**: MeiliSearch full-text search
- **API Docs**: OpenAPI/Swagger (debug builds)

## Architecture

```
API Layer (src/api/)           # HTTP handlers, routing, OpenAPI docs
    ↓
Service Layer (src/service/)   # Business logic, validation
    ↓
Repository Layer (src/repository/)  # Database queries
    ↓
Entity Layer (src/entity/)     # SeaORM models
```

**Key components:**
- `AppState` - Shared state (DB conn, Redis, HTTP client)
- `SessionContext` - Authenticated user info extracted by middleware
- `Errors` - Centralized error enum, auto-converts to HTTP responses

## Error Handling

All errors go through the `Errors` enum in `crates/axumkit-errors/`. Domain-specific variants (user, auth, oauth, session) automatically convert to appropriate HTTP status codes.

```rust
pub async fn handler() -> Result<Json<Response>, Errors> {
    let user = repo::get_by_id(&conn, id)
        .await
        .map_err(|_| Errors::UserNotFound)?;
    Ok(Json(response))
}
```

- Development: detailed error info exposed
- Production: internal details hidden

## Configuration

Environment variables loaded from `.env` via `ServerConfig::get()` (uses `LazyLock`). See `.env.example` for all options.

| Category | Variables |
|----------|-----------|
| Server | `HOST`, `PORT`, `ENVIRONMENT` |
| Auth | `TOTP_SECRET`, `AUTH_SESSION_*`, `OAUTH_PENDING_SIGNUP_TTL_MINUTES` |
| Database | `POSTGRES_USER`, `POSTGRES_PASSWORD`, `POSTGRES_HOST`, `POSTGRES_PORT`, `POSTGRES_NAME` |
| Redis | `REDIS_SESSION_HOST`, `REDIS_SESSION_PORT`, `REDIS_CACHE_HOST`, `REDIS_CACHE_PORT` |
| OAuth | `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`, `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET` |
| Storage | `R2_*`, `SEAWEEDFS_ENDPOINT` |
| Search | `MEILISEARCH_HOST`, `MEILISEARCH_API_KEY` |
| Queue | `NATS_URL` |
| CORS | `CORS_ALLOWED_ORIGINS`, `CORS_ALLOWED_HEADERS`, `CORS_MAX_AGE` |

## Quick Start

```bash
git clone https://github.com/shiueo/AxumKit.git
cd AxumKit
cp .env.example .env

# Run migrations
cd crates/migration && cargo run && cd ../..

# Start server & worker
cargo run -p axumkit-server
cargo run -p axumkit-worker  # separate terminal
```

## License

MIT