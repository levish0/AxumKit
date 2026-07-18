---
title: Getting started
description: From clone to a running API in a few minutes.
order: 2
---

## Prerequisites

- Rust (stable, edition 2024)
- Docker with Compose
- [`just`](https://github.com/casey/just)

## First run

```bash
git clone https://github.com/levish0/AxumKit
cd AxumKit
cp .env.example .env          # native dev env (server/worker read this)
cp -r .envs/.example .envs/.local   # compose env tree, fill real values

just dev                      # start infra containers + apply migrations
cargo run -p server           # API on http://localhost:8000
cargo run -p worker           # background worker (separate terminal)
```

`just dev` brings up only the infrastructure set — Postgres (behind the PgDog pooler),
three Redis instances (session / cache / lock), NATS with JetStream, and Meilisearch —
so the two binaries run natively for fast iteration.

In debug builds, Swagger UI is served at `/docs` and the OpenAPI spec at
`/swagger.json`.

## Everyday commands

```bash
just check          # everything CI checks: fmt, clippy -D warnings, tests, OpenAPI drift
just test [filter]  # workspace tests, excluding e2e
just e2e            # full docker stack + black-box e2e suite (always tears down)
just openapi        # regenerate swagger.json — run after any route change
just migrate-fresh  # drop and reapply all migrations
```

## First admin

The application deliberately has no first-admin bootstrap path — roles are only ever
granted by an existing admin. For local development, insert the role directly:

```sql
INSERT INTO user_roles (user_id, role) VALUES ('<your-user-uuid>', 'admin');
```

The server reads roles per request, so the grant takes effect immediately.
