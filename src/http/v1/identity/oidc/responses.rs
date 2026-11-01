use super::super::contracts::*;
use crate::http::{HttpState, cookies};
use axum::{
    Json,
    response::{IntoResponse, Response},
};

pub(super) fn redirect_response(
    state: &HttpState,
    redirect: application::identity::oidc::AuthorizationRedirect,
) -> Response {
    let mut response = Json(OidcRedirectOutput {
        authorization_url: redirect.url.into(),
    })
    .into_response();
    cookies::set_cookie(
        &mut response,
        &state.settings,
        state.settings.oauth_cookie_name(),
        redirect.browser_binding,
        300,
    );
    response
}
