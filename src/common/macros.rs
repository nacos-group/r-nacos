#[macro_export]
macro_rules! merge_web_param {
    ($param:expr,$payload:expr) => {{
        let _body = match crate::common::web_utils::get_req_body($payload).await {
            Ok(v) => v,
            Err(err) => {
                return actix_web::HttpResponse::InternalServerError().body(err.to_string());
            }
        };
        let _b = match serde_urlencoded::from_bytes(&_body) {
            Ok(v) => v,
            Err(err) => {
                return actix_web::HttpResponse::InternalServerError().body(err.to_string());
            }
        };
        $param.merge(_b)
    }};
}
