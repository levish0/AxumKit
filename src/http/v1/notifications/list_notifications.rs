use crate::http::{
    HttpState,
    error::ApiError,
    extract::{ApiQuery, Session},
    v1::{notifications::contracts::NotificationOutput, pagination::PaginationQuery},
};
use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct NotificationPageOutput {
    pub items: Vec<NotificationOutput>,
    pub next_cursor: Option<String>,
}

#[utoipa::path(get,path="/api/v1/notifications",tag="Notifications",security(("cookieAuth"=[]),("bearerAuth"=[])),params(PaginationQuery),responses((status=200,body=NotificationPageOutput)))]
pub async fn list_notifications(
    State(state): State<HttpState>,
    Session(actor): Session,
    ApiQuery(query): ApiQuery<PaginationQuery>,
) -> Result<Json<NotificationPageOutput>, ApiError> {
    let page = state
        .app
        .notifications
        .list_notifications(&actor, query.limit, query.cursor.as_deref())
        .await?;
    Ok(Json(NotificationPageOutput {
        items: page.items.into_iter().map(Into::into).collect(),
        next_cursor: page.next_cursor,
    }))
}
