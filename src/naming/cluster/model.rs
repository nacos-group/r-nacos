use std::sync::Arc;
use serde::{Deserialize, Serialize};
use actix::prelude::*;

use crate::naming::model::{Instance, InstanceUpdateTag};

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
pub enum NamingRouteRequest {
    Ping(u64),
    UpdateInstance {
        instance: Instance,
        tag: Option<InstanceUpdateTag>,
    },
    RemoveInstance {
        instance: Instance,
    },
    SyncUpdateInstance{
        instance: Instance,
    },
    SyncRemoveInstance{
        instance: Instance,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NamingRouterResponse {
    None,
}

#[derive(Message,Debug,Clone)]
#[rtype(result = "anyhow::Result<SyncSenderResponse>")]
pub struct SyncSenderRequest(pub NamingRouteRequest);

#[derive(Message,Debug)]
#[rtype(result = "anyhow::Result<SyncSenderResponse>")]
pub enum SyncSenderSetCmd {
    UpdateTargetAddr(Arc<String>),
}

pub enum SyncSenderResponse {
    None
}
