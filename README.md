# Axum + SeaORM + PostgreSQL + OpenAPI Template
This project is a template for quickly starting a web application using Rust, the `axum` framework, and the `sea-orm` library for interacting with a PostgreSQL database.

## Key Features

* Efficient web application development with the `axum` framework
* Easy database interaction with the `sea-orm` library
* PostgreSQL database support
* Modular code structure
* Standardized error handling
* Configuration management via environment variables
* Database connection pooling and management
* Automatic Swagger UI generation via OpenAPI

## OpenAPI and Swagger UI

This template includes automatic Swagger UI documentation for your API using the `utoipa` crate. With `utoipa` and `utoipa_swagger_ui`, you can easily generate and serve the API documentation.

Once the application is running, you can access the Swagger UI at the following URL:
```http://localhost:8000/docs```

The Swagger UI will display interactive API documentation for all your endpoints and allow you to make API requests directly from the UI.

### Customizing the API Documentation

The OpenAPI specification is automatically generated from your endpoint annotations using `utoipa`. You can customize the paths, components, and tags in your API documentation by adding attributes to your route handler functions. For example:

```rust
#[utoipa::path(
    get,
    path = "/v0/user/{id}",
    params(
        ("id" = i32, Path, description = "User ID")
    ),
    responses(
        (status = StatusCode::OK, description = "Successfully retrieved user information", body = UserInfoResponse),
        (status = StatusCode::NOT_FOUND, description = "User not found"),
        (status = StatusCode::INTERNAL_SERVER_ERROR, description = "Internal Server Error")
    ),
    tag = "User"
)]
pub async fn get_user(
    state: State<AppState>,
    Path(id): Path<String>,
) -> Result<UserInfoResponse, Errors> {
    // handler code here
}
```

This will add detailed API documentation for the GET /user/{id} endpoint in the Swagger UI.
## Changelog
See [CHANGELOG.md](./CHANGELOG.md) for a list of changes and version history.

## License
MIT License. See [LICENSE](./LICENSE) for more details.