use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiPath, ApiQuery, Session},
    v1::posts::contracts::{PostPageOutput, PostStateInput},
};
use axum::{Json, extract::State};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

#[derive(Deserialize, IntoParams, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct PostListQuery {
    pub state: Option<PostStateInput>,
    #[validate(range(min = 1, max = 100))]
    pub limit: Option<u64>,
    #[validate(length(max = 512))]
    pub cursor: Option<String>,
}

#[utoipa::path(get,path="/api/v1/organizations/{organization_id}/posts",tag="Posts",security(("cookieAuth"=[]),("bearerAuth"=[])),params(("organization_id"=Uuid,Path),PostListQuery),responses((status=200,body=PostPageOutput)))]
pub async fn list_posts(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiPath(organization_id): ApiPath<Uuid>,
    ApiQuery(query): ApiQuery<PostListQuery>,
) -> Result<Json<PostPageOutput>, ApiError> {
    let page = state
        .app
        .posts
        .list_posts(
            &actor,
            organization_id,
            query.state.map(Into::into).unwrap_or_default(),
            query.limit,
            query.cursor.as_deref(),
        )
        .await?;
    Ok(Json(PostPageOutput {
        items: page.items.into_iter().map(Into::into).collect(),
        next_cursor: page.next_cursor,
    }))
}
