use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiJson, ApiPath, Session},
    v1::posts::contracts::CommentOutput,
    validation,
};
use axum::{Extension, Json, extract::State, http::StatusCode};
use jobs::TraceContext;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Deserialize, Validate, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CommentInput {
    #[validate(custom(function = "validation::comment_body"))]
    #[schema(min_length = 1, max_length = 2000)]
    pub body: String,
}

#[utoipa::path(post,path="/api/v1/organizations/{organization_id}/posts/{post_id}/comments",tag="Comments",security(("cookieAuth"=[]),("bearerAuth"=[])),params(("organization_id"=Uuid,Path),("post_id"=Uuid,Path)),request_body=CommentInput,responses((status=201,body=CommentOutput)))]
pub async fn create_comment(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath((organization_id, post_id)): ApiPath<(Uuid, Uuid)>,
    Extension(trace): Extension<TraceContext>,
    ApiJson(input): ApiJson<CommentInput>,
) -> Result<(StatusCode, Json<CommentOutput>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .app
                .posts
                .create_comment(&actor, organization_id, post_id, input.body, trace)
                .await?
                .into(),
        ),
    ))
}
