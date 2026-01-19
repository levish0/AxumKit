# Chef stage - install cargo-chef
FROM rust:1.92.0 AS chef
WORKDIR /app
RUN cargo install cargo-chef

# Planner stage - generate recipe.json
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Builder stage - build dependencies then application
FROM chef AS builder

# Install necessary system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency recipe
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies (this layer is cached unless dependencies change)
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source code
COPY . .

# Build all binaries (server, worker, migration)
RUN cargo build --release -p axumkit_server -p axumkit_worker -p migration

# Runtime stage for server
FROM debian:stable-slim AS server-runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN groupadd -r app && useradd -r -g app app

# Set work directory
WORKDIR /app

# Copy binaries from builder stage
COPY --from=builder /app/target/release/axumkit_server .
COPY --from=builder /app/target/release/migration .

# Change ownership to app user
RUN chown -R app:app /app

# Switch to app user
USER app

# Expose port
EXPOSE 8000

# Run the application
CMD ["./axumkit_server"]

# Runtime stage for worker
FROM debian:stable-slim AS worker-runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN groupadd -r app && useradd -r -g app app

# Set work directory
WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/axumkit_worker .

# Change ownership to app user
RUN chown -R app:app /app

# Switch to app user
USER app

# Run the worker
CMD ["./axumkit_worker"]