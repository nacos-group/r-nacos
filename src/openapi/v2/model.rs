use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ApiResult<T>
where
    T: Sized + Default
{
    pub code: i32,
    pub message: String,
    pub data: T,
}

impl<T> ApiResult<T>
where
    T: Sized + Default
{
    pub fn success(data: T) -> Self {
        Self {
            code: 0,
            message: "success".into(),
            data,
        }
    }

    pub fn error(code: i32, message: String, data: T) -> Self {
        Self {
            code,
            message,
            data,
        }
    }

    pub fn server_error(data: T) -> Self {
        Self::error(30000, "server error".into(), data)
    }
}