# Project Overview

## Purpose
AxumKit is a production-ready Rust web API template for building high-performance web applications. It provides a complete, battle-tested foundation for building modern web APIs with authentication, database integration, and file storage capabilities.

## Tech Stack

### Core Framework
- **Axum 0.8**: High-performance web framework built on Tokio and Hyper
- **Tokio 1.48**: Async runtime with full feature set
- **Tower/Tower-HTTP**: Middleware and service abstractions

### Database & Storage
- **SeaORM 2.0-rc.15**: Type-safe ORM with PostgreSQL support via sqlx
- **PostgreSQL**: Primary relational database
- **Redis 0.32**: Session management and caching (with connection-manager)
- **Cloudflare R2/S3**: Object storage via aws-sdk-s3
- **MeiliSearch 0.30**: Full-text search engine integration

### Authentication & Security
- **OAuth2 5.0**: OAuth2 client for Google and GitHub integrations
- **JWT (jsonwebtoken 10.1)**: Token-based authentication with aws_lc_rs
- **Argon2 0.5**: Password hashing
- **Session-based auth**: Redis-backed sessions with cookie management (tower-cookies)

### Data Validation & Serialization
- **Serde 1.0**: JSON serialization/deserialization
- **Validator 0.20**: Request validation with derive macros
- **UUID 1.18**: UUID v4 and v7 support
- **Chrono 0.4**: Date/time handling

### API Documentation
- **utoipa 5.4**: OpenAPI specification generation
- **utoipa-swagger-ui 9.0**: Swagger UI integration (debug builds only)

### Utilities
- **reqwest 0.12**: HTTP client for OAuth and external APIs
- **dotenvy 0.15**: Environment variable loading from .env
- **tracing/tracing-subscriber**: Structured logging
- **infer 0.19**: MIME type detection
- **image 0.25**: Image processing (WebP support)
- **blake3 1.8**: Fast hashing
- **zstd 0.13**: Compression

## Project Version
- Version: 0.1.2
- Rust Edition: 2024
- Minimum Rust Version: 1.86.0
- License: MIT
