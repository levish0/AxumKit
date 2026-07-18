---
title: Architecture
description: Workspace layout and how the pieces talk to each other.
order: 3
---

AxumKit is a Cargo workspace with two binaries and a set of shared contract crates.

## The two binaries

- **`server`** — the API. Layered as routes → services → repositories → entities, with
  cross-cutting modules for permissions, middleware, extractors, and outbound clients.
- **`worker`** — background jobs. NATS JetStream consumers plus a cron scheduler.

They never link each other. Everything that crosses the boundary lives in dedicated
contract crates, so the compiler catches drift:

| Crate | Contract |
| --- | --- |
| `job_queue` | Job payloads, stream/subject/consumer names, idempotent stream creation. Both binaries call `initialize_all_streams` at startup, so a fresh NATS works regardless of boot order. |
| `notification_repository` | How a notification event + its per-recipient deliveries are written, and preference filtering. |
| `search_index` | Meilisearch index uids and document schemas. The worker serializes `SearchUser` in; the server deserializes the same struct out. |
| `entity` / `migration` | SeaORM entities and the schema itself. |
| `dto` | Request/response types + validators. |
| `errors` | The `Errors` enum, per-domain handler chain, and `protocol.rs` wire codes. |
| `config` | `ServerConfig` / `WorkerConfig` (env-driven, missing vars reported all at once). |
| `constants` | Cache keys and the codename enums (`Permission`, `NotificationAction`, `ModerationAction`). |
| `auth-core` | Project-agnostic crypto primitives (AEAD, constant-time compare, keyed hashes, tokens). |
| `storage` | R2/S3 clients with per-operation timeouts. |

## Server internals

```
api/            route handlers (+ utoipa annotations, per-domain openapi.rs)
service/        business logic; owns transactions
repository/     queries; find_* → Option, get_* → Result, one function per file
permission/     UserContext, has_perm/require_perm, per-domain Rule objects
middleware/     anonymous user, CORS, require_role gates, stability layer
extractors/     session resolution (cookie or Bearer), Turnstile verification
bridge/         outbound clients: job publisher, media processor, Turnstile
eventstream/    SSE fan-out over core NATS (multi-replica safe)
connection/     one file per external dependency
```

Conventions worth knowing before writing code:

- Handlers never touch the database directly; services own transactions and call
  repositories.
- Rows with an expiry (roles, bans, group memberships) are **filtered at read time**;
  a cron job reclaims dead rows later.
- Best-effort side effects (indexing, notifications, cache invalidation) are enqueued
  post-commit via `tokio::spawn` — a queue outage must not fail the user's request.
- Codename-style values (`board:pin_post`, `user_mentioned`) are Rust enums in
  `constants` stored as TEXT in Postgres: adding a variant needs no migration, and a
  stored string that no longer parses fails closed.

## Error handling

Services and handlers return `Errors`; the `IntoResponse` impl maps variants through a
per-domain handler chain to `(status, "domain:code", details)`. Client errors (4xx)
always include details; server errors (5xx) hide them outside development. The string
codes in `errors/src/protocol.rs` are a stable wire contract for frontends.
