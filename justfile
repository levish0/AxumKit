set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

export COMPOSE_FILE := "docker-compose.dev.yml"

# Local dependencies from docker-compose.dev.yml. App containers are excluded.
infra := "postgres pgdog redis-session redis-cache redis-lock nats meilisearch image-processor"

# Default command to list all available commands.
default:
    @just --list

# Set up local development infrastructure and run migrations using the root .env.
dev: up-infra migrate
    @echo "Development environment ready."
    @echo "  cargo run -p server"
    @echo "  cargo run -p worker"

# Build docker images (e.g., `just build` or `just build <service>`)
build *args:
    docker compose build {{args}}

# Start local dependency containers (postgres, pgdog, redis, nats, meilisearch, image-processor)
up-infra:
    docker compose up -d {{infra}}

# Start the full stack including server/worker containers
up:
    docker compose up -d --build --remove-orphans

# Stop and remove containers (volumes are preserved)
down:
    docker compose down

# Remove containers and their volumes
prune *args:
    docker compose down -v {{args}}

# Show container status
status:
    docker compose ps -a

# View container logs (e.g. `just logs postgres`)
logs *args:
    docker compose logs -f {{args}}

# Run database migrations
migrate:
    cargo run -p migration

# Drop everything and re-run all migrations
migrate-fresh:
    cargo run -p migration fresh

# Show migration status
migrate-status:
    cargo run -p migration status

# Export merged OpenAPI schema to swagger.json
openapi:
    cargo xtask openapi

# Format all code
fmt:
    cargo fmt --all

# Run all CI checks locally (fmt, clippy, tests, openapi drift)
# e2e is excluded here: it needs the full docker stack (see `just e2e`).
check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --exclude e2e -- -D warnings
    cargo test --workspace --exclude e2e
    cargo xtask openapi
    git diff --exit-code swagger.json

# Run unit/integration tests (e.g. `just test some_test`). Excludes the e2e crate.
test *args:
    cargo test --workspace --exclude e2e {{args}}

# Run end-to-end tests against the full disposable docker stack.
# Brings the stack up (waiting for healthchecks), runs the e2e crate, then tears it down.
e2e:
    docker compose -f docker-compose.test.yml up -d --build --wait
    -cargo test -p e2e
    docker compose -f docker-compose.test.yml down -v

# Build and push GHCR Docker images (e.g. `just publish 0.8.0 --latest`)
publish tag *args:
    cargo xtask docker-publish --tag {{tag}} {{args}}
