# xtask

AxumKit development environment management tool.

## Commands

```bash
cargo xtask dev            # Full setup (docker + migrate)
cargo xtask docker-up      # Start Docker services
cargo xtask docker-down    # Stop/remove Docker services
cargo xtask docker-status  # Check service status
cargo xtask migrate        # Run DB migrations
cargo xtask migrate-fresh  # Drop & recreate DB
```

## Docker Services

| Service       | Port | Description         |
|---------------|------|---------------------|
| Redis Session | 6379 | Session storage     |
| Redis Cache   | 6380 | Cache               |
| NATS          | 4222 | Message queue       |
| MeiliSearch   | 7700 | Search engine       |
| SeaweedFS     | 8333 | File storage (S3)   |

> PostgreSQL runs locally (not in Docker)

## Development Setup

```bash
# 1. Full setup (recommended)
cargo xtask dev

# 2. Start server
cargo run -p axumkit-server

# 3. Start worker (in a separate terminal)
cargo run -p axumkit-worker
```

## Notes

- Docker volumes persist after `docker-down`
- Delete volumes: `docker volume rm <volume-name>`
