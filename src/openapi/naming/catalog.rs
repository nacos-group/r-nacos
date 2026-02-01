use crate::naming::api_model::InstanceVO;
use crate::naming::core::{NamingActor, NamingCmd, NamingResult};
use crate::naming::ops::ops_model::{
    OpsQueryServiceInstanceListRequest, OpsQueryServiceInstanceListResponse, OpsServiceDto,
    OpsServiceOptQueryListResponse, OpsServiceQueryListRequest,
};
use actix::Addr;
use actix_web::http::header;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder, Scope};

pub(super) fn service() -> Scope {
    web::scope("/catalog").service(query_opt_service_list)
}

#[get("/services")]
pub async fn query_opt_service_list(
    req: HttpRequest,
    param: web::Query<OpsServiceQueryListRequest>,
    naming_addr: web::Data<Addr<NamingActor>>,
) -> impl Responder {
    let service_param = param.0.to_param(&req).unwrap();
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

pub async fn get_instance_page(
    param: web::Query<OpsQueryServiceInstanceListRequest>,
    naming_addr: web::Data<Addr<NamingActor>>,
) -> impl Responder {
    let page_index = param.page_no.unwrap_or(1);
    let page_size = param.page_size.unwrap_or(20);
    match param.to_clusters_key() {
        Ok((service_key, cluster)) => {
            match naming_addr
                .send(NamingCmd::QueryInstancePage {
                    service_key,
                    cluster,
                    page_index,
                    page_size,
                    only_healthy: false,
                })
                .await
            {
                Ok(res) => {
                    let result: NamingResult = res.unwrap();
                    match result {
                        NamingResult::InstanceInfoPage((count, list)) => {
                            let response = OpsQueryServiceInstanceListResponse::new(
                                count as u64,
                                list.into_iter()
                                    .map(|e| InstanceVO::from_instance(&e))
                                    .collect::<Vec<_>>(),
                            );
                            HttpResponse::Ok()
                                .insert_header(header::ContentType(mime::APPLICATION_JSON))
                                .json(response)
                        }
                        _ => HttpResponse::InternalServerError()
                            .body("naming actor response type error"),
                    }
                }
                Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
            }
        }
        Err(err) => HttpResponse::InternalServerError().body(err),
    }
}
