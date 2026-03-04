# Study Guide

This guide gives a practical reading order for the current AxumKit codebase.

## 1) Start with Configuration

Read:

- `crates/axumkit-config/src/server_config.rs`
- `crates/axumkit-worker/src/config/worker_config.rs`

Focus on required env vars and defaults.

## 2) Understand Shared Types

Read:

- `crates/axumkit-entity/src/users.rs`
- `crates/axumkit-entity/src/user_oauth_connections.rs`
- `crates/axumkit-dto/src/auth/*`
- `crates/axumkit-dto/src/oauth/*`
- `crates/axumkit-dto/src/search/*`

## 3) Learn the Error System

Read:

- `crates/axumkit-errors/src/errors.rs`
- `crates/axumkit-errors/src/protocol.rs`
- `crates/axumkit-errors/src/handlers/*`

Goal: understand how domain errors map to HTTP responses.

## 4) Server Runtime Entry

Read:

- `crates/axumkit-server/src/main.rs`
- `crates/axumkit-server/src/state.rs`
- `crates/axumkit-server/src/connection/*`

Goal: how DB, Redis, NATS, R2, and MeiliSearch are wired.

## 5) Middleware and Extractors

Read:

- `crates/axumkit-server/src/middleware/*`
- `crates/axumkit-server/src/extractors/*`

Goal: request lifecycle, session extraction, and rate-limit flow.

## 6) Route -> Service -> Repository Flow

Read in this order:

- `crates/axumkit-server/src/api/v0/routes/auth/*`
- `crates/axumkit-server/src/service/auth/*`
- `crates/axumkit-server/src/repository/user/*`
- `crates/axumkit-server/src/repository/oauth/*`

Then check user/search routes:

- `crates/axumkit-server/src/api/v0/routes/user/*`
- `crates/axumkit-server/src/api/v0/routes/search/*`
- `crates/axumkit-server/src/service/search/*`

## 7) Worker Pipeline

Read:

- `crates/axumkit-worker/src/main.rs`
- `crates/axumkit-worker/src/nats/streams.rs`
- `crates/axumkit-worker/src/nats/consumer.rs`
- `crates/axumkit-worker/src/jobs/email/*`
- `crates/axumkit-worker/src/jobs/index/*`
- `crates/axumkit-worker/src/jobs/reindex/*`
- `crates/axumkit-worker/src/jobs/cron/*`

Goal: understand asynchronous job execution and cron scheduling.

## 8) E2E Test Harness

Read:

- `crates/e2e/src/fixtures/*`
- `crates/e2e/src/helpers/*`

Goal: how docker-compose infra is started, migrated, and tested.

## Suggested Exercises

1. Add a new field to user profile and expose it in API.
2. Add a new worker job subject and consumer.
3. Add an e2e test that verifies a full auth flow.
