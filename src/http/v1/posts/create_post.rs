use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiJson, ApiPath, Session},
    v1::posts::contracts::{PostInput, PostOutput},
};
use axum::{Extension, Json, extract::State, http::StatusCode};
use jobs::TraceContext;
use uuid::Uuid;

#[utoipa::path(post,path="/api/v1/organizations/{organization_id}/posts",tag="Posts",security(("cookieAuth"=[]),("bearerAuth"=[])),params(("organization_id"=Uuid,Path)),request_body=PostInput,responses((status=201,body=PostOutput)))]
pub async fn create_post(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath(organization_id): ApiPath<Uuid>,
    Extension(trace): Extension<TraceContext>,
    ApiJson(input): ApiJson<PostInput>,
) -> Result<(StatusCode, Json<PostOutput>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .app
                .posts
                .create_post(&actor, organization_id, input.into(), trace)
                .await?
                .into(),
        ),
    ))
}
