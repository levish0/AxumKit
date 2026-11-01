use super::{
    add_member, create_organization, get_organization, list_organizations, remove_member,
    rename_organization,
};
use crate::http::HttpState;

pub(in crate::http::v1) fn routes() -> axum::Router<HttpState> {
    axum::Router::new()
        .route(
            "/organizations",
            axum::routing::get(list_organizations::list_organizations)
                .post(create_organization::create_organization),
        )
        .route(
            "/organizations/{organization_id}",
            axum::routing::get(get_organization::get_organization)
                .put(rename_organization::rename_organization),
        )
        .route(
            "/organizations/{organization_id}/members/{account_id}",
            axum::routing::put(add_member::add_member).delete(remove_member::remove_member),
        )
}
