mod api_access;
mod files;
mod identity;
mod notifications;
mod openapi;
mod organizations;
pub(super) mod pagination;
mod posts;
mod profiles;

pub(super) fn routes() -> axum::Router<super::HttpState> {
    axum::Router::new()
        .merge(identity::routes())
        .merge(organizations::routes())
        .merge(api_access::routes())
        .merge(files::routes())
        .merge(posts::routes())
        .merge(profiles::routes())
        .merge(notifications::routes())
}

pub fn openapi_document() -> utoipa::openapi::OpenApi {
    openapi::document()
}
