---
title: Deployment
description: Compose topologies, images, and backups.
order: 10
---

Deployment assets live in `deploy/` and `compose/`, split so infrastructure and
application lifecycles are independent:

- **`deploy/docker-compose.infra.yml`** — Postgres (with a pgBackRest-enabled image),
  the PgDog connection pooler, three Redis instances (session / cache / lock), NATS,
  and Meilisearch. Postgres binds to `127.0.0.1:${POSTGRES_HOST_PORT}` only.
- **`deploy/docker-compose.app.yml`** — the one-shot migration job, the server, the
  worker, and the media-processor sidecar, pulled from GHCR.
- **`deploy/docker-compose.logging.yml`** — optional Loki + Grafana + Alloy log
  pipeline.

Environment selection works via `deploy/<env>.env` plus the gitignored
`.envs/.production/` tree; `COMPOSE_PROJECT_NAME` isolates multiple environments on
one host. A Neon variant (`deploy/neon/`) drops local Postgres entirely — the
migration service gets the direct endpoint, the app goes through the pooler URL.

```bash
cd deploy
just up production          # infra + app
just up-logging production  # + log pipeline
just logs production server
```

## Images

`cargo xtask docker-publish --tag <version> [--latest]` builds and pushes the server
and worker images to GHCR; the `docker.yml` workflow does the same on `v*` tags.
`Dockerfile` is the release build, `Dockerfile.dev` the fast debug build used by the
test stack.

## Backups

pgBackRest is wired into the Postgres image and driven from `deploy/justfile`:
`pgbackrest-init`, `-backup [full|diff|incr]`, `-info`, `-restore` (delta restore with
PITR), and `-cron-install` for a weekly-full/daily-diff host crontab. Repositories
target R2 via its S3 API.

## Edge assumptions

The template expects to sit behind an edge that terminates TLS and forwards
`CF-Connecting-IP` (Cloudflare) — or a trusted proxy sending `X-Real-Client-IP` with
the shared `X-Internal-Secret`. Rate limiting can ride the built-in middleware or be
delegated to a gateway (an APISIX compose file and a `/v0/auth/check` forward-auth
endpoint are included for rate-limit keying by identity).
