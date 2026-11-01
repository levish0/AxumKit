# AxumKit has moved

**Continue at [Hirune Axum Kit](https://github.com/HiruneWorks/hirune-axum-kit).**

This repository is retired. Use the new repository for current source,
documentation, issues, and releases.

The source below is a one-time snapshot of Hirune Axum Kit
[`31ccac9`](https://github.com/HiruneWorks/hirune-axum-kit/commit/31ccac9c35c2f8a89e910721435a1e7ec66fdebb),
copied on 2026-11-02. This repository will receive no further updates.
The previous implementation remains on the
[`legacy-axumkit` branch](https://github.com/levish0/AxumKit/tree/legacy-axumkit)
and in existing tags. Hirune Axum Kit is a new implementation. Legacy
applications need separate API, configuration, and database migration work.

---

# Hirune Axum Kit

A Rust source template for a maintainable Axum service. It uses SeaORM,
PostgreSQL, Tokio, structured tracing, Cedar authorization, and MJML email.
API and worker processes scale independently. PostgreSQL stores their shared state.
Production ingress uses Cloudflare Tunnel and a private APISIX gateway. SSR
servers can preserve the original client IP with a shared proxy secret.
The [k3s profile](deploy/k3s/README.md) includes two API replicas, private APISIX,
shared Valkey edge counters, Tunnel replicas, migration sequencing, and live
rolling-update verification.
For a single Linux host, the optional [data profile](deploy/data-services/README.md) runs
Docker Compose PostgreSQL/Meilisearch and pgBackRest backups beside k3s.
Application `v*` tags publish the tested release image to the repository's GHCR
package. PostgreSQL uses independent `postgres-v*` tags. See the
[publication and image selection instructions](deploy/README.md#environments-and-commands).

## Start locally

Install Rust 1.98.1, just, and Docker with Compose. Then run:

```sh
just init
just infra-up
just migrate
just api
```

Run `just worker` in another terminal. The API listens on localhost:3000.
Mailpit listens on localhost:8029. The API serves OpenAPI at `/openapi.json`.
`just init` creates independent local keys and never overwrites an existing `.envs/.local/app.env`.
This template has no frontend. Your frontend must read the token from an email
link's fragment and submit it to the documented confirmation endpoint.

## Verify

```sh
just check
just test-db
just test-api
just test-process
just test-files
just test-content
just test-deployment
just test-production
just load-test
just template-check
```

Database and process checks use temporary Compose projects with unique names and
separate test databases. See [development](docs/development.md) for external
local infrastructure, test prerequisites, and load-test profiles. CI uses the same
commands. Check the remote result before reporting that CI passed.
The production check builds a release image and uses local HTTPS, STARTTLS, and
database restoration fixtures. It does not deploy the application.

## Optional file storage and search

Run `just infra-up-files`, enable the commented S3/Meilisearch settings in
`.envs/.local/app.env`, and run `just manage initialize-integrations` after
migrations. Clients transfer original bytes directly to the private S3 bucket through short-lived presigned URLs. The API authorizes issuance and verifies upload completion. PostgreSQL
controls attachment access. Separate Meilisearch projections support filename,
published-post, and organization-user search. Run the worker
for deletion and indexing, and run `maintenance` for periodic cleanup and search
repair in deployment. See the
[file contract](docs/behavior/files-and-search.md) for limits and lifecycle.

## Connected example

Organization posts and single-level comments demonstrate Cedar permissions,
optimistic updates, private attachments, profiles, and scoped pagination/search.
A reply and the post author's in-app notification commit in the same transaction.
Recipients can manage preferences and read state. Optional FCM delivery uses the
durable worker queue. It is disabled by default. See the
[content](docs/behavior/posts-and-comments.md),
[search](docs/behavior/profiles-and-search.md), and
[notification](docs/behavior/notifications-and-push.md) contracts.

HTTP adapters are organized by version, domain, and named operation under
`src/http/v1/`. Each domain's `routes.rs` is the route map; operation-specific
DTOs live beside their handler, and shared DTOs live in `contracts.rs`.

## External developer API

Organization owners can register clients and issue scoped, expiring API keys.
New clients have zero allocation; an operator assigns minute and calendar-month
budgets with `just manage set-api-client-quota <uuid> <per-minute> <per-month>`.
All keys and API replicas share the same PostgreSQL authority. See the
[API access contract](docs/behavior/api-access.md) for routes, credential format,
rotation, request accounting, and extension boundaries.

## Own the source

Copy the repository and change the root Cargo package name, README/NOTICE,
Compose project name, and application settings for your project. Keep the library
name `kit` unless you also change Rust imports. Regenerate Cargo.lock and OpenAPI,
then run the checks. Internal crates organize the source code. They are not
separate products. Template updates do not automatically update your copy.

- [Architecture](docs/architecture.md) and [runtime](docs/runtime.md)
- [Environment layout](.envs/README.md), [container files](compose/README.md), and [deployment](deploy/README.md)
- [Confirmed requirements](docs/requirements.md) and [code conventions](docs/conventions.md)
- [API conventions](docs/api.md) and [generated OpenAPI](docs/openapi.json)
- [Behavior contracts](docs/behavior/README.md) and [transaction boundaries](docs/transaction-boundaries.md)
- [Security controls](docs/security.md) and [regression inventory](docs/regressions.md)
- [Agent instructions](AGENTS.md) and [milestone evidence](memory/README.md)

Copyright 2026 Levi Laine. Licensed under [Apache-2.0](LICENSE).
