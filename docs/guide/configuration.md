# Configuration

## How Configuration Works

AxumKit uses `LazyLock` for static configuration that's initialized once on first access.

```rust
static CONFIG: LazyLock<ServerConfig> = LazyLock::new(|| {
    dotenv().ok();  // Load .env file
    // ... parse environment variables
});

impl ServerConfig {
    pub fn get() -> &'static ServerConfig {
        &CONFIG
    }
}
```

This means:
- Configuration is loaded from environment variables (via `.env` or system env)
- All required variables are validated at startup — missing ones cause a panic with a clear error listing
- `ServerConfig::get()` returns a `&'static` reference — zero-cost after initialization

## Two Configs

| Config | Used by | File |
|--------|---------|------|
| `ServerConfig` | `server` | `crates/config/src/server_config.rs` |
| `WorkerConfig` | `worker` | `crates/worker/src/config/worker_config.rs` |

Both follow the same `LazyLock` pattern but load different sets of variables.

## Required vs Optional Variables

**Required variables** use the `require!` macro — the server won't start without them:

```rust
macro_rules! require {
    ($name:expr) => {
        env::var($name).unwrap_or_else(|_| {
            errors.push(format!("  - {} (missing)", $name));
            String::new()
        })
    };
}
```

**Optional variables** use `.unwrap_or()` with defaults:

```rust
redis_session_host: env::var("REDIS_SESSION_HOST")
    .unwrap_or_else(|_| "redis-session".to_string()),
```

## Variable Categories

### Server

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `ENVIRONMENT` | No | `prod` | Set to `dev` or `development` for dev mode |
| `HOST` | Yes | — | Server bind address |
| `PORT` | Yes | — | Server bind port |

### Authentication

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `TOTP_SECRET` | Yes | — | Secret for TOTP backup code hashing |
| `AUTH_SESSION_MAX_LIFETIME_HOURS` | Yes | — | Absolute session expiration (hours) |
| `AUTH_SESSION_SLIDING_TTL_HOURS` | Yes | — | Activity-based TTL extension (hours) |
| `AUTH_SESSION_REFRESH_THRESHOLD` | Yes | — | TTL refresh threshold (%) |
| `AUTH_EMAIL_VERIFICATION_TOKEN_EXPIRE_TIME` | No | `1` | Email verification token TTL (minutes) |
| `AUTH_PASSWORD_RESET_TOKEN_EXPIRE_TIME` | No | `15` | Password reset token TTL (minutes) |
| `AUTH_EMAIL_CHANGE_TOKEN_EXPIRE_TIME` | No | `15` | Email change token TTL (minutes) |
| `OAUTH_PENDING_SIGNUP_TTL_MINUTES` | No | `10` | OAuth pending signup TTL (minutes) |

### Database

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `POSTGRES_HOST` | Yes | — | PostgreSQL or PgDog host |
| `POSTGRES_PORT` | Yes | — | PostgreSQL or PgDog port |
| `POSTGRES_NAME` | Yes | — | Database name |
| `POSTGRES_USER` | Yes | — | Database user |
| `POSTGRES_PASSWORD` | Yes | — | Database password |
| `POSTGRES_MAX_CONNECTION` | No | `30` server / `10` worker | Max pool connections |
| `POSTGRES_MIN_CONNECTION` | No | `5` server / `2` worker | Min pool connections |

For production, use `POSTGRES_HOST=pgdog` and `POSTGRES_PORT=6432` when the app
should connect through PgDog. The example/local/test templates default to direct
`postgres:5432`.

### Redis

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `REDIS_SESSION_HOST` | No | `redis-session` | Session Redis host |
| `REDIS_SESSION_PORT` | No | `6379` | Session Redis port |
| `REDIS_CACHE_HOST` | No | `redis-cache` | Cache Redis host |
| `REDIS_CACHE_PORT` | No | `6379` | Cache Redis port |
| `REDIS_CACHE_TTL` | No | `3600` | Cache TTL (seconds) |
| `REDIS_LOCK_HOST` | No | `127.0.0.1` | Worker lock Redis host |
| `REDIS_LOCK_PORT` | No | `6381` | Worker lock Redis port |

### Stability Layer

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `STABILITY_CONCURRENCY_LIMIT` | No | `500` | Max concurrent requests |
| `STABILITY_BUFFER_SIZE` | No | `1024` | Request queue size |
| `STABILITY_TIMEOUT_SECS` | No | `30` | Per-request timeout (seconds) |

### Image Processor

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `IMAGE_PROCESSOR_URL` | No | `http://127.0.0.1:6701` | smol-image-processor base URL |
| `IMAGE_PROCESSOR_TIMEOUT_SECS` | No | `30` | Image processing request timeout |

See [Environment Variables](/reference/environment) for the full list including OAuth, R2, CORS, SMTP, and more.
