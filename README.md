<p align="center">
  <img src="assets/axumkit_banner.png" alt="AxumKit" />
</p>

# AxumKit
Production-ready Rust web backend template.

## Features

- **Auth**:Session (Redis), email/password (Argon2), OAuth2 (Google, GitHub), TOTP 2FA
- **Users & Posts**:Profiles, image uploads (R2), CRUD with ownership
- **Search**:Full-text via MeiliSearch, auto-indexed by worker
- **Background Jobs**:NATS JetStream worker (email, indexing, cleanup, cron)
- **Email**:SMTP templates (Lettre + MRML + MiniJinja)
- **Rate Limiting**:Sliding window (Redis Lua), per-route
- **API Docs**:Auto-generated Swagger UI (debug builds)
- **Deploy**:Docker multi-stage, Helm charts, GitHub Actions CI/CD

## Quick Start

```bash
git clone https://github.com/levish0/AxumKit.git && cd AxumKit
cp .env.example .env  # edit with your config

cd crates/migration && cargo run && cd ../..  # migrations
cargo run -p axumkit-server                   # API server
cargo run -p axumkit-worker                   # worker (separate terminal)
```

## Project Structure

```
crates/
├── axumkit-server     # API (handlers → services → repositories → entities)
├── axumkit-worker     # Background jobs (NATS consumers, cron)
├── axumkit-config     # Env config
├── axumkit-constants  # Shared constants
├── axumkit-dto        # Request / response types
├── axumkit-entity     # SeaORM models
├── axumkit-errors     # Centralized error handling
├── migration          # DB migrations
└── e2e                # E2E tests
```

## Configuration

Env vars from `.env`, validated at startup. See [`.env.example`](.env.example) for the full list.

## License

[MIT](LICENSE)
