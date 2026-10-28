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
            .get::<std::sync::Arc<$crate::common::model::UserSession>>()
        {
            session
                .namespace_privilege
                .clone()
                .map($crate::common::model::privilege::NamespacePrivilegeGroup::new)
                .unwrap_or_default()
        } else if let Some(session) = $req
            .extensions()
            .get::<std::sync::Arc<$crate::common::model::TokenSession>>()
        {
            session.namespace_privilege_group()
        } else {
            $crate::common::model::privilege::NamespacePrivilegeGroup::default()
        }
    }};
}

/// 从请求 extensions 取 TokenSession 的编辑权限; 无会话(鉴权关闭)时不限制, 返回 true
#[macro_export]
macro_rules! token_can_edit {
    ($req:expr) => {{
        $req.extensions()
            .get::<std::sync::Arc<$crate::common::model::TokenSession>>()
            .map(|s| s.can_edit())
            .unwrap_or(true)
    }};
}

/// open-api 无命名空间权限错误(403)
#[macro_export]
macro_rules! token_no_namespace_permission {
    ($param:expr) => {{
        return actix_web::HttpResponse::Forbidden().json(
            $crate::openapi::ForbiddenResult::new(
                $crate::common::error_code::NO_NAMESPACE_PERMISSION,
                format!("user no such namespace permission: {:?}", $param),
            ),
        );
    }};
}

/// open-api 无编辑权限错误(403)
#[macro_export]
macro_rules! token_no_edit_permission {
    () => {{
        return actix_web::HttpResponse::Forbidden().json($crate::openapi::ForbiddenResult::new(
            $crate::common::error_code::NO_PERMISSION,
            "user no permission to edit config".to_owned(),
        ));
    }};
}

#[macro_export]
macro_rules! user_no_namespace_permission {
    ($param:expr) => {{
        return actix_web::HttpResponse::Ok().json($crate::common::model::ApiResult::<()>::error(
            $crate::common::error_code::NO_NAMESPACE_PERMISSION.to_string(),
            Some(format!("user no such namespace permission: {:?}", $param)),
        ));
    }};
}
