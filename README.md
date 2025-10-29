<div align="right">
  <a href="./README.ko.md">한국어</a> | <strong>English</strong>
</div>

# AxumKit - Production-Ready Rust Web API Template

[![Rust](https://img.shields.io/badge/rust-stable-blue.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A production-ready template for building high-performance web APIs with Rust. Built with Axum, SeaORM, PostgreSQL, Redis, and OAuth2.

## Features

- **High-performance Web Server**: Built on Axum, a fast and modular web framework
- **Type-safe ORM**: SeaORM 2.0 for database operations with compile-time guarantees
- **PostgreSQL Support**: Robust relational database with connection pooling
- **Session-Based Authentication**: Secure Redis-backed session management with HTTP-only cookies
- **OAuth2 Integration**: Pre-configured Google and GitHub OAuth2 authentication flows
- **Redis Caching**: Session storage, rate limiting, and caching layer
- **Cloudflare R2 Support**: S3-compatible object storage integration
- **RESTful API**: Well-structured, versioned endpoints (v0) following best practices
- **OpenAPI Documentation**: Auto-generated API docs with Swagger UI (debug builds only)
- **Layered Architecture**: Clean separation: API → Service → Repository → Entity
- **Centralized Error Handling**: Domain-specific error handlers with development/production modes
- **Rate Limiting**: Per-route, per-IP rate limiting via Redis
- **Request Validation**: JSON and multipart validation using validator crate
- **Security Features**: Argon2 password hashing, CORS configuration, secure cookie management
- **Async/Await**: Fully asynchronous for maximum performance with Tokio runtime

## Getting Started

### Prerequisites

- Rust (latest stable version recommended)
- PostgreSQL (18+)
- Redis (8.0+)
- Cargo (Rust's package manager)
- Docker and Docker Compose (optional, for containerized development)

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/shiueo/AxumKit.git
   cd AxumKit
   ```

2. Set up environment variables:
   ```bash
   cp .env.example .env
   ```

   Update the `.env` file with your credentials. **Required variables**:
   ```env
   # Environment
   ENVIRONMENT=dev

   # Security (generate with: python -c "import secrets; print(secrets.token_urlsafe(64))")
   JWT_SECRET=your_secret_key_here

   # PostgreSQL
   POSTGRES_USER=postgres
   POSTGRES_PASSWORD=your_password
   POSTGRES_HOST=localhost
   POSTGRES_PORT=5432
   POSTGRES_NAME=axumkit
   POSTGRES_MAX_CONNECTION=100
   POSTGRES_MIN_CONNECTION=10

   # Redis
   REDIS_HOST=localhost
   REDIS_PORT=6379
   REDIS_TTL=3600

   # Server
   HOST=127.0.0.1
   PORT=8000

   # Session (hours)
   AUTH_SESSION_EXPIRE_TIME=24
   AUTH_EMAIL_VERIFICATION_TOKEN_EXPIRE_TIME=1
   AUTH_PASSWORD_RESET_TOKEN_EXPIRE_TIME=1

   # Google OAuth2 (get from https://console.cloud.google.com/)
   GOOGLE_CLIENT_ID=your_google_client_id
   GOOGLE_CLIENT_SECRET=your_google_client_secret
   GOOGLE_REDIRECT_URI=http://localhost:5173/auth/google/callback

   # GitHub OAuth2 (get from https://github.com/settings/developers)
   GITHUB_CLIENT_ID=your_github_client_id
   GITHUB_CLIENT_SECRET=your_github_client_secret
   GITHUB_REDIRECT_URI=http://localhost:5173/auth/github/callback

   # Cloudflare R2 (optional, for file storage)
   R2_PUBLIC_DOMAIN=your_r2_public_domain
   R2_ACCOUNT_ID=your_r2_account_id
   R2_BUCKET_NAME=your_bucket_name
   R2_ACCESS_KEY_ID=your_access_key_id
   R2_SECRET_ACCESS_KEY=your_secret_access_key

   # CORS
   CORS_ALLOWED_ORIGINS=http://localhost:5173
   CORS_ALLOWED_HEADERS=Content-Type,Authorization
   CORS_MAX_AGE=86400
   ```

3. Install dependencies and build:
   ```bash
   cargo build
   ```

4. Run database migrations:
   ```bash
   cd migration
   cargo run
   cd ..
   ```

5. Start the server:
   ```bash
   cargo run
   ```

6. Access the API documentation (debug builds only):
   ```
   http://localhost:8000/docs
   ```

## Docker Setup

This project includes Docker configuration for easy development and deployment.

### Prerequisites

- Docker Engine 20.10.0+
- Docker Compose 2.0.0+

### Building and Running with Docker

#### Build Only

If you just want to build the Docker image without running it:

```bash
docker-compose build
```

This will build all services defined in the `docker-compose.yml` file.

#### Build and Run

1. Build and start the application:
   ```bash
   docker-compose up --build
   ```

2. For running in detached mode:
   ```bash
   docker-compose up -d --build
   ```

3. View logs:
   ```bash
   docker-compose logs -f
   ```

4. Stop the application:
   ```bash
   docker-compose down
   ```

5. Stop and remove all containers, networks, and volumes:
   ```bash
   docker-compose down -v
   ```

### Services

- **app**: The main application server (port 8000)
- **db**: PostgreSQL database (port 5432)
- **redis**: Redis cache server (port 6379)
- **migrate**: Runs database migrations on startup

### Environment Variables

You can configure the application by creating a `.env` file in the project root. See the Installation section above for all required variables.

### Development Workflow

- The application will automatically reload when source code changes
- Database migrations run automatically on startup
- PostgreSQL and Redis data persist in Docker volumes

## Project Structure

```
src/
├── api/                       # API routes and handlers
│   └── v0/                    # API version 0
│       └── routes/            # Route definitions
│           ├── auth/          # Authentication endpoints (login, logout, OAuth)
│           └── health/        # Health check endpoints
├── config/                    # Application configuration
│   └── db_config.rs          # Environment-based configuration with LazyLock
├── connection/                # External service connections
│   ├── database_conn.rs      # PostgreSQL connection pool
│   ├── redis_conn.rs         # Redis connection manager
│   ├── http_conn.rs          # HTTP client for OAuth
│   └── r2_conn.rs            # Cloudflare R2 client
├── dto/                       # Data Transfer Objects (organized by domain)
│   ├── auth/                  # Authentication DTOs
│   │   ├── request/           # Login request DTOs
│   │   ├── response/          # Login/logout response DTOs
│   │   └── internal/          # Session, SessionContext, AnonymousUser
│   ├── oauth/                 # OAuth2 DTOs
│   │   ├── request/           # Google/GitHub login requests
│   │   ├── response/          # OAuth authorization URL response
│   │   └── internal/          # OAuth state, user result
│   └── user/                  # User DTOs
│       ├── request/           # User creation request
│       └── response/          # User response DTOs
├── entity/                    # SeaORM database entities
│   ├── users.rs              # User entity
│   ├── user_oauth_connections.rs  # OAuth provider links
│   └── user_refresh_tokens.rs     # Refresh tokens (future use)
├── errors/                    # Centralized error handling
│   ├── errors.rs             # Error enum with IntoResponse
│   ├── protocol.rs           # Error protocol
│   └── handlers/             # Domain-specific error handlers
│       ├── user_handler.rs
│       ├── oauth_handler.rs
│       ├── session_handler.rs
│       └── ...
├── extractors/                # Axum request extractors
│   └── session.rs            # RequiredSession and OptionalSession
├── middleware/                # Axum middleware
│   ├── auth.rs               # Session auth middleware (deprecated)
│   ├── anonymous_user.rs     # Anonymous user ID assignment
│   ├── rate_limit.rs         # Rate limiting per IP/route
│   └── cors.rs               # CORS configuration
├── repository/                # Data access layer
│   ├── user/                  # User database operations
│   │   ├── find_by_*.rs      # Returns Option<Model>
│   │   └── get_by_*.rs       # Returns Result<Model, Errors>
│   └── oauth/                 # OAuth connection operations
├── service/                   # Business logic layer
│   ├── auth/                  # Authentication services
│   │   ├── login.rs          # Email/password login
│   │   ├── logout.rs         # Session termination
│   │   └── session.rs        # Session CRUD operations
│   ├── oauth/                 # OAuth2 services
│   │   ├── provider/         # Provider-specific OAuth clients
│   │   │   ├── google/
│   │   │   └── github/
│   │   ├── google_sign_in.rs # Complete Google OAuth flow
│   │   └── github_sign_in.rs # Complete GitHub OAuth flow
│   └── validator/             # Request validation
│       ├── json_validator.rs
│       └── form_validator.rs
├── utils/                     # Utility functions
│   ├── crypto.rs             # Argon2 password hashing
│   ├── extract_ip_address.rs
│   ├── extract_user_agent.rs
│   ├── logger.rs             # Tracing initialization
│   ├── redis_cache.rs        # Redis cache operations
│   └── image_processor/      # Image processing utilities
├── main.rs                    # Application entry point
├── state.rs                   # AppState (PostgreSQL, Redis, HTTP client)
└── lib.rs                     # Library root

migration/                     # SeaORM migrations
├── src/
│   ├── m20250515_*.rs        # Create users table
│   ├── m20250521_*.rs        # Add refresh tokens
│   └── m20250531_*.rs        # Add user_agent to refresh_tokens
└── Cargo.toml
```

## API Documentation

This project uses `utoipa` to automatically generate OpenAPI documentation and serve it via Swagger UI.

**Note**: API documentation is only available in **debug builds** (when `ENVIRONMENT=dev` or `ENVIRONMENT=development`).

- **Swagger UI**: `http://localhost:8000/docs`
- **OpenAPI JSON**: `http://localhost:8000/swagger.json`

### Available API Endpoints

All endpoints are versioned under `/v0`:

#### Health Check
```
GET /v0/health_check     - Server health status
```

#### Authentication
```
POST /v0/auth/login      - Email/password login
  Request:  { "email": "user@example.com", "password": "password123" }
  Response: 204 No Content (sets session_id cookie)

POST /v0/auth/logout     - Logout (requires authentication)
  Response: 204 No Content (clears session_id cookie)
```

#### Google OAuth2
```
GET  /v0/auth/google/authorize?redirect_uri=<uri>  - Get authorization URL
  Response: { "auth_url": "https://accounts.google.com/..." }

POST /v0/auth/google     - Complete OAuth flow
  Request:  { "code": "auth_code", "state": "state_value", "handle": "username" }
  Response: 204 No Content (sets session_id cookie)
```

#### GitHub OAuth2
```
GET  /v0/auth/github/authorize?redirect_uri=<uri>  - Get authorization URL
  Response: { "auth_url": "https://github.com/login/oauth/..." }

POST /v0/auth/github     - Complete OAuth flow
  Request:  { "code": "auth_code", "state": "state_value", "handle": "username" }
  Response: 204 No Content (sets session_id cookie)
```

### Rate Limiting

The following routes have rate limiting (10 requests per minute per IP):
- `/v0/auth/google/authorize`
- `/v0/auth/google`
- `/v0/auth/github/authorize`
- `/v0/auth/github`
- `/v0/auth/login`

### Authentication Flow

1. **Session-Based**: Sessions are stored in Redis with HTTP-only cookies
2. **Session Cookie**: `session_id` (HTTP-only, Secure, SameSite)
3. **Session TTL**: Configurable via `AUTH_SESSION_EXPIRE_TIME` (default: 24 hours)
4. **Protected Routes**: Use `RequiredSession` extractor for authentication

Example protected handler:
```rust
pub async fn protected_handler(
    RequiredSession(session): RequiredSession,
) -> Result<Json<Response>, Errors> {
    let user_id = session.user_id; // UUID of authenticated user
    // Handler implementation
    Ok(Json(response))
}
```

### OAuth2 Flow

1. Client requests authorization URL from `/v0/auth/{provider}/authorize`
2. User is redirected to provider (Google/GitHub) to authorize
3. Provider redirects back to client with `code` and `state`
4. Client sends `code` and `state` to `/v0/auth/{provider}`
5. Server verifies state, exchanges code for access token
6. Server fetches user info from provider
7. Server creates or links user account
8. Server creates session and returns `session_id` cookie

## Architecture

This project follows a **layered architecture** with clear separation of concerns:

```
HTTP Request
    ↓
API Layer (src/api/)
    ↓
Service Layer (src/service/)
    ↓
Repository Layer (src/repository/)
    ↓
Entity Layer (src/entity/)
    ↓
Database (PostgreSQL)
```

### Key Components

**AppState** (`src/state.rs`)
- Shared application state containing:
  - `conn`: PostgreSQL connection pool (SeaORM)
  - `redis_client`: Redis connection manager
  - `http_client`: HTTP client for OAuth and external APIs

**Configuration** (`src/config/db_config.rs`)
- Centralized config using `LazyLock` for lazy initialization
- Loaded from `.env` file on first access
- Access via `DbConfig::get()` - returns `&'static DbConfig`

**Error Handling** (`src/errors/`)
- Centralized error system with `Errors` enum
- Domain-specific error handlers (user, oauth, session, password, etc.)
- Automatic conversion to HTTP responses via `IntoResponse`
- Development mode shows detailed errors; production mode hides them

**Session Management** (`src/service/auth/session.rs`)
- Redis-backed sessions with pattern: `session:{session_id}`
- Session data: `user_id`, `session_id`, `created_at`, `expires_at`, `user_agent`, `ip_address`
- Services: `create_session`, `get_session`, `refresh_session`, `delete_session`

**Middleware** (`src/middleware/`)
- `anonymous_user_middleware`: Assigns anonymous user IDs to all requests
- `rate_limit`: Per-route, per-IP rate limiting via Redis
- `cors_layer`: CORS configuration from environment variables

**Repository Pattern**
- `find_by_*()`: Returns `Option<Model>` (no error if not found)
- `get_by_*()`: Returns `Result<Model, Errors>` (errors if not found)

**DTO Organization** (`src/dto/`)
- Organized by domain: `auth/`, `oauth/`, `user/`
- Each domain has: `request/`, `response/`, `internal/` subdirectories
- Internal DTOs (e.g., `Session`, `SessionContext`) are not exposed in API

### Database Migrations

Migrations are managed in the `migration/` directory:
```bash
cd migration
cargo run              # Apply all pending migrations
cargo run -- up        # Same as above
cargo run -- down      # Rollback last migration
cargo run -- fresh     # Drop all tables and reapply
cargo run -- refresh   # Rollback all, then reapply
cargo run -- status    # Check migration status
cargo run -- generate <NAME>  # Generate new migration
```

### Adding New Endpoints

1. Create handler in `src/api/v0/routes/{domain}/{handler}.rs`
2. Add `#[utoipa::path(...)]` annotation for OpenAPI docs
3. Register route in `src/api/v0/routes/{domain}/routes.rs`
4. Add schemas to `src/api/v0/routes/{domain}/openapi.rs`
5. Create DTOs in `src/dto/{domain}/request/` and `response/`
6. Implement service logic in `src/service/{domain}/`
7. Add repository functions in `src/repository/{domain}/`

## License

This project is licensed under the MIT License - see the [LICENSE](./LICENSE) file for details.

---

<div align="center">
  <sub>Created by <a href="https://github.com/shiueo">Levi Lim</a> | <a href="https://github.com/shiueo/AxumKit">GitHub</a></sub>
</div>
