---
title: Testing
description: Unit tests, the e2e harness, and CI gates.
order: 9
---

## Unit tests

Inline `#[cfg(test)]` modules, heaviest around the things that must not regress: the
permission engine (`has_perm` resolution, board policy rules), crypto primitives in
`auth-core`, DTO validators, and the notification target encoding. Run with
`just test` (workspace, e2e excluded).

## e2e: black-box by design

`crates/e2e` never imports the server. Tests drive a fully running stack over HTTP,
exactly like a real client, against `docker-compose.test.yml`:

- **tmpfs Postgres** on host port 55432 (fast, wiped, never shadowed by a native
  Postgres on 5432)
- **SeaweedFS** as the S3-compatible stand-in for R2
- **Mailpit** capturing SMTP (its REST API is how tests read emailed tokens)
- **a Turnstile stub** answering success to everything
- the real server, worker, and migration containers built from `Dockerfile.dev`

Run it with `just e2e` — it brings the stack up behind healthchecks, runs the suite
with capped parallelism, always tears down, and preserves the suite's exit status.

### The harness

`TestClient` is a cookie-jar HTTP client; each instance is an independent browser.
`with_ip()` simulates distinct client IPs. `signup_and_login()` registers a unique
user and completes email verification via Mailpit polling. Two deliberate
direct-to-database helpers exist because the app has no first-admin path:
`grant_role(handle, Role)` and `backdate_user(handle, days)`.

### Suites

`auth`, `totp`, `account`, `moderation`, `rbac`, `board`, `notification`, `search`,
`user_public`, and `smoke`. Security regressions are pinned with `sec_NNN` names
(e.g. concurrent single-use backup codes, notification IDOR probes) so they are
recognizable and never quietly dropped.

## CI

| Workflow | Gate |
| --- | --- |
| `check.yml` | `fmt --check`, `clippy --all-targets -D warnings`, **OpenAPI drift** (`cargo xtask openapi` + `git diff --exit-code swagger.json`), plus a job applying all migrations to a fresh Postgres |
| `test.yml` | workspace unit/integration tests (runs in parallel with check) |
| `e2e.yml` | the full docker-stack e2e suite, dumping container logs on failure |
| `build.yml` / `docker.yml` | build check and GHCR image publishing on version tags |

The OpenAPI drift gate means `swagger.json` is always in sync with the code — run
`just openapi` after changing any route.
