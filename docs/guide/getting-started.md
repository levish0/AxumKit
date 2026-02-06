# Getting Started

## Prerequisites

Ensure the following are installed:

| Tool | Version | Purpose |
|------|---------|---------|
| [Rust](https://rustup.rs/) | 1.93.0+ | Compiler (edition 2024) |
| [PostgreSQL](https://www.postgresql.org/) | 18+ | Primary database |
| [Redis](https://redis.io/) | 8+ | Sessions, cache, rate limiting |
| [NATS](https://nats.io/) | 2.12+ | Job queue (JetStream) |
| [MeiliSearch](https://www.meilisearch.com/) | 1.30+ | Full-text search engine |
| [SeaweedFS](https://github.com/seaweedfs/seaweedfs) | 4.x | Object storage for post content |

::: tip
You can use the provided `docker-compose.e2e.yml` to spin up all infrastructure services at once. See [Docker deployment](/deploy/docker) for details.
:::

## Clone & Setup

```bash
git clone https://github.com/levish0/AxumKit.git
cd AxumKit
```

Copy the environment file and fill in the required values:

```bash
cp .env.example .env
```

See [Environment Variables](/reference/environment) for a complete list.

## Run Migrations

AxumKit uses SeaORM migrations. Apply them to your database:

```bash
cd crates/migration
cargo run -- up
```

Other migration commands:

```bash
cargo run -- down       # Rollback last migration
cargo run -- fresh      # Drop all tables and reapply
cargo run -- status     # Check migration status
```

## Start the Server

```bash
cargo run -p axumkit_server
```

The API server starts at `http://localhost:8000` (configurable via `HOST` and `PORT`).

## Start the Worker

In a separate terminal:

```bash
cargo run -p axumkit_worker
```

The worker handles background jobs: email delivery, search indexing, storage cleanup, and cron tasks.

## Verify Setup

### Health Check

```bash
curl http://localhost:8000/health-check
```

### Swagger UI (debug builds only)

Open [http://localhost:8000/docs](http://localhost:8000/docs) in your browser to explore the auto-generated API documentation.

::: info
Swagger UI is only available in debug builds (`cargo run`). It is excluded from release builds.
:::

## Project Structure Overview

```
AxumKit/
├── crates/
│   ├── axumkit-config/     # Configuration (ServerConfig, LazyLock)
│   ├── axumkit-constants/  # Shared constants (action log actions, NATS subjects)
│   ├── axumkit-dto/        # Data Transfer Objects (request/response types)
│   ├── axumkit-entity/     # SeaORM database entities
│   ├── axumkit-errors/     # Centralized error types and handlers
│   ├── axumkit-server/     # API server (routes, services, middleware)
│   ├── axumkit-worker/     # Background worker (NATS consumers, cron jobs)
│   ├── migration/          # SeaORM database migrations
│   └── e2e/                # End-to-end tests
├── charts/                 # Helm charts for Kubernetes
├── docs/                   # This documentation (VitePress)
├── Dockerfile              # Multi-stage Docker build
└── docker-compose.e2e.yml  # Infrastructure for e2e testing
```

## Next Steps

- [Study Guide](/guide/study-guide) - Recommended order for reading the codebase
- [Architecture](/guide/architecture) - Deep dive into the layered architecture
- [Configuration](/guide/configuration) - All configuration options explained
