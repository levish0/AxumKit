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
| `ServerConfig` | `axumkit-server` | `crates/axumkit-config/src/server_config.rs` |
| `WorkerConfig` | `axumkit-worker` | `crates/axumkit-worker/src/config/worker_config.rs` |

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
| `POSTGRES_WRITE_HOST` | Yes | — | Primary DB host |
| `POSTGRES_WRITE_PORT` | Yes | — | Primary DB port |
| `POSTGRES_WRITE_NAME` | Yes | — | Primary DB name |
| `POSTGRES_WRITE_USER` | Yes | — | Primary DB user |
| `POSTGRES_WRITE_PASSWORD` | Yes | — | Primary DB password |
| `POSTGRES_WRITE_MAX_CONNECTION` | No | `100` | Max pool connections |
| `POSTGRES_WRITE_MIN_CONNECTION` | No | `10` | Min pool connections |
| `POSTGRES_READ_*` | Yes | — | Same set for read replica |

### Redis

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `REDIS_SESSION_HOST` | No | `redis-session` | Session Redis host |
| `REDIS_SESSION_PORT` | No | `6379` | Session Redis port |
| `REDIS_CACHE_HOST` | No | `redis-cache` | Cache Redis host |
| `REDIS_CACHE_PORT` | No | `6379` | Cache Redis port |
| `REDIS_CACHE_TTL` | No | `3600` | Cache TTL (seconds) |

### Stability Layer

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `STABILITY_CONCURRENCY_LIMIT` | No | `500` | Max concurrent requests |
| `STABILITY_BUFFER_SIZE` | No | `1024` | Request queue size |
| `STABILITY_TIMEOUT_SECS` | No | `30` | Per-request timeout (seconds) |

See [Environment Variables](/reference/environment) for the full list including OAuth, R2, CORS, SMTP, and more.
