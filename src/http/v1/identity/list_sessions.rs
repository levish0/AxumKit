use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiQuery, Session},
    v1::pagination::PaginationQuery,
};
use axum::{Json, extract::State};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct SessionOutput {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
pub struct SessionPageOutput {
    pub items: Vec<SessionOutput>,
    pub next_cursor: Option<String>,
}

#[utoipa::path(get, path="/api/v1/sessions", tag="Sessions", security(("cookieAuth"=[]),("bearerAuth"=[])), params(PaginationQuery), responses((status=200,body=SessionPageOutput)))]
pub async fn list_sessions(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiQuery(query): ApiQuery<PaginationQuery>,
) -> Result<Json<SessionPageOutput>, ApiError> {
    let page = state
        .app
        .identity
        .list_sessions(&actor, query.limit, query.cursor.as_deref())
        .await?;
    Ok(Json(SessionPageOutput {
        items: page
            .items
            .into_iter()
            .map(|session| SessionOutput {
                id: session.id,
                created_at: session.created_at,
                last_seen_at: session.last_seen_at,
                expires_at: session.expires_at,
            })
            .collect(),
        next_cursor: page.next_cursor,
    }))
}
