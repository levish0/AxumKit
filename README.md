<div align="right">
  <a href="./README.ko.md">한국어</a> | <strong>English</strong>
</div>

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
- Docker and Docker Compose (optional, for containerized development)

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
   
```bash
python -c "import secrets; print(secrets.token_urlsafe(64))"
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
- **migrate**: Runs database migrations on startup

### Environment Variables

You can configure the application by creating a `.env` file in the project root. See `.env.example` for available options.

### Development Workflow

- The application will automatically reload when source code changes
- Database migrations run automatically on startup
- The database persists data in a Docker volume

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
