use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiKeyCredential, ApiQuery},
    v1::{
        api_access::quota_headers::quota_headers, pagination::PaginationQuery,
        posts::contracts::PostPageOutput,
    },
};
use axum::{Extension, Json, extract::State, http::HeaderMap};
use jobs::TraceContext;

#[utoipa::path(get,path="/api/v1/external/posts",tag="External API",security(("apiKeyAuth"=[])),params(PaginationQuery),responses((status=200,body=PostPageOutput)))]
pub async fn external_posts(
    State(state): State<HttpState>,
    credential: ApiKeyCredential,
    Extension(trace): Extension<TraceContext>,
    ApiQuery(query): ApiQuery<PaginationQuery>,
) -> Result<(HeaderMap, Json<PostPageOutput>), ApiError> {
    let grant = state
        .app
        .api_access
        .admit_posts(credential.0.expose(), trace)
        .await?;
    let page = state
        .app
        .posts
        .list_for_api_client(&grant, query.limit, query.cursor.as_deref())
        .await?;
    Ok((
        quota_headers(grant.quota()),
        Json(PostPageOutput {
            items: page.items.into_iter().map(Into::into).collect(),
            next_cursor: page.next_cursor,
        }),
    ))
}
