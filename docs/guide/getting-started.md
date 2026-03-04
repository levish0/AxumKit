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
| S3-compatible object storage | Latest | R2 assets storage (images, sitemap) |

::: tip
You can use `docker-compose.e2e.yml` to spin up infrastructure services quickly. See [Docker deployment](/deploy/docker).
:::

## Clone and Setup

```bash
git clone https://github.com/levish0/AxumKit.git
cd AxumKit
cp .env.example .env
```

See [Environment Variables](/reference/environment) for all required values.

## Run Migrations

AxumKit uses SeaORM migrations.

```bash
cd crates/migration
cargo run -- up
```

Other commands:

```bash
cargo run -- down
cargo run -- fresh
cargo run -- status
```

## Start the Server

```bash
cargo run -p axumkit_server
```

API server starts at `http://localhost:8000`.

## Start the Worker

In another terminal:

```bash
cargo run -p axumkit_worker
```

The worker handles email delivery, user search indexing, and cron jobs.

## Verify Setup

```bash
curl http://localhost:8000/health-check
```

Swagger UI (debug builds): [http://localhost:8000/docs](http://localhost:8000/docs)

## Next Steps

- [Study Guide](/guide/study-guide)
- [Architecture](/guide/architecture)
- [Configuration](/guide/configuration)
