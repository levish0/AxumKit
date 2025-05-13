# Stage 1: Build the application
FROM rust:1.86-slim-bookworm as builder

ARG BUILD_MODE

# 릴리스 환경 설정 오버라이드
RUN if [ "$BUILD_MODE" = "--release" ]; then \
    echo "Building in release mode"; \
else \
    echo "Building in debug mode"; \
fi

# Install required system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /usr/src/app

# Copy the manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy source files to cache dependencies
RUN mkdir -p src \
    && echo "fn main() {}" > src/main.rs \
    && echo "#[cfg(test)] mod tests {}" > src/lib.rs

# Build dependencies only (cached unless Cargo.toml changes)
RUN cargo build

# Now copy the actual source code
COPY . .

# Touch the source files to ensure cargo rebuilds them
RUN touch src/main.rs src/lib.rs

# Build the application
RUN if [ "$BUILD_MODE" = "--release" ]; then \
    cargo build --release; \
else \
    cargo build; \
fi

# Stage 2: Create a minimal runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /usr/src/app/target/${BUILD_MODE:-debug}/axum-seaorm-postgresql-template /usr/local/bin/

# Set the working directory
WORKDIR /app

# Copy environment file (if exists)
COPY .env ./

# Expose the port the app runs on
EXPOSE 8000

# Run the application
CMD ["axum-seaorm-postgresql-template"]