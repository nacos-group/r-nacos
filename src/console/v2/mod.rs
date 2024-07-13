use crate::common::model::ApiResult;
use actix_web::HttpResponse;

pub mod cluster_api;
pub mod config_api;
pub mod login_api;
pub mod metrics_api;
pub mod namespace_api;
pub mod naming_api;
pub mod user_api;

pub const ERROR_CODE_SYSTEM_ERROR: &str = "SYSTEM_ERROR";

pub enum ApiResponse<T>
where
    T: Sized + Default + serde::Serialize,
{
    Result(ApiResult<T>),
    Response(HttpResponse),
}

impl<T> From<ApiResponse<T>> for HttpResponse
where
    T: Sized + Default + serde::Serialize,
{
    fn from(value: ApiResponse<T>) -> Self {
        match value {
            ApiResponse::Result(v) => HttpResponse::Ok().json(v),
            ApiResponse::Response(v) => v,
        }
    }
}
