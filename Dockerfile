# Build stage
FROM rust:1.75-slim-bookworm as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
    && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

# Build the application
COPY . .
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary and configuration
COPY --from=builder /usr/src/app/target/release/axum-seaorm-postgresql-template .
COPY --from=builder /usr/src/app/.env .

# Expose the port the app runs on
EXPOSE 8000

# Run the application
CMD ["./axum-seaorm-postgresql-template"]
