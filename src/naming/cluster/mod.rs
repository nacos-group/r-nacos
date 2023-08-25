
//distor cluster

use std::sync::Arc;

use crate::common::appdata::AppShareData;

use self::model::{NamingRouterRequest, NamingRouterResponse};

pub mod node_manage;
pub mod route;
pub mod model;


pub async fn handle_naming_route(
    app: &Arc<AppShareData>,
    req: NamingRouterRequest,
) -> anyhow::Result<NamingRouterResponse> {
    todo!()
}

