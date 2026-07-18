# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

AxumKit is a production-ready Rust web API template built on Axum, SeaORM, PostgreSQL, Redis, NATS
JetStream, and Meilisearch. It ships session-based auth (Redis sessions, TOTP 2FA, new-device
verification, OAuth2 Google/GitHub incl. One Tap), Django-style RBAC (roles + ACL groups +
permission grants), a board domain (posts/comments/pins/locks) as the demo feature, an in-app
notification inbox, a background worker (email, search indexing, cron), and Cloudflare R2 storage.

## Workspace Layout

Cargo workspace under `crates/`:

- `server` — API binary: `api/` (routes), `service/`, `repository/`, `permission/`, `middleware/`,
  `extractors/`, `bridge/` (outbound clients: worker queue, media processor, Turnstile),
  `connection/`, `eventstream/` (SSE), `utils/`, `state.rs`
- `worker` — background binary: NATS JetStream consumers (`jobs/`), cron scheduler, email
  templates (MJML), Meilisearch indexing
- `job_queue` — shared server↔worker contract: job payloads, stream/subject/consumer names,
  idempotent stream creation (both binaries call `initialize_all_streams` at startup)
- `notification_repository` — shared notification persistence (event + delivery insertion,
  preference filtering) used by server and worker
- `search_index` — shared Meilisearch index contract (index uids + document schemas) between the
  server (reader) and worker (writer)
- `entity` / `migration` — SeaORM entities and migrations (fresh-DB model; `cd` not needed:
  `cargo run -p migration -- <cmd>`)
- `dto` — request/response/internal DTOs per domain + validators
- `errors` — workspace-wide `Errors` enum, per-domain handlers, `protocol.rs` wire codes
  (`"domain:snake_case"`); 4xx details always shown, 5xx details hidden in production
- `config` — `ServerConfig` / `WorkerConfig` (LazyLock; missing env vars are collected and
  reported all at once), `redact_database_url`
- `auth-core` — project-agnostic crypto primitives (AEAD, constant-time compare, keyed hash,
  tokens); app adapters live in `server/src/utils/crypto/`
- `constants` — cache keys, `Permission` / `NotificationAction` / `ModerationAction` codenames
  (stored as TEXT in DB, parsed with fail-closed semantics)
- `storage` — R2/S3 clients with operation timeouts
- `e2e` — black-box HTTP test harness + suites (runs against `docker-compose.test.yml`)

## Common Commands

```bash
just dev                 # infra containers + migrations (then `cargo run -p server` / `-p worker`)
cargo run -p server      # API server (localhost:8000)
cargo run -p worker      # background worker
cargo run -p migration -- fresh   # drop + reapply all migrations
just check               # fmt --check, clippy -D warnings, tests, OpenAPI drift gate
just test [filter]       # workspace tests excluding e2e
just e2e [threads]       # full docker test stack + e2e suite (teardown always runs)
just openapi             # regenerate swagger.json (CI fails on drift — run after route changes)
```

Swagger UI at `/docs`, spec at `/swagger.json` (debug builds only).

## Configuration

Single `DATABASE_URL` (full connection URL; the deployment controls the query string, e.g.
`?sslmode=require`) is read by server, worker, and migration. See `.env.example` (native dev) and
`.envs/` (compose env trees: `.example/` committed templates, `.local/` dev, `.test/` committed
test values). Required vars include `JWT_SECRET`-style secrets (`TOTP_SECRET`,
`TOTP_ENCRYPTION_KEY`), Redis session/cache hosts, NATS, Meilisearch, R2, OAuth (Google/GitHub),
Turnstile, and CORS origins (production panics when unset).

## Authorization Model (Django-style RBAC)

Three layers, evaluated in `server/src/permission/`:

1. **Roles** (`user_roles`, enum `{Mod, Admin}`, optional expiry): coarse capability axis.
   Router-level gates via `middleware/require_role.rs` (`require_admin`/`require_mod`).
2. **Permissions** (`constants::Permission`, codenames like `board:pin_post` stored as TEXT):
   fine-grained grants. `UserContext::has_perm` resolves ban gate → Admin bypass → Mod default
   set (`Permission::MOD_DEFAULTS`) → group-granted union. Denials return `acl:denied` with the
   missing codename.
3. **ACL groups** (`acl_groups` + `acl_group_members` + `acl_group_permissions`): admin-managed
   permission bundles with member expiry/reason metadata. Admin API under `/v0/acl/*`.

Domain policy objects (e.g. `permission/board.rs` `BoardPermission`) implement the `Rule` trait
and are the single source of truth per domain (owner-only edits, ban-exempt reads, etc.).

## Development Patterns

- **New endpoint**: handler in `api/v0/routes/{domain}/` with `#[utoipa::path]` → register in the
  domain `routes.rs` + `openapi.rs` → DTOs in `dto/{domain}/` → service → repository. Then run
  `just openapi`.
- **Repository naming**: `find_*` returns `Option<Model>`, `get_*` returns `Result<Model, Errors>`;
  one function per file; expiring rows (roles, bans, group members) are filtered at read time.
- **New background job**: payload + names in `job_queue`, handler + consumer in `worker/src/jobs/`,
  publish via `server/src/bridge/worker_client/` (post-commit `tokio::spawn` for best-effort jobs).
- **Auth in handlers**: extractors (`RequiredSession` / optional session), not route middleware;
  `SessionContext { user_id, session_id, management_id }`.
- **Errors**: return `Errors` variants; map them in `errors/src/handlers/{domain}_handler.rs` and
  add wire codes to `protocol.rs`.
- **Sessions**: opaque bearer tokens stored hashed in Redis (`session:{blake3}`), sliding TTL with
  absolute lifetime cap, `management_id` indirection for listing/revocation.
- Comments are English; never reference the codebase this template was derived from.

## Testing

- Unit tests inline (`#[cfg(test)]`); the permission engine and validators carry the largest suites.
- e2e (`crates/e2e`) is black-box HTTP only: `TestClient` (cookie jar, `with_ip` for per-actor IP),
  Mailpit polling for emailed tokens, `grant_role`/`backdate_user` direct-DB bootstrap (no
  first-admin path exists by design). Security regressions are pinned as `sec_NNN` tests.
- Test stack (`docker-compose.test.yml`): tmpfs Postgres on host port 55432, SeaweedFS as the R2
  stand-in, Mailpit SMTP, Turnstile stub; server on 18000, Mailpit REST on 18025.
