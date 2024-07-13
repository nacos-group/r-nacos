use crate::common::appdata::AppShareData;
use crate::common::model::ApiResult;
use crate::console::model::metrics_model::TimelineQueryRequest;
use crate::grpc::handler::NAMING_ROUTE_REQUEST;
use crate::grpc::PayloadUtils;
use crate::metrics::model::{MetricsRequest, MetricsResponse};
use crate::metrics::timeline::model::TimelineQueryParam;
use crate::naming::cluster::model::{NamingRouteRequest, NamingRouterResponse};
use actix_web::web::Data;
use actix_web::{web, HttpResponse, Responder};
use std::sync::Arc;

pub async fn query_metrics_timeline(
    app: Data<Arc<AppShareData>>,
    web::Query(req): web::Query<TimelineQueryRequest>,
) -> actix_web::Result<impl Responder> {
    let param: TimelineQueryParam = req.into();
    match do_query_metrics_timeline(app, param).await {
        Ok(v) => Ok(v),
        Err(err) => Ok(HttpResponse::Ok().json(ApiResult::<()>::error(
            "SYSTEM_ERROR".to_owned(),
            Some(err.to_string()),
        ))),
    }
}

async fn do_query_metrics_timeline(
    app: Data<Arc<AppShareData>>,
    param: TimelineQueryParam,
) -> anyhow::Result<HttpResponse> {
    let resp = if param.node_id == 0 || param.node_id == app.sys_config.raft_node_id {
        if let MetricsResponse::TimelineResponse(mut resp) = app
            .metrics_manager
            .send(MetricsRequest::TimelineQuery(param))
            .await??
        {
            resp.from_node_id = app.sys_config.raft_node_id;
            resp
        } else {
            return Err(anyhow::anyhow!("query timeline error"));
        }
    } else {
        let node_id = param.node_id;
        let addr = app.naming_node_manage.get_node_addr(param.node_id).await?;
        let req = NamingRouteRequest::MetricsTimelineQuery(param);
        let request = serde_json::to_string(&req).unwrap_or_default();
        let payload = PayloadUtils::build_payload(NAMING_ROUTE_REQUEST, request);
        let resp_payload = app
            .cluster_sender
            .send_request(addr.clone(), payload)
            .await?;
        let body_vec = resp_payload.body.unwrap_or_default().value;
        let resp: NamingRouterResponse = serde_json::from_slice(&body_vec)?;
        if let NamingRouterResponse::MetricsTimeLineResponse(v) = resp {
            v
        } else {
            return Err(anyhow::anyhow!(
                "query remote timeline error,node:{},addr:{}",
                node_id,
                addr
            ));
        }
    };
    Ok(HttpResponse::Ok().json(ApiResult::success(Some(resp))))
}
