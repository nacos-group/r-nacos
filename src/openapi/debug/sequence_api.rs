use crate::common::appdata::AppShareData;
use crate::common::model::ApiResult;
use crate::sequence::{SequenceRequest, SequenceResult};
use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize)]
pub struct SequenceParam {
    pub key: String,
}

async fn get_next_id(share_data: &Arc<AppShareData>, key: String) -> anyhow::Result<u64> {
    let result = share_data
        .sequence_manager
        .send(SequenceRequest::GetNextId(Arc::new(format!("o_{}", &key))))
        .await??;
    match result {
        SequenceResult::NextId(id) => Ok(id),
        _ => Err(anyhow::anyhow!("get next id error,key:{}", &key)),
    }
}

pub async fn next_id(
    app_share_data: web::Data<Arc<AppShareData>>,
    web::Query(param): web::Query<SequenceParam>,
) -> impl Responder {
    match get_next_id(&app_share_data, param.key).await {
        Ok(id) => HttpResponse::Ok().json(ApiResult::success(Some(id))),
        Err(e) => HttpResponse::Ok().json(ApiResult::<()>::error(
            "SEQUENCE_ERROR".to_string(),
            Some(e.to_string()),
        )),
    }
}
