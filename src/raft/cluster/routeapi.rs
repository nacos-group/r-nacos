use std::sync::Arc;

use actix_web::{web::{Data, Json}, Responder};

use crate::{common::appdata::AppShareData, config::core::{ConfigAsyncCmd, ConfigKey}};

use super::model::{RouterRequest, RouterResponse};

pub async fn route_request(app: Data<Arc<AppShareData>>,req: Json<RouterRequest>) -> impl Responder {
    match req.0 {
        RouterRequest::ConfigSet { key, value, extend_info:_} => {
            let config_key:ConfigKey = (&key as &str).into();
            app.config_addr.send(ConfigAsyncCmd::Add(config_key, value)).await.ok();
        },
        RouterRequest::ConfigDel { key, extend_info:_} => {
            let config_key:ConfigKey = (&key as &str).into();
            app.config_addr.send(ConfigAsyncCmd::Delete(config_key,)).await.ok();
        },
    }
    Json(RouterResponse::None)
}