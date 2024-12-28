use actix_web::{http::header, web, HttpRequest, HttpResponse, Responder};

use crate::naming::core::{NamingActor, NamingCmd, NamingResult};

use super::ops_model::{OpsServiceDto, OpsServiceOptQueryListResponse, OpsServiceQueryListRequest};

use actix::prelude::*;

pub async fn query_opt_service_list(
    req: HttpRequest,
    param: web::Query<OpsServiceQueryListRequest>,
    naming_addr: web::Data<Addr<NamingActor>>,
) -> impl Responder {
    let service_param = param.0.to_param(&req).unwrap();
    if !service_param
        .namespace_privilege
        .check_option_value_permission(&service_param.namespace_id, true)
    {
        return HttpResponse::Unauthorized().body(format!(
            "user no such namespace permission: {:?}",
            &service_param.namespace_id
        ));
    }
    match naming_addr
        .send(NamingCmd::QueryServiceInfoPage(service_param))
        .await
    {
        Ok(res) => {
            let result: NamingResult = res.unwrap();
            if let NamingResult::ServiceInfoPage((size, list)) = result {
                let service_list: Vec<OpsServiceDto> =
                    list.into_iter().map(OpsServiceDto::from).collect::<_>();
                let response = OpsServiceOptQueryListResponse::new(size as u64, service_list);
                let v = serde_json::to_string(&response).unwrap();
                HttpResponse::Ok()
                    .insert_header(header::ContentType(mime::APPLICATION_JSON))
                    .body(v)
            } else {
                HttpResponse::InternalServerError().body("naming result error")
            }
        }
        Err(_) => HttpResponse::InternalServerError().body("system error"),
    }
}
