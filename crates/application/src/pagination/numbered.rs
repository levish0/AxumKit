use crate::{ApplicationError, ApplicationResult};

pub const SEARCH_WINDOW_LIMIT: usize = 1000;

#[derive(Clone, Copy)]
pub struct NumberedPageRequest {
    page: usize,
    per_page: usize,
}

pub struct SearchPage<T> {
    pub items: Vec<T>,
    pub page: usize,
    pub per_page: usize,
    pub total_in_window: usize,
    pub search_window_limit: usize,
    pub has_next_page: bool,
}

impl NumberedPageRequest {
    pub fn parse(page: Option<usize>, per_page: Option<usize>) -> ApplicationResult<Self> {
        let request = Self {
            page: page.unwrap_or(1),
            per_page: per_page.unwrap_or(25),
        };
        if request.page == 0
            || !(1..=100).contains(&request.per_page)
            || request
                .page
                .checked_sub(1)
                .and_then(|page| page.checked_mul(request.per_page))
                .is_none_or(|offset| offset >= SEARCH_WINDOW_LIMIT)
        {
            return Err(ApplicationError::InvalidInput("invalid search page"));
        }
        Ok(request)
    }

    pub fn from_authorized_rows<T>(self, rows: Vec<T>) -> SearchPage<T> {
        let total_in_window = rows.len();
        let offset = (self.page - 1) * self.per_page;
        SearchPage {
            items: rows.into_iter().skip(offset).take(self.per_page).collect(),
            page: self.page,
            per_page: self.per_page,
            total_in_window,
            search_window_limit: SEARCH_WINDOW_LIMIT,
            has_next_page: offset + self.per_page < total_in_window,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numbered_pages_bound_offsets_and_count_only_supplied_authorized_rows() {
        assert!(NumberedPageRequest::parse(Some(usize::MAX), Some(100)).is_err());
        assert!(NumberedPageRequest::parse(Some(0), None).is_err());
        assert!(NumberedPageRequest::parse(Some(11), Some(100)).is_err());
        let page = NumberedPageRequest::parse(Some(2), Some(2))
            .unwrap()
            .from_authorized_rows(vec![1, 2, 3]);
        assert_eq!(page.items, vec![3]);
        assert_eq!(page.total_in_window, 3);
        assert!(!page.has_next_page);
    }
}
