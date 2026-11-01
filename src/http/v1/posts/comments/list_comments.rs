use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, ApiQuery, Session},
    v1::{pagination::PaginationQuery, posts::contracts::CommentOutput},
};
use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct CommentPageOutput {
    pub items: Vec<CommentOutput>,
    pub next_cursor: Option<String>,
}

#[utoipa::path(get,path="/api/v1/organizations/{organization_id}/posts/{post_id}/comments",tag="Comments",security(("cookieAuth"=[]),("bearerAuth"=[])),params(("organization_id"=Uuid,Path),("post_id"=Uuid,Path),PaginationQuery),responses((status=200,body=CommentPageOutput)))]
pub async fn list_comments(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath((organization_id, post_id)): ApiPath<(Uuid, Uuid)>,
    ApiQuery(query): ApiQuery<PaginationQuery>,
) -> Result<Json<CommentPageOutput>, ApiError> {
    let page = state
        .app
        .posts
        .list_comments(
            &actor,
            organization_id,
            post_id,
            query.limit,
            query.cursor.as_deref(),
        )
        .await?;
    Ok(Json(CommentPageOutput {
        items: page.items.into_iter().map(Into::into).collect(),
        next_cursor: page.next_cursor,
    }))
}
