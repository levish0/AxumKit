# AxumKit

Production-ready Rust web API template built with Axum, SeaORM, PostgreSQL, Redis, and OAuth2.

## Features

- **Axum** web framework with layered architecture (API → Service → Repository → Entity)
- **SeaORM** for type-safe database operations
- **PostgreSQL** with connection pooling
- **Redis** for sessions, caching, and rate limiting
- **OAuth2** (Google, GitHub) with session-based auth
- **NATS JetStream** for background job processing
- **SeaweedFS** for content storage
- **MeiliSearch** for full-text search
- **Cloudflare R2** for file storage
- **OpenAPI/Swagger** documentation (debug builds)

## Quick Start

```bash
# Clone and setup
git clone https://github.com/shiueo/AxumKit.git
cd AxumKit
cp .env.example .env  # Configure your environment

# Run migrations
cd crates/migration && cargo run && cd ../..

# Start server
cargo run -p axumkit-server

# Start worker (separate terminal)
cargo run -p axumkit-worker
```

API docs available at `http://localhost:8000/docs` (debug builds only).

## Project Structure

```
crates/
├── axumkit-server/     # Main API server
│   ├── api/            # Route handlers
│   ├── service/        # Business logic
│   ├── repository/     # Database queries
│   └── bridge/         # Worker client
├── axumkit-worker/     # Background job processor
│   └── jobs/           # Email, indexing, cron jobs
├── axumkit-entity/     # SeaORM entities
├── axumkit-dto/        # Request/Response types
├── axumkit-errors/     # Error handling
├── axumkit-config/     # Configuration
├── axumkit-constants/  # Constants
├── migration/          # Database migrations
└── e2e/                # End-to-end tests
```

## Core Modules

| Module | Description |
|--------|-------------|
| Auth | Session-based auth with Redis, OAuth2 (Google/GitHub) |
| Users | User management, profiles, preferences |
| Posts | Simple CRUD with SeaweedFS content storage |
| Worker | Email sending, search indexing, scheduled tasks |

## Commands

```bash
# Development
cargo run -p axumkit-server    # Start API server
cargo run -p axumkit-worker    # Start background worker
cargo check                    # Type check
cargo test                     # Run tests

# Migrations
cd crates/migration
cargo run                      # Apply migrations
cargo run -- down              # Rollback last
cargo run -- fresh             # Drop all and reapply
cargo run -- generate <NAME>   # Create new migration
```

## Environment Variables

See `.env.example` for all required variables:
- Database: `POSTGRES_*`
- Redis: `REDIS_*`
- OAuth: `GOOGLE_*`, `GITHUB_*`
- Storage: `R2_*`, `SEAWEEDFS_*`
- Worker: `NATS_*`, `MEILISEARCH_*`

## License

MIT License - see [LICENSE](./LICENSE)
