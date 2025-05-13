FROM rust:1.75-slim-bookworm

WORKDIR /usr/src/migration

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy migration files
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

# Install sqlx-cli for migrations
RUN cargo install sqlx-cli --no-default-features --features postgres

# Run migrations
CMD ["sqlx", "migrate", "run"]
