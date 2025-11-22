# Codebase Structure

## Directory Layout

```
AxumKit/
├── .github/              # GitHub Actions CI/CD workflows
│   └── workflows/
│       └── release.yml   # Release build workflow (cargo build --release)
├── docs/                 # Documentation
├── frontend/             # Frontend application (if any)
├── load_test/            # Load testing utilities
├── migration/            # SeaORM database migrations
│   └── Cargo.toml        # Migration CLI project
├── src/                  # Main application source code
│   ├── api/              # API routes and handlers (versioned)
│   ├── config/           # Application configuration (DbConfig via LazyLock)
│   ├── connection/       # Database, Redis, R2, HTTP client connections
│   ├── dto/              # Data Transfer Objects (request/response/internal)
│   │   ├── auth/
│   │   ├── oauth/
│   │   ├── user/
│   │   └── [domain]/     # Each domain has request/, response/, internal/ subdirs
│   ├── entity/           # SeaORM entities (generated from migrations)
│   ├── errors/           # Centralized error handling (Errors enum + handlers)
│   ├── middleware/       # Axum middleware (session_auth, anonymous_user, cors)
│   ├── repository/       # Database query layer (find_by_*, get_by_*)
│   ├── service/          # Business logic layer
│   │   ├── auth/         # Authentication services
│   │   ├── oauth/        # OAuth2 flow handling
│   │   ├── validator/    # Request validation
│   │   └── SessionService # Redis session management
│   ├── utils/            # Utility functions
│   ├── lib.rs            # Library exports
│   ├── main.rs           # Application entry point (run_server function)
│   └── state.rs          # AppState definition (conn, redis_client, http_client)
├── target/               # Cargo build artifacts
├── .env.example          # Example environment variables
├── Cargo.toml            # Project dependencies and metadata
├── CLAUDE.md             # Claude Code specific instructions
└── README.md             # Project documentation

```

## Architecture

The project follows a **layered architecture** with clear separation of concerns:

```
┌─────────────────────────────────────┐
│   API Layer (src/api/)              │
│   - Route handlers                  │
│   - Request/Response DTOs           │
│   - OpenAPI documentation           │
└─────────────┬───────────────────────┘
              │
┌─────────────▼───────────────────────┐
│   Service Layer (src/service/)      │
│   - Business logic                  │
│   - Session management              │
│   - OAuth2 flows                    │
│   - Validation                      │
└─────────────┬───────────────────────┘
              │
┌─────────────▼───────────────────────┐
│   Repository Layer (src/repository/)│
│   - Database queries                │
│   - find_by_* (returns Option)      │
│   - get_by_* (returns Result)       │
└─────────────┬───────────────────────┘
              │
┌─────────────▼───────────────────────┐
│   Entity Layer (src/entity/)        │
│   - SeaORM models                   │
│   - Database schema representation  │
└─────────────────────────────────────┘
```

## Key Components

### AppState (src/state.rs)
Application-wide shared state containing:
- `conn`: PostgreSQL database connection (SeaORM)
- `redis_client`: Redis connection manager for sessions/caching
- `http_client`: reqwest HTTP client for OAuth2 and external APIs

### Configuration (src/config/db_config.rs)
- Centralized config using `LazyLock` for environment variables
- Access via `DbConfig::get()` - returns static reference
- Loaded from `.env` file on first access

### Error Handling (src/errors/)
- Centralized error system with `Errors` enum
- Domain-specific error handlers (user, oauth, session, password, etc.)
- Errors automatically convert to HTTP responses via `IntoResponse`
- Development mode shows detailed error info; production mode hides it
- Standard result types: `ServiceResult<T>` and `ApiResult<T>`

### Authentication Flow
1. Session-based auth using Redis (not JWT tokens in production)
2. Session data stored as `session:{session_id}` in Redis with TTL
3. `session_auth` middleware extracts session from cookies and validates via Redis
4. `SessionContext` (containing `user_id` and `session_id`) injected into request extensions
5. Handlers extract `SessionContext` from request to get authenticated user info
6. OAuth2 providers (Google, GitHub) create sessions after successful authentication

### Route Structure
Routes are versioned and organized by domain:
```
/v0/health/*        - Health check endpoints
/v0/auth/*          - Authentication endpoints (login, logout, OAuth)
/docs               - Swagger UI (debug builds only)
/swagger.json       - OpenAPI spec (debug builds only)
```

### OpenAPI Documentation
- Auto-generated using `utoipa`
- Define schemas with `#[derive(ToSchema)]`
- Document endpoints with `#[utoipa::path(...)]`
- Register in `src/api/v0/routes/openapi.rs`
- Available at `/docs` in debug builds only

## Naming Conventions

### Repository Pattern
- `find_by_*`: Returns `Option<Model>` (soft lookup)
- `get_by_*`: Returns `Result<Model, Errors>` (expected to exist)

### DTO Organization
Each domain (auth, oauth, user) has:
- `request/`: Input DTOs for API endpoints
- `response/`: Output DTOs for API responses
- `internal/`: Internal DTOs not exposed in API (e.g., Session, SessionContext)
