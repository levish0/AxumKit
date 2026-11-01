use crate::http::{
    HttpState, cookies,
    error::ApiError,
    extract::{ApiPath, ApiQuery, read_cookie},
    v1::identity::{
        contracts::{LoginOutput, SessionTransport},
        session_responses::login_response,
    },
};
use application::identity::oidc::CallbackOutcome;
use axum::{
    Extension,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use jobs::TraceContext;
use serde::Deserialize;
use utoipa::IntoParams;
use validator::Validate;

#[derive(Deserialize, IntoParams, Validate)]
pub struct OidcCallbackQuery {
    #[validate(length(min = 1, max = 1024))]
    pub state: String,
    #[validate(length(min = 1, max = 8192))]
    pub code: Option<String>,
    #[validate(length(max = 256))]
    pub error: Option<String>,
    #[validate(length(max = 2048))]
    pub iss: Option<String>,
}

#[utoipa::path(get, path="/api/v1/auth/oidc/{provider}/callback", tag="OIDC", params(("provider"=String,Path),OidcCallbackQuery), responses((status=200,body=LoginOutput),(status=204)))]
pub async fn callback(
    State(state): State<HttpState>,
    Extension(trace): Extension<TraceContext>,
    ApiPath(provider): ApiPath<String>,
    ApiQuery(query): ApiQuery<OidcCallbackQuery>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let binding = read_cookie(&headers, state.settings.oauth_cookie_name())?
        .ok_or(application::ApplicationError::Unauthenticated)?;
    if query.code.is_some() == query.error.is_some() {
        return Err(ApiError::invalid_input());
    }
    let outcome = state
        .app
        .oidc
        .complete_authorization(
            &provider,
            &query.state,
            &binding,
            query.code.as_deref(),
            query.iss.as_deref(),
            trace,
        )
        .await?;
    let mut response = match outcome {
        CallbackOutcome::Login(outcome) => {
            login_response(&state, outcome, SessionTransport::Cookie)
        }
        CallbackOutcome::Linked => StatusCode::NO_CONTENT.into_response(),
    };
    cookies::set_cookie(
        &mut response,
        &state.settings,
        state.settings.oauth_cookie_name(),
        String::new(),
        0,
    );
    Ok(response)
}
