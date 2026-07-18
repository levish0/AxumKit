<p align="center">
  <img src="assets/axumkit_banner.png" alt="AxumKit" />
</p>

# AxumKit

Production-ready Rust web backend template — Axum, SeaORM, PostgreSQL, Redis,
NATS JetStream, Meilisearch.

## Features

- **Auth** — Redis sessions (hashed at rest, sliding TTL, listing/revocation),
  email/password (Argon2id, enumeration-safe), OAuth2 with PKCE (Google, GitHub,
  Google One Tap), TOTP 2FA with encrypted secrets and single-use backup codes,
  new-device email verification, native-app (`/app/auth`) mirrors, auth audit log
- **RBAC** — Django-style roles + groups + permission codenames
  (`user.has_perm` semantics) with an admin API; ban hard gate, admin anti-lockout
- **Boards (demo domain)** — posts, comments (depth-capped replies), pins with
  stale-set-safe reorder, locking, moderation, buffered view counts, @handle mentions
- **Notifications** — per-user inbox (comment alerts, mentions) with per-action opt-outs
- **Search** — Meilisearch user index, auto-indexed by the worker, drift-proof via a
  shared schema crate
- **Background jobs** — NATS JetStream consumers (email via MJML templates, indexing,
  avatar processing) with retries, dedup, a dead-letter queue, and cron cleanups
- **Storage** — Cloudflare R2 with content-addressed WebP image processing
- **Testing** — 59 black-box e2e tests against a disposable Docker stack
  (tmpfs Postgres, Mailpit, S3 stand-in), pinned `sec_NNN` security regressions
- **Ops** — OpenAPI drift gate in CI, Swagger UI (debug builds), compose trees for
  dev/test/production, pgBackRest backups, optional Loki/Grafana logging

## Quick Start

```bash
git clone https://github.com/levish0/AxumKit.git && cd AxumKit
cp .env.example .env  # edit with your config

just dev              # infra containers + migrations
cargo run -p server   # API server
cargo run -p worker   # worker (separate terminal)
```

Swagger UI at `http://localhost:8000/docs` (debug builds).
Full guides live in the [documentation site](docs/).

## Project Structure

```
crates/
├── server                    # API (handlers → services → repositories → entities)
├── worker                    # Background jobs (NATS consumers, cron)
├── job_queue                 # Shared server↔worker queue contract
├── notification_repository   # Shared notification persistence
├── search_index              # Shared Meilisearch schema contract
├── entity / migration        # SeaORM models and schema
├── dto                       # Request / response types + validators
├── errors                    # Centralized errors + wire protocol codes
├── config / constants        # Env config, cache keys, permission codenames
├── auth-core                 # Project-agnostic crypto primitives
├── storage                   # R2/S3 clients
└── e2e                       # Black-box end-to-end tests
```

## Configuration

A single `DATABASE_URL` drives the server, worker, and migrations. Host
`cargo run` loads `.env`; Docker Compose loads concern-grouped files in
[`.envs`](.envs/README.md) (copy `.envs/.example` to `.envs/.local`).

## Testing

```bash
just check   # fmt, clippy -D warnings, tests, OpenAPI drift
just e2e     # disposable Docker stack + the full e2e suite
```

## License

[MIT](LICENSE)
