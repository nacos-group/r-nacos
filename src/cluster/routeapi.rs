use actix_web::web::{Data, Json};

use crate::common::appdata::AppData;

use super::route::RouterRequest;

pub async fn route_request(app: Data<AppData>,req: Json<RouterRequest>) {

}