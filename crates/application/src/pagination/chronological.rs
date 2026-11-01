use super::{Page, PageRequest};
use crate::{ApplicationError, ApplicationResult};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use sea_orm::ExprTrait;
use sea_orm::sea_query::{Condition, Expr, IntoColumnRef};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct ChronologicalPosition {
    pub created_at: DateTime<Utc>,
    pub id: Uuid,
}

#[derive(Serialize, Deserialize)]
enum CursorOrder {
    #[serde(rename = "created_at_desc")]
    CreatedAtDescending,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    version: u8,
    scope: String,
    order: CursorOrder,
    before: ChronologicalPosition,
}

#[derive(Clone, Copy)]
pub struct ChronologicalPageRequest {
    pub(crate) limit: u64,
    pub(crate) before: Option<ChronologicalPosition>,
}

impl ChronologicalPageRequest {
    pub fn parse(limit: Option<u64>, cursor: Option<&str>, scope: &str) -> ApplicationResult<Self> {
        let limit = PageRequest::parse(limit, None, scope)?.limit;
        let before = cursor
            .map(|encoded| {
                if encoded.len() > 512 {
                    return Err(ApplicationError::InvalidInput("invalid cursor"));
                }
                let bytes = URL_SAFE_NO_PAD
                    .decode(encoded)
                    .map_err(|_| ApplicationError::InvalidInput("invalid cursor"))?;
                let cursor: Cursor = serde_json::from_slice(&bytes)
                    .map_err(|_| ApplicationError::InvalidInput("invalid cursor"))?;
                if cursor.version != 1 || cursor.scope != scope {
                    return Err(ApplicationError::InvalidInput(
                        "cursor does not match this list",
                    ));
                }
                Ok(cursor.before)
            })
            .transpose()?;
        Ok(Self { limit, before })
    }

    pub(crate) fn condition(
        &self,
        timestamp: impl IntoColumnRef + Clone,
        id: impl IntoColumnRef,
    ) -> Condition {
        match self.before {
            Some(before) => Condition::any()
                .add(Expr::col(timestamp.clone()).lt(before.created_at))
                .add(
                    Condition::all()
                        .add(Expr::col(timestamp).eq(before.created_at))
                        .add(Expr::col(id).lt(before.id)),
                ),
            None => Condition::all(),
        }
    }
}

impl<T> Page<T> {
    pub fn from_chronological_rows(
        mut rows: Vec<T>,
        request: ChronologicalPageRequest,
        scope: &str,
        position: impl Fn(&T) -> ChronologicalPosition,
    ) -> Self {
        let has_more = rows.len() as u64 > request.limit;
        rows.truncate(request.limit as usize);
        let next_cursor = has_more.then(|| {
            URL_SAFE_NO_PAD.encode(
                serde_json::to_vec(&Cursor {
                    version: 1,
                    scope: scope.into(),
                    order: CursorOrder::CreatedAtDescending,
                    before: position(rows.last().expect("positive page limit")),
                })
                .expect("cursor serialization"),
            )
        });
        Self {
            items: rows,
            next_cursor,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cursors_bind_filter_scope_order_and_timestamp_tie_breaker() {
        let position = ChronologicalPosition {
            created_at: Utc::now(),
            id: Uuid::now_v7(),
        };
        let page = Page::from_chronological_rows(
            vec![position, position],
            ChronologicalPageRequest::parse(Some(1), None, "org:published").unwrap(),
            "org:published",
            |item| *item,
        );
        let decoded =
            ChronologicalPageRequest::parse(None, page.next_cursor.as_deref(), "org:published")
                .unwrap()
                .before
                .unwrap();
        assert_eq!(decoded.id, position.id);
        assert_eq!(decoded.created_at, position.created_at);
        assert!(
            ChronologicalPageRequest::parse(None, page.next_cursor.as_deref(), "org:draft")
                .is_err()
        );
        assert!(ChronologicalPageRequest::parse(Some(101), None, "org:published").is_err());
        let invalid_order = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&serde_json::json!({
                "version": 1, "scope": "org:published", "order": "created_at_asc", "before": position
            })).unwrap(),
        );
        assert!(
            ChronologicalPageRequest::parse(None, Some(&invalid_order), "org:published").is_err()
        );
    }
}
