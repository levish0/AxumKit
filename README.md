# Axum + SeaORM + PostgreSQL + JWT + REST API + OpenAPI Template

[![Rust](https://img.shields.io/badge/rust-stable-blue.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A production-ready template for building high-performance web APIs with Rust, Axum, and SeaORM.

## Features

- **High-performance Web Server**: Built on Axum, a fast and modular web framework
- **Type-safe ORM**: SeaORM for database operations with compile-time guarantees
- **PostgreSQL Support**: Robust relational database integration
- **JWT Authentication**: Secure authentication system out of the box
- **RESTful API**: Well-structured endpoints following best practices
- **OpenAPI Documentation**: Auto-generated API docs with Swagger UI
- **Modular Architecture**: Clean separation of concerns for maintainability
- **Environment-based Configuration**: Flexible configuration management
- **Database Connection Pooling**: Efficient connection handling
- **Async/Await**: Fully asynchronous for maximum performance
- **Error Handling**: Consistent error responses and logging

## Getting Started

### Prerequisites

- Rust (latest stable version recommended)
- PostgreSQL (12+)
- Cargo (Rust's package manager)

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/axum-seaorm-postgresql-template.git
   cd axum-seaorm-postgresql-template
   ```

2. Set up environment variables:
   ```bash
   cp .env.example .env
   ```
   Update the `.env` file with your database credentials and other settings.

3. Install dependencies and build:
   ```bash
   cargo build
   ```

4. Run database migrations (requires SeaORM CLI):
   ```bash
   cargo install sea-orm-cli
   sea-orm-cli migrate up
   ```

5. Start the server:
   ```bash
   cargo run
   ```

6. Access the API documentation:
   ```
   http://localhost:8000/docs
   ```

## Project Structure

```
src/
├── api/               # API routes and handlers
├── config/            # Application configuration
├── database/          # Database connection and setup
├── dto/               # Data Transfer Objects
├── entity/            # SeaORM entities
├── middleware/        # Axum middleware
├── service/           # Business logic
├── main.rs            # Application entry point
└── state.rs           # Application state
```

## API Documentation

This project uses `utoipa` to automatically generate OpenAPI documentation and serve it via Swagger UI.

- **Swagger UI**: `http://localhost:8000/docs`
- **OpenAPI JSON**: `http://localhost:8000/api-doc/openapi.json`

### Example API Endpoint

```rust
#[utoipa::path(
    get,
    path = "/v0/user/{id}",
    params(
        ("id" = i32, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Successfully retrieved user", body = UserInfoResponse),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "User"
)]
pub async fn get_user(
    state: State<AppState>,
    Path(id): Path<String>,
) -> Result<UserInfoResponse, Errors> {
    // Handler implementation
}
```

## Architecture

This project follows a layered architecture:

1. **API Layer**: Handles HTTP requests/responses
2. **Service Layer**: Contains business logic
3. **Repository Layer**: Manages database operations
4. **Domain Layer**: Defines entities and DTOs

## Environment Variables

Configure the following in your `.env` file:

```env
DATABASE_URL=postgres://username:password@localhost:5432/dbname
JWT_SECRET=your_jwt_secret_key
PORT=8000
RUST_LOG=info
```

## License

This project is licensed under the MIT License - see the [LICENSE](./LICENSE) file for details.

---

<div align="center">
  <sub>Created by <a href="https://github.com/shiueo">Levi Lim</a> | <a href="https://github.com/shiueo/axum-seaorm-postgresql-jwt-rest-openapi-template">GitHub</a></sub>
</div>