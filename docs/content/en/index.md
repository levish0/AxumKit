---
title: Introduction
description: What AxumKit is and what it ships with.
order: 1
---

AxumKit is a production-ready Rust web API template built on **Axum**, **SeaORM**,
**PostgreSQL**, **Redis**, **NATS JetStream**, and **Meilisearch**. It is meant to be
cloned and grown into a real service: the plumbing every serious backend needs —
authentication, authorization, background jobs, email, search, testing, deployment —
is already in place and wired together.

## What's included

- **Session-based authentication** — opaque bearer tokens hashed at rest in Redis,
  sliding TTL with an absolute lifetime cap, session listing/revocation, TOTP 2FA with
  encrypted secrets and single-use backup codes, new-device email verification, and
  OAuth2 (Google, GitHub, Google One Tap) with PKCE and state binding.
- **Django-style RBAC** — coarse roles (`Mod`, `Admin`), fine-grained permission
  codenames (`board:pin_post`, …), and admin-managed ACL groups that bundle
  permissions for their members.
- **A board domain as the demo feature** — boards, posts, comments with a reply-depth
  cap, pinned posts with reorder, post locking, moderation, buffered view counts, and
  @handle mentions.
- **In-app notifications** — a per-user inbox fed by comment alerts and mentions, with
  per-action opt-out preferences.
- **A background worker** — NATS JetStream consumers for email (MJML templates),
  Meilisearch indexing, and OAuth avatar processing, plus cron jobs for cleanups and
  view-count flushing. Retries, per-message dedup, a dead-letter queue, and graceful
  shutdown are built into the consumer engine.
- **Operational tooling** — a disposable Docker test stack with black-box e2e suites,
  an OpenAPI drift gate in CI, compose trees for dev/test/production, and Cloudflare
  R2 storage with content-addressed image handling.

## Where to go next

Start with [Getting started](/docs/getting-started), then skim
[Architecture](/docs/architecture) to learn the workspace layout before changing code.
