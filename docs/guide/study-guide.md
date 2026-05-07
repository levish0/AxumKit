# Study Guide

This guide gives a practical reading order for the current AxumKit codebase.

## 1) Start with Configuration

Read:

- `crates/config/src/server_config.rs`
- `crates/worker/src/config/worker_config.rs`

Focus on required env vars and defaults.

## 2) Understand Shared Types

Read:

- `crates/entity/src/users.rs`
- `crates/entity/src/user_oauth_connections.rs`
- `crates/dto/src/auth/*`
- `crates/dto/src/oauth/*`
- `crates/dto/src/search/*`

## 3) Learn the Error System

Read:

- `crates/errors/src/errors.rs`
- `crates/errors/src/protocol.rs`
- `crates/errors/src/handlers/*`

Goal: understand how domain errors map to HTTP responses.

## 4) Server Runtime Entry

Read:

- `crates/server/src/main.rs`
- `crates/server/src/state.rs`
- `crates/server/src/connection/*`

Goal: how DB, Redis, NATS, R2, and MeiliSearch are wired.

## 5) Middleware and Extractors

Read:

- `crates/server/src/middleware/*`
- `crates/server/src/extractors/*`

Goal: request lifecycle, session extraction, and rate-limit flow.

## 6) Route -> Service -> Repository Flow

Read in this order:

- `crates/server/src/api/v0/routes/auth/*`
- `crates/server/src/service/auth/*`
- `crates/server/src/repository/user/*`
- `crates/server/src/repository/oauth/*`

Then check user/search routes:

- `crates/server/src/api/v0/routes/user/*`
- `crates/server/src/api/v0/routes/search/*`
- `crates/server/src/service/search/*`

## 7) Worker Pipeline

Read:

- `crates/worker/src/main.rs`
- `crates/worker/src/nats/streams.rs`
- `crates/worker/src/nats/consumer.rs`
- `crates/worker/src/jobs/email/*`
- `crates/worker/src/jobs/index/*`
- `crates/worker/src/jobs/reindex/*`
- `crates/worker/src/jobs/cron/*`

Goal: understand asynchronous job execution and cron scheduling.

## Suggested Exercises

1. Add a new field to user profile and expose it in API.
2. Add a new worker job subject and consumer.
3. Add an integration or API-level test that verifies a full auth flow.
