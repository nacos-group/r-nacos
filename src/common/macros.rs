#[macro_export]
macro_rules! merge_web_param {
    ($param:expr,$payload:expr) => {{
        let _body = match $crate::common::web_utils::get_req_body($payload).await {
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

#[macro_export]
macro_rules! merge_web_param_with_result {
    ($param:expr,$payload:expr) => {{
        let _body = match $crate::common::web_utils::get_req_body($payload).await {
            Ok(v) => v,
            Err(err) => {
                return Ok(actix_web::HttpResponse::InternalServerError().body(err.to_string()));
            }
        };
        let _b = match serde_urlencoded::from_bytes(&_body) {
            Ok(v) => v,
            Err(err) => {
                return Ok(actix_web::HttpResponse::InternalServerError().body(err.to_string()));
            }
        };
        $param.merge(_b)
    }};
}

#[macro_export]
macro_rules! user_namespace_privilege {
    ($req:expr) => {{
        if let Some(session) = $req
            .extensions()
            .get::<Arc<$crate::common::model::UserSession>>()
        {
            session
                .namespace_privilege
                .clone()
                .unwrap_or($crate::common::model::privilege::PrivilegeGroup::all())
        } else {
            $crate::common::model::privilege::PrivilegeGroup::all()
        }
    }};
}

#[macro_export]
macro_rules! user_no_namespace_permission {
    ($param:expr) => {{
        return actix_web::HttpResponse::Ok().json(crate::common::model::ApiResult::<()>::error(
            crate::common::error_code::NO_PERMISSION.to_string(),
            Some(format!("user no such namespace permission: {:?}", $param)),
        ));
    }};
}
