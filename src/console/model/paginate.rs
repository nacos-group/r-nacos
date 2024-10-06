use std::ops::Sub;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(from = "String")]
pub struct Page(usize);

impl Page {
    pub fn new(page: usize) -> Self {
        Self(page.max(1))
    }
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

impl Default for Page {
    fn default() -> Self {
        Self(1)
    }
}

impl Sub<usize> for Page {
    type Output = usize;

    fn sub(self, rhs: usize) -> Self::Output {
        self.0 - rhs
    }
}

impl From<String> for Page {
    fn from(page: String) -> Self {
        Self(page.parse::<usize>().unwrap_or(1))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginateQuery {
    pub page_no: Page,
    pub page_size: usize,
}

impl Default for PaginateQuery {
    fn default() -> Self {
        Self {
            page_no: Page::default(),
            page_size: 0xffff_ffff,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginateResponse<T> {
    pub count: usize,
    pub list: Vec<T>,
}
