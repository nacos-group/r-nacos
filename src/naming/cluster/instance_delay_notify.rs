
use std::{sync::Arc, time::Duration, collections::HashMap};

use actix::prelude::*;

use crate::{naming::model::{Instance, InstanceKey}};

use super::{node_manage::{InnerNodeManage, NodeManageRequest}, model::{NamingRouteRequest, SyncBatchForSend, SyncBatchDataInfo}};

pub struct NotifyItem{
    instance: Arc<Instance>,
    is_update: bool,
}


pub struct ClusterInstanceDelayNotifyActor {
    instances_map: HashMap<InstanceKey,NotifyItem>,
    manage_addr: Option<Addr<InnerNodeManage>>,
    delay: u64,
}

impl Actor for ClusterInstanceDelayNotifyActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!("InstanceDelayNotifyActor started");
        self.notify_heartbeat(ctx);
    }
}

impl ClusterInstanceDelayNotifyActor {
    pub fn new() -> Self {
        Self {
            instances_map: Default::default(),
            manage_addr: None,
            delay: 500
        }
    }

    fn delay_notify(&mut self,instance:Arc<Instance>,is_update:bool) {
        let key = instance.get_instance_key();
        let item = NotifyItem{
            instance,
            is_update
        };
        self.instances_map.insert(key, item);
    }

    fn do_notify(&mut self) {
        if self.instances_map.is_empty() {
            return;
        }
        let mut update_instances = vec![];
        let mut remove_instances = vec![];
        for item in self.instances_map.values() {
            if item.is_update {
                update_instances.push(item.instance.clone());
            }
            else {
                remove_instances.push(item.instance.clone());
            }
        }
        self.instances_map.clear();
        if let Some(manage_addr) = self.manage_addr.as_ref() {
            let batch_for_send = SyncBatchForSend {
                update_instances,
                remove_instances,
            };
            let batch_info: SyncBatchDataInfo = batch_for_send.into();
            if let Ok(batch_data) = batch_info.to_bytes() {
                let req = NamingRouteRequest::SyncBatchInstances(batch_data);
                manage_addr.do_send(NodeManageRequest::SendToOtherNodes(req));
            }
            else{
                log::error!("SyncBatchDataInfo to_bytes error!");
            }
        }
    }

    pub fn notify_heartbeat(&self, ctx: &mut actix::Context<Self>) {
        ctx.run_later(Duration::from_millis(self.delay), |act, ctx| {
            act.do_notify();
            act.notify_heartbeat(ctx);
        });
    }
}


#[derive(Message, Debug)]
#[rtype(result = "anyhow::Result<InstanceDelayNotifyResponse>")]
pub enum InstanceDelayNotifyRequest {
    SetNamingNodeManageAddr(Addr<InnerNodeManage>),
    UpdateInstance(Arc<Instance>),
    RemoveInstance(Arc<Instance>),
}

pub enum InstanceDelayNotifyResponse {
    None
}

impl Handler<InstanceDelayNotifyRequest> for ClusterInstanceDelayNotifyActor {
    type Result=anyhow::Result<InstanceDelayNotifyResponse>;

    fn handle(&mut self, msg: InstanceDelayNotifyRequest, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            InstanceDelayNotifyRequest::SetNamingNodeManageAddr(manage_addr) => {
                self.manage_addr = Some(manage_addr);
            },
            InstanceDelayNotifyRequest::UpdateInstance(instance) => {
                self.delay_notify(instance, true);
            },
            InstanceDelayNotifyRequest::RemoveInstance(instance) => {
                self.delay_notify(instance, false);
            },
        };
        Ok(InstanceDelayNotifyResponse::None)
    }
}
