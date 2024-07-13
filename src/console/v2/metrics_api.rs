use crate::common::appdata::AppShareData;
use crate::common::model::ApiResult;
use crate::console::model::metrics_model::TimelineQueryRequest;
use crate::metrics::model::{MetricsRequest, MetricsResponse};
use crate::metrics::timeline::model::TimelineQueryParam;
use actix_web::web::Data;
use actix_web::{web, HttpResponse, Responder};
use std::sync::Arc;

pub async fn query_metrics_timeline(
    app: Data<Arc<AppShareData>>,
    web::Query(req): web::Query<TimelineQueryRequest>,
) -> actix_web::Result<impl Responder> {
    let param: TimelineQueryParam = req.into();
    if let Ok(Ok(v)) = app
        .metrics_manager
        .send(MetricsRequest::TimelineQuery(param))
        .await
    {
        match v {
            MetricsResponse::TimelineResponse(response) => {
                Ok(HttpResponse::Ok().json(ApiResult::success(Some(response))))
            }
            _ => Ok(HttpResponse::Ok().json(ApiResult::<()>::error(
                "SYSTEM_ERROR".to_owned(),
                Some("send msg to metrics_manager error".to_owned()),
            ))),
        }
    } else {
        Ok(HttpResponse::Ok().json(ApiResult::<()>::error(
            "SYSTEM_ERROR".to_owned(),
            Some("send msg to metrics_manager error".to_owned()),
        )))
    }
}
