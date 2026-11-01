use super::{
    create_client, external_organization, external_posts, get_client, issue_key, list_clients,
    list_keys, revoke_client, revoke_key, update_client,
};
use crate::http::HttpState;

pub(in crate::http::v1) fn routes() -> axum::Router<HttpState> {
    axum::Router::new()
        .route(
            "/organizations/{organization_id}/api-clients",
            axum::routing::get(list_clients::list_clients).post(create_client::create_client),
        )
        .route(
            "/organizations/{organization_id}/api-clients/{client_id}",
            axum::routing::get(get_client::get_client)
                .put(update_client::update_client)
                .delete(revoke_client::revoke_client),
        )
        .route(
            "/organizations/{organization_id}/api-clients/{client_id}/keys",
            axum::routing::get(list_keys::list_keys).post(issue_key::issue_key),
        )
        .route(
            "/organizations/{organization_id}/api-clients/{client_id}/keys/{key_id}",
            axum::routing::delete(revoke_key::revoke_key),
        )
        .route(
            "/external/organization",
            axum::routing::get(external_organization::external_organization).head(|| async {
                (
                    axum::http::StatusCode::METHOD_NOT_ALLOWED,
                    [(axum::http::header::ALLOW, "GET")],
                )
            }),
        )
        .route(
            "/external/posts",
            axum::routing::get(external_posts::external_posts).head(|| async {
                (
                    axum::http::StatusCode::METHOD_NOT_ALLOWED,
                    [(axum::http::header::ALLOW, "GET")],
                )
            }),
        )
}
