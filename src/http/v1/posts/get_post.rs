use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, Session},
    v1::posts::contracts::PostOutput,
};
use axum::{Json, extract::State};
use uuid::Uuid;

#[utoipa::path(get,path="/api/v1/organizations/{organization_id}/posts/{post_id}",tag="Posts",security(("cookieAuth"=[]),("bearerAuth"=[])),params(("organization_id"=Uuid,Path),("post_id"=Uuid,Path)),responses((status=200,body=PostOutput)))]
pub async fn get_post(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath((organization_id, post_id)): ApiPath<(Uuid, Uuid)>,
) -> Result<Json<PostOutput>, ApiError> {
    Ok(Json(
        state
            .app
            .posts
            .get_post(&actor, organization_id, post_id)
            .await?
            .into(),
    ))
}
