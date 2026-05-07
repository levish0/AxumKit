use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        super::health_check::health_check,
    ),
    tags(
        (name = "Health", description = "Health endpoints")
    )
)]
pub struct HealthApiDoc;
