---
title: Configuration
description: Environment variables and the env file layout.
order: 4
---

Configuration is environment-driven. `ServerConfig` and `WorkerConfig` load once (via
`LazyLock`) and validate eagerly — every missing or unparsable variable is collected
and reported in a single panic message instead of one at a time.

## Env file layout

- **`.env`** (repo root, gitignored; template: `.env.example`) — used when running the
  binaries natively (`cargo run -p server`).
- **`.envs/`** — concern-grouped env trees for Docker Compose:
  - `.example/` — committed templates (`postgres.env`, `server.env`, `worker.env`, `r2.env`, …)
  - `.local/` — gitignored dev values
  - `.test/` — committed values for the disposable test stack (no real secrets)
  - `.production/` — gitignored production values

## Database

The app standardizes on a single **`DATABASE_URL`** — a full connection URL read by
the server, the worker, and the migration binary alike:

```
DATABASE_URL=postgres://axumkit:secret@localhost:6432/axumkit
```

It is deliberately a URL rather than assembled host/port/user parts so the deployment
controls the query string — `?sslmode=require` or `channel_binding=require` for
managed providers like Neon, or nothing for a local Postgres. Point it at `pgdog:6432`
to go through the bundled connection pooler. Logs never print the URL verbatim;
`redact_database_url` strips credentials first.

The `POSTGRES_DB` / `POSTGRES_USER` / `POSTGRES_PASSWORD` entries in
`.envs/*/postgres.env` are consumed by the Postgres **container's** initdb and the
compose healthcheck — not by the app.

## Variable reference (high level)

| Group | Variables |
| --- | --- |
| Server bind | `HOST`, `PORT`, `ENVIRONMENT` (`dev` enables Swagger + relaxed cookies) |
| Database | `DATABASE_URL`, `POSTGRES_MAX_CONNECTION`, `POSTGRES_MIN_CONNECTION` |
| Redis | `REDIS_SESSION_HOST/PORT` (noeviction + AOF), `REDIS_CACHE_HOST/PORT` (LRU); worker adds `REDIS_LOCK_HOST/PORT` (noeviction) |
| Auth secrets | `TOTP_SECRET` (backup-code hashing), `TOTP_ENCRYPTION_KEY` (AES-GCM key derivation) |
| Session tuning | `AUTH_SESSION_MAX_LIFETIME_HOURS`, `AUTH_SESSION_SLIDING_TTL_HOURS`, `AUTH_SESSION_REFRESH_THRESHOLD` |
| Token expiries | email verification, password reset, email change, account deletion, device verification (minutes each) |
| OAuth | `GOOGLE_CLIENT_ID/SECRET/REDIRECT_URI`, `GITHUB_CLIENT_ID/SECRET/REDIRECT_URI` |
| Infrastructure | `NATS_URL`, `MEILISEARCH_HOST` (+ optional `MEILISEARCH_API_KEY`), `MEDIA_PROCESSOR_URL` |
| Storage | `R2_ENDPOINT`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, `R2_ASSETS_BUCKET_NAME`, `R2_ASSETS_PUBLIC_DOMAIN` |
| Edge | `CORS_ALLOWED_ORIGINS` (**production panics when unset**), `CORS_ALLOWED_HEADERS`, `COOKIE_DOMAIN`, `TURNSTILE_SECRET_KEY`, `INTERNAL_PROXY_SECRET` |
| Worker email | `SMTP_HOST/PORT/USER/PASSWORD/TLS`, `EMAILS_FROM_*`, `FRONTEND_HOST` + per-flow link paths |

See `.env.example` and `.envs/.example/` for the complete, commented list.

## Security-relevant defaults

- Production cookies use the `__Host-` prefix (or `__Secure-` when `COOKIE_DOMAIN` is
  set), `HttpOnly`, `Secure`, `SameSite=Lax`.
- The backend trusts only `CF-Connecting-IP`, or `X-Real-Client-IP` when accompanied
  by a constant-time-verified `X-Internal-Secret` — never `X-Forwarded-For`.
- CORS mirrors request headers instead of wildcarding, so credentialed requests stay
  valid; unset origins are a startup error in production rather than an open default.
