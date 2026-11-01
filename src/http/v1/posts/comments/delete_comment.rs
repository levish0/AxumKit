use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, ApiQuery, Session},
    v1::pagination::VersionQuery,
};
use axum::{Extension, extract::State, http::StatusCode};
use jobs::TraceContext;
use uuid::Uuid;

#[utoipa::path(delete,path="/api/v1/organizations/{organization_id}/posts/{post_id}/comments/{comment_id}",tag="Comments",security(("cookieAuth"=[]),("bearerAuth"=[])),params(("organization_id"=Uuid,Path),("post_id"=Uuid,Path),("comment_id"=Uuid,Path),VersionQuery),responses((status=204)))]
pub async fn delete_comment(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath((organization_id, post_id, comment_id)): ApiPath<(Uuid, Uuid, Uuid)>,
    ApiQuery(query): ApiQuery<VersionQuery>,
    Extension(trace): Extension<TraceContext>,
) -> Result<StatusCode, ApiError> {
    state
        .app
        .posts
        .delete_comment(
            &actor,
            organization_id,
            post_id,
            comment_id,
            query.expected_version,
            trace,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
