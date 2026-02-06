---
layout: home

hero:
  name: "AxumKit"
  text: "Production-ready Rust web backend template"
  tagline: Built on Axum, SeaORM, PostgreSQL, Redis, NATS, MeiliSearch, and more.
  actions:
    - theme: brand
      text: Get Started
      link: /guide/getting-started
    - theme: alt
      text: View on GitHub
      link: https://github.com/levish0/AxumKit

features:
  - title: Session-based Auth
    details: Secure session management with Redis, email verification, password reset, and OAuth2 (Google, GitHub).
  - title: TOTP 2FA
    details: Time-based one-time password support with QR code setup and backup codes.
  - title: Background Workers
    details: NATS JetStream job queue with email delivery, search indexing, storage cleanup, and cron jobs.
  - title: Full-text Search
    details: MeiliSearch integration with automatic indexing via worker for posts and users.
  - title: Rate Limiting
    details: Redis Lua sliding window rate limiter with per-route configuration.
  - title: Multi-stage Docker
    details: Production-ready Dockerfile with cargo-chef caching, separate server and worker targets.
---
