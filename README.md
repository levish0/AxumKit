# AxumKit

Production-ready Rust web API template.

## What's Included

- **Web Framework**: Axum with layered architecture (API → Service → Repository → Entity)
- **Database**: PostgreSQL + SeaORM with migrations
- **Auth**: Session-based auth (Redis), OAuth2 (Google, GitHub)
- **Background Jobs**: NATS JetStream worker (email, search indexing, cron)
- **Storage**: SeaweedFS (content), Cloudflare R2 (files)
- **Search**: MeiliSearch full-text search
- **API Docs**: OpenAPI/Swagger (debug builds)

## Quick Start

```bash
git clone https://github.com/shiueo/AxumKit.git
cd AxumKit
cp .env.example .env

# Run migrations
cd crates/migration && cargo run && cd ../..

# Start server & worker
cargo run -p axumkit-server
cargo run -p axumkit-worker  # separate terminal
```

## License

MIT