use actix::Addr;
use actix_web::{web, HttpResponse, Responder, Scope};

use crate::common::web_utils::get_req_body;
use crate::naming::api_model::ServiceInfoParam;
use crate::naming::core::{NamingActor, NamingCmd, NamingResult};
use crate::naming::model::ServiceKey;
use crate::naming::NamingUtils;
use crate::openapi::constant::EMPTY;
use crate::openapi::naming::model::{ServiceQueryListRequest, ServiceQueryListResponce};

pub(super) fn service() -> Scope {
    web::scope("/service")
        .service(
            web::resource(EMPTY)
                .route(web::post().to(update_service))
                .route(web::put().to(update_service))
                .route(web::delete().to(remove_service))
                .route(web::get().to(query_service)),
        )
        .service(web::resource("/list").route(web::get().to(query_service_list)))
}

pub async fn query_service(
    _param: web::Query<ServiceQueryListRequest>,
    _naming_addr: web::Data<Addr<NamingActor>>,
) -> impl Responder {
    HttpResponse::InternalServerError().body("error,not support at present")
}

pub async fn update_service(
    a: web::Query<ServiceInfoParam>,
    payload: web::Payload,
    naming_addr: web::Data<Addr<NamingActor>>,
) -> impl Responder {
    let body = match get_req_body(payload).await {
        Ok(v) => v,
        Err(err) => {
            return HttpResponse::InternalServerError().body(err.to_string());
        }
    };
    let b = match serde_urlencoded::from_bytes(&body) {
        Ok(v) => v,
        Err(err) => {
            return HttpResponse::InternalServerError().body(err.to_string());
        }
    };
    let param = ServiceInfoParam::merge_value(a.0, b);
    match param.build_service_info() {
        Ok(service_info) => {
            let _ = naming_addr
                .send(NamingCmd::UpdateService(service_info))
                .await;
            HttpResponse::Ok().body("ok")
        }
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

pub async fn remove_service(
    a: web::Query<ServiceInfoParam>,
    payload: web::Payload,
    naming_addr: web::Data<Addr<NamingActor>>,
) -> impl Responder {
    let body = match get_req_body(payload).await {
        Ok(v) => v,
        Err(err) => {
            return HttpResponse::InternalServerError().body(err.to_string());
        }
    };
    let b = match serde_urlencoded::from_bytes(&body) {
        Ok(v) => v,
        Err(err) => {
            return HttpResponse::InternalServerError().body(err.to_string());
        }
    };
    let param = ServiceInfoParam::merge_value(a.0, b);
    match param.build_service_info() {
        Ok(service_info) => {
            let key = service_info.to_service_key();
            match naming_addr.send(NamingCmd::RemoveService(key)).await {
                Ok(res) => {
                    let res: anyhow::Result<NamingResult> = res;
                    match res {
                        Ok(_) => HttpResponse::Ok().body("ok"),
                        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
                    }
                }
                Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
            }
        }
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

pub async fn query_service_list(
    param: web::Query<ServiceQueryListRequest>,
    naming_addr: web::Data<Addr<NamingActor>>,
) -> impl Responder {
    let page_size = param.page_size.unwrap_or(0x7fffffff);
    let page_index = param.page_no.unwrap_or(1);
    let namespace_id = NamingUtils::default_namespace(
        param
            .namespace_id
            .as_ref()
            .unwrap_or(&"".to_owned())
            .to_owned(),
    );
    let group = NamingUtils::default_group(
        param
            .group_name
            .as_ref()
            .unwrap_or(&"".to_owned())
            .to_owned(),
    );
    let key = ServiceKey::new(&namespace_id, &group, "");
    match naming_addr
        .send(NamingCmd::QueryServicePage(key, page_size, page_index))
        .await
    {
        Ok(res) => {
            let result: NamingResult = res.unwrap();
            match result {
                NamingResult::ServicePage((c, v)) => {
                    let resp = ServiceQueryListResponce { count: c, doms: v };
                    HttpResponse::Ok().body(serde_json::to_string(&resp).unwrap())
                }
                _ => HttpResponse::InternalServerError().body("error"),
            }
        }
        Err(_) => HttpResponse::InternalServerError().body("error"),
    }
}
