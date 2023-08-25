use std::sync::Arc;
use serde::{Deserialize, Serialize};

use crate::naming::model::Instance;

#[derive(Clone, Debug)]
pub enum NamingRouteAddr {
    Local(u64),
    Remote(u64,Arc<String>),
}

#[derive(Clone, Debug,Serialize, Deserialize)]
pub struct UpdateInstanceReq {
    instance: Instance,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NamingRouterRequest {
    UpdateInstance {
        instance: Instance,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NamingRouterResponse {
    None
}