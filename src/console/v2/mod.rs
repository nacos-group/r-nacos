use actix_web::HttpResponse;
use crate::common::model::ApiResult;

pub mod login_api;

pub enum ApiResponse<T>
    where
    T: Sized + Default + serde::Serialize,
{
    Result(ApiResult<T>),
    Response(HttpResponse),
}

impl <T> From<ApiResponse<T>> for HttpResponse
    where
        T: Sized + Default + serde::Serialize,
{
    fn from(value: ApiResponse<T>) -> Self {
        match value {
            ApiResponse::Result(v) => {
                HttpResponse::Ok().json(v)
            }
            ApiResponse::Response(v) => v
        }
    }
}
