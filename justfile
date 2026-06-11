set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

export COMPOSE_FILE := "docker-compose.dev.yml"

# Stateful infrastructure from docker-compose.dev.yml. App containers are excluded.
infra := "postgres pgdog redis-session redis-cache redis-lock nats meilisearch"

default:
    @just --list

# Set up local development infrastructure and run migrations using the root .env.
dev: up-infra migrate
    @echo "Development environment ready."
    @echo "  cargo run -p server"
    @echo "  cargo run -p worker"

build *args:
    docker compose build {{args}}

up-infra:
    docker compose up -d {{infra}}

up:
    docker compose up -d --build --remove-orphans

down:
    docker compose down

prune *args:
    docker compose down -v {{args}}

status:
    docker compose ps -a

logs *args:
    docker compose logs -f {{args}}

migrate:
    cargo run -p migration

migrate-fresh:
    cargo run -p migration fresh

migrate-status:
    cargo run -p migration status

openapi:
    cargo xtask openapi

fmt:
    cargo fmt --all

check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --exclude e2e -- -D warnings
    cargo test --workspace --exclude e2e
    cargo xtask openapi

test *args:
    cargo test --workspace --exclude e2e {{args}}

e2e:
    $code = 0; docker compose -f docker-compose.test.yml up -d --build --wait; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }; cargo test -p e2e; $code = $LASTEXITCODE; docker compose -f docker-compose.test.yml down -v; $down = $LASTEXITCODE; if ($code -ne 0) { exit $code }; if ($down -ne 0) { exit $down }

publish tag *args:
    cargo xtask docker-publish --tag {{tag}} {{args}}
