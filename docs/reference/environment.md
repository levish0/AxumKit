# Environment Variables

Complete list of all environment variables used by AxumKit. Variables marked **Required** will cause a panic at startup if missing.

## Server (`axumkit-server`)

### General

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `ENVIRONMENT` | No | — | Set to `dev` or `development` for dev mode |
| `HOST` | Yes | — | Server bind address (e.g., `0.0.0.0`) |
| `PORT` | Yes | — | Server bind port (e.g., `8000`) |

### Authentication & Sessions

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `TOTP_SECRET` | Yes | — | Secret for TOTP backup code hashing |
| `AUTH_SESSION_MAX_LIFETIME_HOURS` | Yes | — | Absolute session expiration (hours) |
| `AUTH_SESSION_SLIDING_TTL_HOURS` | Yes | — | Activity-based TTL extension (hours) |
| `AUTH_SESSION_REFRESH_THRESHOLD` | Yes | — | TTL refresh threshold (0-100%) |
| `AUTH_EMAIL_VERIFICATION_TOKEN_EXPIRE_TIME` | No | `1` | Email verification token TTL (minutes) |
| `AUTH_PASSWORD_RESET_TOKEN_EXPIRE_TIME` | No | `15` | Password reset token TTL (minutes) |
| `AUTH_EMAIL_CHANGE_TOKEN_EXPIRE_TIME` | No | `15` | Email change token TTL (minutes) |
| `OAUTH_PENDING_SIGNUP_TTL_MINUTES` | No | `10` | OAuth pending signup TTL (minutes) |

### PostgreSQL Write (Primary)

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `POSTGRES_WRITE_HOST` | Yes | — | Primary DB host |
| `POSTGRES_WRITE_PORT` | Yes | — | Primary DB port |
| `POSTGRES_WRITE_NAME` | Yes | — | Primary DB name |
| `POSTGRES_WRITE_USER` | Yes | — | Primary DB user |
| `POSTGRES_WRITE_PASSWORD` | Yes | — | Primary DB password |
| `POSTGRES_WRITE_MAX_CONNECTION` | No | `100` | Max connection pool size |
| `POSTGRES_WRITE_MIN_CONNECTION` | No | `10` | Min connection pool size |

### PostgreSQL Read (Replica)

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `POSTGRES_READ_HOST` | Yes | — | Replica DB host |
| `POSTGRES_READ_PORT` | Yes | — | Replica DB port |
| `POSTGRES_READ_NAME` | Yes | — | Replica DB name |
| `POSTGRES_READ_USER` | Yes | — | Replica DB user |
| `POSTGRES_READ_PASSWORD` | Yes | — | Replica DB password |
| `POSTGRES_READ_MAX_CONNECTION` | No | `100` | Max connection pool size |
| `POSTGRES_READ_MIN_CONNECTION` | No | `10` | Min connection pool size |

### Redis

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `REDIS_SESSION_HOST` | No | `redis-session` | Session Redis host |
| `REDIS_SESSION_PORT` | No | `6379` | Session Redis port |
| `REDIS_CACHE_HOST` | No | `redis-cache` | Cache Redis host |
| `REDIS_CACHE_PORT` | No | `6379` | Cache Redis port |
| `REDIS_CACHE_TTL` | No | `3600` | Cache TTL in seconds |

### Google OAuth

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `GOOGLE_CLIENT_ID` | Yes | — | Google OAuth client ID |
| `GOOGLE_CLIENT_SECRET` | Yes | — | Google OAuth client secret |
| `GOOGLE_REDIRECT_URI` | Yes | — | Google OAuth callback URL |

### GitHub OAuth

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `GITHUB_CLIENT_ID` | Yes | — | GitHub OAuth client ID |
| `GITHUB_CLIENT_SECRET` | Yes | — | GitHub OAuth client secret |
| `GITHUB_REDIRECT_URI` | Yes | — | GitHub OAuth callback URL |

### Cloudflare R2

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `R2_ENDPOINT` | Yes | — | R2 S3-compatible endpoint |
| `R2_REGION` | Yes | — | R2 region |
| `R2_PUBLIC_DOMAIN` | Yes | — | Public URL for R2 assets |
| `R2_BUCKET_NAME` | Yes | — | R2 bucket name |
| `R2_ACCESS_KEY_ID` | Yes | — | R2 access key |
| `R2_SECRET_ACCESS_KEY` | Yes | — | R2 secret key |

### Cloudflare Turnstile

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `TURNSTILE_SECRET_KEY` | Yes | — | Turnstile verification secret |

### SeaweedFS

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `SEAWEEDFS_ENDPOINT` | Yes | — | SeaweedFS filer endpoint |

### NATS

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `NATS_URL` | No | `nats://localhost:4222` | NATS server URL |

### MeiliSearch

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `MEILISEARCH_HOST` | No | `http://localhost:7700` | MeiliSearch URL |
| `MEILISEARCH_API_KEY` | No | — | MeiliSearch API key |

### CORS

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `CORS_ALLOWED_ORIGINS` | No | `[]` | Comma-separated allowed origins |
| `CORS_ALLOWED_HEADERS` | No | `[]` | Comma-separated allowed headers |
| `CORS_MAX_AGE` | No | — | CORS preflight cache duration (seconds) |

### Cookies

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `COOKIE_DOMAIN` | No | — | Cookie domain (e.g., `.example.com`) |

### Stability Layer

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `STABILITY_CONCURRENCY_LIMIT` | No | `500` | Max concurrent requests |
| `STABILITY_BUFFER_SIZE` | No | `1024` | Request queue buffer size |
| `STABILITY_TIMEOUT_SECS` | No | `30` | Per-request timeout (seconds) |

---

## Worker (`axumkit-worker`)

### SMTP

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `SMTP_HOST` | Yes | — | SMTP server host |
| `SMTP_PORT` | No | `587` | SMTP server port |
| `SMTP_USER` | Yes | — | SMTP username |
| `SMTP_PASSWORD` | Yes | — | SMTP password |
| `SMTP_TLS` | No | `true` | Enable TLS for SMTP |
| `EMAILS_FROM_EMAIL` | Yes | — | Sender email address |
| `EMAILS_FROM_NAME` | No | `SevenWiki` | Sender display name |

### Frontend

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `FRONTEND_HOST` | Yes | — | Frontend base URL |
| `PROJECT_NAME` | Yes | — | Project name (used in emails) |
| `FRONTEND_PATH_VERIFY_EMAIL` | Yes | — | Path for email verification page |
| `FRONTEND_PATH_RESET_PASSWORD` | Yes | — | Path for password reset page |
| `FRONTEND_PATH_CONFIRM_EMAIL_CHANGE` | Yes | — | Path for email change confirmation |

### Database (Write Only)

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `POSTGRES_WRITE_HOST` | Yes | — | Primary DB host |
| `POSTGRES_WRITE_PORT` | Yes | — | Primary DB port |
| `POSTGRES_WRITE_NAME` | Yes | — | Primary DB name |
| `POSTGRES_WRITE_USER` | Yes | — | Primary DB user |
| `POSTGRES_WRITE_PASSWORD` | Yes | — | Primary DB password |
| `POSTGRES_WRITE_MAX_CONNECTION` | No | `10` | Max connection pool size |
| `POSTGRES_WRITE_MIN_CONNECTION` | No | `2` | Min connection pool size |

### Other Services

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `NATS_URL` | No | `nats://localhost:4222` | NATS server URL |
| `MEILISEARCH_HOST` | No | `http://localhost:7700` | MeiliSearch URL |
| `MEILISEARCH_API_KEY` | No | — | MeiliSearch API key |
| `REDIS_CACHE_HOST` | No | `127.0.0.1` | Cache Redis host |
| `REDIS_CACHE_PORT` | No | `6380` | Cache Redis port |
| `SEAWEEDFS_ENDPOINT` | Yes | — | SeaweedFS filer endpoint |
| `R2_ENDPOINT` | Yes | — | R2 endpoint |
| `R2_REGION` | No | `auto` | R2 region |
| `R2_ACCESS_KEY_ID` | Yes | — | R2 access key |
| `R2_SECRET_ACCESS_KEY` | Yes | — | R2 secret key |
| `R2_BUCKET_NAME` | Yes | — | R2 bucket name |
| `R2_PUBLIC_DOMAIN` | Yes | — | Public URL for R2 assets |
| `CRON_TIMEZONE` | No | `UTC` | Timezone for cron schedules |
