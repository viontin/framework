//! Pagination — paginator for API/list results.
//!
//! Inspired by Laravel's Paginator.

/// A page of paginated results.
#[derive(Debug, Clone)]
pub struct Page<T: Clone> {
    pub items: Vec<T>,
    pub total: u64,
    pub per_page: u64,
    pub current_page: u64,
    pub last_page: u64,
    pub has_more: bool,
}

impl<T: Clone> Page<T> {
    pub fn new(items: Vec<T>, total: u64, page: u64, per_page: u64) -> Self {
        let last_page = if per_page == 0 { 0 } else { total.div_ceil(per_page) };
        Page {
            items,
            total,
            per_page,
            current_page: page,
            last_page,
            has_more: page < last_page,
        }
    }

    pub fn items(&self) -> &[T] { &self.items }
    pub fn into_items(self) -> Vec<T> { self.items }
    pub fn count(&self) -> usize { self.items.len() }
    pub fn is_empty(&self) -> bool { self.items.is_empty() }
    pub fn has_pages(&self) -> bool { self.last_page > 1 }
    pub fn first_page(&self) -> bool { self.current_page == 1 }
    pub fn last_page(&self) -> bool { self.current_page >= self.last_page }

    /// Generate page links (for simple pagination results).
    pub fn links(&self) -> PaginationLinks {
        PaginationLinks {
            first: Some(1),
            last: Some(self.last_page),
            prev: if self.current_page > 1 { Some(self.current_page - 1) } else { None },
            next: if self.current_page < self.last_page { Some(self.current_page + 1) } else { None },
            current: self.current_page,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PaginationLinks {
    pub first: Option<u64>,
    pub last: Option<u64>,
    pub prev: Option<u64>,
    pub next: Option<u64>,
    pub current: u64,
}

/// Create a paginated result from a full collection.
pub fn paginate<T: Clone>(items: &[T], total: u64, page: u64, per_page: u64) -> Page<T> {
    let start = ((page.max(1) - 1) * per_page) as usize;
    let sliced = items.iter().skip(start).take(per_page as usize).cloned().collect();
    Page::new(sliced, total, page, per_page)
}

/// Simple pagination without total count (for infinite scroll).
pub fn simple_paginate<T: Clone>(items: &[T], page: u64, per_page: u64) -> Page<T> {
    let start = ((page.max(1) - 1) * per_page) as usize;
    let sliced: Vec<T> = items.iter().skip(start).take(per_page as usize + 1).cloned().collect();
    let has_more = sliced.len() > per_page as usize;
    let items = if has_more { sliced[..sliced.len() - 1].to_vec() } else { sliced };
    Page {
        items,
        total: 0,
        per_page,
        current_page: page,
        last_page: 0,
        has_more,
    }
}
