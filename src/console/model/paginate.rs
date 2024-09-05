use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginateQuery {
    pub page_no: usize,
    pub page_size: usize,
}

impl Default for PaginateQuery {
    fn default() -> Self {
        Self {
            page_no: 1,
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
