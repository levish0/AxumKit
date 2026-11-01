use crate::{ApplicationError, ApplicationResult};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod chronological;
mod numbered;
pub use chronological::{ChronologicalPageRequest, ChronologicalPosition};
pub use numbered::{NumberedPageRequest, SEARCH_WINDOW_LIMIT, SearchPage};

#[derive(Clone, Copy)]
pub struct PageRequest {
    pub(crate) limit: u64,
    pub(crate) after_id: Option<Uuid>,
}
#[derive(Serialize, Deserialize)]
struct Cursor {
    version: u8,
    scope: String,
    after_id: Uuid,
}

impl PageRequest {
    pub fn parse(limit: Option<u64>, cursor: Option<&str>, scope: &str) -> ApplicationResult<Self> {
        let limit = limit.unwrap_or(25);
        if !(1..=100).contains(&limit) {
            return Err(ApplicationError::InvalidInput(
                "limit must be between 1 and 100",
            ));
        }
        let after_id = cursor
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
                Ok(cursor.after_id)
            })
            .transpose()?;
        Ok(Self { limit, after_id })
    }
}

pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}
impl<T> Page<T> {
    pub fn from_rows(
        mut rows: Vec<T>,
        request: PageRequest,
        scope: &str,
        row_id: impl Fn(&T) -> Uuid,
    ) -> Self {
        let has_more = rows.len() as u64 > request.limit;
        rows.truncate(request.limit as usize);
        let next_cursor = has_more.then(|| {
            let cursor = Cursor {
                version: 1,
                scope: scope.into(),
                after_id: row_id(rows.last().expect("positive page limit")),
            };
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&cursor).expect("cursor serialization"))
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
    fn cursor_is_scoped_and_limits_are_bounded() {
        assert!(PageRequest::parse(Some(0), None, "sessions").is_err());
        assert!(PageRequest::parse(Some(101), None, "sessions").is_err());
        let rows = vec![Uuid::now_v7(), Uuid::now_v7()];
        let page = Page::from_rows(
            rows,
            PageRequest::parse(Some(1), None, "sessions").unwrap(),
            "sessions",
            |id| *id,
        );
        assert!(PageRequest::parse(None, page.next_cursor.as_deref(), "organizations").is_err());
        assert!(
            PageRequest::parse(None, page.next_cursor.as_deref(), "sessions")
                .unwrap()
                .after_id
                .is_some()
        );
    }
}
