use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiJson, ApiPath, Session},
    v1::posts::contracts::{PostInput, PostOutput},
};
use axum::{Extension, Json, extract::State};
use jobs::TraceContext;
use uuid::Uuid;

#[utoipa::path(put,path="/api/v1/organizations/{organization_id}/posts/{post_id}",tag="Posts",security(("cookieAuth"=[]),("bearerAuth"=[])),params(("organization_id"=Uuid,Path),("post_id"=Uuid,Path)),request_body=PostInput,responses((status=200,body=PostOutput)))]
pub async fn update_post(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath((organization_id, post_id)): ApiPath<(Uuid, Uuid)>,
    Extension(trace): Extension<TraceContext>,
    ApiJson(input): ApiJson<PostInput>,
) -> Result<Json<PostOutput>, ApiError> {
    Ok(Json(
        state
            .app
            .posts
            .update_post(&actor, organization_id, post_id, input.into(), trace)
            .await?
            .into(),
    ))
}
