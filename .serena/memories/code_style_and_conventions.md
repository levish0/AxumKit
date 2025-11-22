# Code Style and Conventions

## Rust Style Guidelines

This project follows **standard Rust conventions** as defined by the Rust community and enforced by `rustfmt` and `clippy`.

### Formatting
- **Tool**: `rustfmt` (no custom config, uses default settings)
- **Command**: `cargo fmt` before committing
- **Indentation**: 4 spaces (Rust standard)
- **Line length**: Default rustfmt setting (100 characters)

### Naming Conventions

#### Snake Case
- **Functions**: `run_server()`, `establish_connection()`
- **Variables**: `redis_client`, `http_client`, `server_url`
- **Module names**: `db_config`, `session_service`
- **File names**: `mod.rs`, `db_config.rs`, `session_service.rs`

#### Pascal Case
- **Structs**: `AppState`, `DbConfig`, `SessionContext`
- **Enums**: `Errors`, `OAuthProvider`
- **Traits**: `IntoResponse`
- **Type aliases**: `ServiceResult<T>`, `ApiResult<T>`

#### Screaming Snake Case
- **Constants**: `JWT_SECRET`, `DATABASE_URL`
- **Static variables**: Used with `LazyLock`

### Code Organization

#### Module Structure
```rust
pub mod errors;          // Public module
mod handlers;            // Private module
mod protocol;            // Private module
```

#### Import Organization
Group imports in this order:
1. Standard library (`std::*`)
2. External crates (`axum::*`, `serde::*`)
3. Internal modules (`crate::*`)

Example:
```rust
use std::net::SocketAddr;

use axum::{Router, middleware};
use tower_cookies::CookieManagerLayer;

use crate::state::AppState;
use crate::config::db_config::DbConfig;
```

### Error Handling

#### Pattern
- Use `Result<T, Errors>` for operations that can fail
- Define type aliases: `ServiceResult<T>` and `ApiResult<T>`
- Errors implement `IntoResponse` for automatic HTTP conversion
- Use `?` operator for error propagation
- Use `.map_err()` for error conversion

Example:
```rust
pub async fn handler() -> Result<Json<Response>, Errors> {
    let user = repository::get_by_id(&conn, id)
        .await
        .map_err(|_| Errors::UserNotFound)?;
    Ok(Json(response))
}
```

#### Error Messages
- Development mode: Detailed error information
- Production mode: Generic messages, hide implementation details

### Async/Await

- Use `async fn` for asynchronous functions
- All handlers are `async`
- Use `.await` for async operations
- Tokio runtime with full features

Example:
```rust
pub async fn run_server() -> anyhow::Result<()> {
    let conn = establish_connection().await;
    // ...
}
```

### Repository Pattern

```rust
// Soft lookup - returns Option
pub async fn find_by_id(conn: &DatabaseConnection, id: Uuid) -> Option<Model> {
    // ...
}

// Expected to exist - returns Result
pub async fn get_by_id(conn: &DatabaseConnection, id: Uuid) -> Result<Model, Errors> {
    find_by_id(conn, id)
        .await
        .ok_or(Errors::UserNotFound)
}
```

### DTO Patterns

#### Request DTOs
- Use `serde::Deserialize`
- Use `validator::Validate` for validation
- Use `utoipa::ToSchema` for OpenAPI docs

```rust
#[derive(Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    
    #[validate(length(min = 8))]
    pub password: String,
}
```

#### Response DTOs
- Use `serde::Serialize`
- Use `utoipa::ToSchema` for OpenAPI docs

```rust
#[derive(Serialize, ToSchema)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub created_at: DateTime<Utc>,
}
```

### OpenAPI Documentation

Every API endpoint should have documentation:

```rust
#[utoipa::path(
    post,
    path = "/v0/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Successfully logged in", body = LoginResponse),
        (status = 401, description = "Invalid credentials"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Authentication"
)]
pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, Errors> {
    // Implementation
}
```

### Session Management Pattern

Extract session context in handlers:
```rust
pub async fn handler(
    Extension(session): Extension<SessionContext>,
) -> Result<Response, Errors> {
    let user_id = session.user_id; // UUID
    // ...
}
```

### Configuration Access

Use `DbConfig::get()` for configuration:
```rust
let server_url = format!(
    "{}:{}",
    &DbConfig::get().server_host,
    &DbConfig::get().server_port
);
```

### Comments and Documentation

- Use `///` for public API documentation
- Use `//` for inline comments
- Document public functions, structs, and modules
- Explain "why", not "what" (code should be self-explanatory)

### Testing

- Unit tests in the same file: `#[cfg(test)] mod tests { ... }`
- Integration tests in `tests/` directory
- Mock external dependencies (database, Redis, HTTP)
- Use descriptive test names: `test_login_with_valid_credentials`

### Code Quality Standards

Before committing:
1. `cargo fmt` - Format code
2. `cargo clippy` - Check for common mistakes
3. `cargo test` - Run tests
4. `cargo check` - Verify compilation

### Clippy Linting

Follow Clippy suggestions:
- Avoid unwrap() in production code (use ? or proper error handling)
- Use `&str` instead of `&String` in function parameters
- Prefer `if let` over `match` for single pattern
- Use iterator methods instead of explicit loops when possible
