# Environment Variables

Complete list of environment variables used by AxumKit.

## Server (`axumkit-server`)

### Required

| Variable | Description |
|----------|-------------|
| `HOST` | Server bind host |
| `PORT` | Server bind port |
| `TOTP_SECRET` | Secret for TOTP backup code hashing |
| `AUTH_SESSION_MAX_LIFETIME_HOURS` | Session absolute max lifetime |
| `AUTH_SESSION_SLIDING_TTL_HOURS` | Sliding session TTL |
| `AUTH_SESSION_REFRESH_THRESHOLD` | Session refresh threshold (%) |
| `GOOGLE_CLIENT_ID` | Google OAuth client ID |
| `GOOGLE_CLIENT_SECRET` | Google OAuth client secret |
| `GOOGLE_REDIRECT_URI` | Google OAuth redirect URI |
| `GITHUB_CLIENT_ID` | GitHub OAuth client ID |
| `GITHUB_CLIENT_SECRET` | GitHub OAuth client secret |
| `GITHUB_REDIRECT_URI` | GitHub OAuth redirect URI |
| `R2_ENDPOINT` | R2-compatible S3 endpoint |
| `R2_REGION` | R2 region |
| `R2_ACCESS_KEY_ID` | R2 access key |
| `R2_SECRET_ACCESS_KEY` | R2 secret key |
| `R2_ASSETS_PUBLIC_DOMAIN` | Public domain for assets URLs |
| `R2_ASSETS_BUCKET_NAME` | Assets bucket name |
| `TURNSTILE_SECRET_KEY` | Cloudflare Turnstile secret |
| `POSTGRES_WRITE_HOST` | Write DB host |
| `POSTGRES_WRITE_PORT` | Write DB port |
| `POSTGRES_WRITE_NAME` | Write DB name |
| `POSTGRES_WRITE_USER` | Write DB user |
| `POSTGRES_WRITE_PASSWORD` | Write DB password |
| `POSTGRES_READ_HOST` | Read DB host |
| `POSTGRES_READ_PORT` | Read DB port |
| `POSTGRES_READ_NAME` | Read DB name |
| `POSTGRES_READ_USER` | Read DB user |
| `POSTGRES_READ_PASSWORD` | Read DB password |
| `POSTGRES_WRITE_MAX_CONNECTION` | Write DB max pool size |
| `POSTGRES_WRITE_MIN_CONNECTION` | Write DB min pool size |
| `POSTGRES_READ_MAX_CONNECTION` | Read DB max pool size |
| `POSTGRES_READ_MIN_CONNECTION` | Read DB min pool size |

### Optional

| Variable | Default | Description |
|----------|---------|-------------|
| `ENVIRONMENT` | prod | Set `dev`/`development` for dev mode |
| `AUTH_EMAIL_VERIFICATION_TOKEN_EXPIRE_TIME` | 1 | Minutes |
| `AUTH_PASSWORD_RESET_TOKEN_EXPIRE_TIME` | 15 | Minutes |
| `AUTH_EMAIL_CHANGE_TOKEN_EXPIRE_TIME` | 15 | Minutes |
| `OAUTH_PENDING_SIGNUP_TTL_MINUTES` | 10 | Minutes |
| `REDIS_SESSION_HOST` | `redis-session` | Session Redis host |
| `REDIS_SESSION_PORT` | `6379` | Session Redis port |
| `REDIS_CACHE_HOST` | `redis-cache` | Cache Redis host |
| `REDIS_CACHE_PORT` | `6379` | Cache Redis port |
| `REDIS_CACHE_TTL` | 3600 | Cache TTL seconds |
| `NATS_URL` | `nats://localhost:4222` | NATS URL |
| `MEILISEARCH_HOST` | `http://localhost:7700` | MeiliSearch URL |
| `MEILISEARCH_API_KEY` | empty | MeiliSearch API key |
| `CORS_ALLOWED_ORIGINS` | empty | Comma-separated origins |
| `CORS_ALLOWED_HEADERS` | empty | Comma-separated headers |
| `CORS_MAX_AGE` | empty | CORS max-age seconds |
| `COOKIE_DOMAIN` | empty | Cookie domain |
| `STABILITY_CONCURRENCY_LIMIT` | 500 | Stability concurrency limit |
| `STABILITY_BUFFER_SIZE` | 1024 | Stability queue size |
| `STABILITY_TIMEOUT_SECS` | 30 | Stability timeout seconds |

## Worker (`axumkit-worker`)

### Required

| Variable | Description |
|----------|-------------|
| `SMTP_HOST` | SMTP server host |
| `SMTP_USER` | SMTP username |
| `SMTP_PASSWORD` | SMTP password |
| `EMAILS_FROM_EMAIL` | Sender email |
| `FRONTEND_HOST` | Frontend base URL |
| `PROJECT_NAME` | Project name |
| `FRONTEND_PATH_VERIFY_EMAIL` | Verify-email path |
| `FRONTEND_PATH_RESET_PASSWORD` | Reset-password path |
| `FRONTEND_PATH_CONFIRM_EMAIL_CHANGE` | Confirm-email-change path |
| `POSTGRES_WRITE_HOST` | Write DB host |
| `POSTGRES_WRITE_PORT` | Write DB port |
| `POSTGRES_WRITE_NAME` | Write DB name |
| `POSTGRES_WRITE_USER` | Write DB user |
| `POSTGRES_WRITE_PASSWORD` | Write DB password |
| `R2_ENDPOINT` | R2-compatible S3 endpoint |
| `R2_ACCESS_KEY_ID` | R2 access key |
| `R2_SECRET_ACCESS_KEY` | R2 secret key |
| `R2_ASSETS_BUCKET_NAME` | Assets bucket name |
| `R2_ASSETS_PUBLIC_DOMAIN` | Public domain for assets URLs |

### Optional

| Variable | Default | Description |
|----------|---------|-------------|
| `SMTP_PORT` | 587 | SMTP port |
| `SMTP_TLS` | true | Enable TLS |
| `EMAILS_FROM_NAME` | `SevenWiki` | Sender display name |
| `MEILISEARCH_HOST` | `http://localhost:7700` | MeiliSearch URL |
| `MEILISEARCH_API_KEY` | empty | MeiliSearch API key |
| `NATS_URL` | `nats://localhost:4222` | NATS URL |
| `REDIS_CACHE_HOST` | `127.0.0.1` | Cache Redis host |
| `REDIS_CACHE_PORT` | `6380` | Cache Redis port |
| `POSTGRES_WRITE_MAX_CONNECTION` | 10 | DB max pool size |
| `POSTGRES_WRITE_MIN_CONNECTION` | 2 | DB min pool size |
| `R2_REGION` | `auto` | R2 region |
| `CRON_TIMEZONE` | `UTC` | Cron timezone |
