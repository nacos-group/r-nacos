use std::{collections::HashMap, sync::Arc, time::Duration};

use actix::prelude::*;
use bean_factory::{bean, Inject};

use super::{
    model::{NamingRouteRequest, SyncBatchDataInfo, SyncBatchForSend},
    node_manage::{InnerNodeManage, NodeManageRequest},
};
use crate::naming::model::{Instance, InstanceKey};
use crate::now_second_u32;

pub struct NotifyItem {
    instance: Arc<Instance>,
    is_update: bool,
}

#[bean(inject)]
pub struct ClusterInstanceDelayNotifyActor {
    instances_map: HashMap<InstanceKey, NotifyItem>,
    beat_instances_map: HashMap<InstanceKey, Arc<Instance>>,
    manage_addr: Option<Addr<InnerNodeManage>>,
    delay: u64,
    beat_last_notify_time: u32,
}

impl Actor for ClusterInstanceDelayNotifyActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!("ClusterInstanceDelayNotifyActor started");
        self.notify_heartbeat(ctx);
    }
}

impl Inject for ClusterInstanceDelayNotifyActor {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        factory_data: bean_factory::FactoryData,
        _factory: bean_factory::BeanFactory,
        _ctx: &mut Self::Context,
    ) {
        self.manage_addr = factory_data.get_actor();
        log::info!("ClusterInstanceDelayNotifyActor inject complete");
    }
}

impl Default for ClusterInstanceDelayNotifyActor {
    fn default() -> Self {
        Self::new()
    }
}

impl ClusterInstanceDelayNotifyActor {
    pub fn new() -> Self {
        Self {
            instances_map: Default::default(),
            beat_instances_map: Default::default(),
            manage_addr: None,
            delay: 500,
            beat_last_notify_time: 0,
        }
    }

    fn delay_notify(&mut self, instance: Arc<Instance>, is_update: bool) {
        let key = instance.get_instance_key();
        self.beat_instances_map.remove(&key);
        let item = NotifyItem {
            instance,
            is_update,
        };
        self.instances_map.insert(key, item);
    }

    fn delay_beat_notify(&mut self, instance: Arc<Instance>) {
        let key = instance.get_instance_key();
        //如果不存在，则添加到心跳列表中
        if !self.instances_map.contains_key(&key) {
            self.beat_instances_map.insert(key, instance);
        }
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
            } else {
                remove_instances.push(item.instance.clone());
            }
        }
        if update_instances.is_empty() && remove_instances.is_empty() {
            return;
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
            } else {
                log::error!("SyncBatchDataInfo to_bytes error!");
            }
        }
    }

    fn do_notify_beat(&mut self) {
        let now_second = now_second_u32();
        if now_second - self.beat_last_notify_time < 15 {
            return;
        }
        self.beat_last_notify_time = now_second;
        if self.beat_instances_map.is_empty() {
            return;
        }
        let mut update_instances = vec![];
        for instance in self.beat_instances_map.values() {
            update_instances.push(instance.clone());
        }
        self.beat_instances_map.clear();
        if let Some(manage_addr) = self.manage_addr.as_ref() {
            let batch_for_send = SyncBatchForSend {
                update_instances,
                remove_instances: vec![],
            };
            let batch_info: SyncBatchDataInfo = batch_for_send.into();
            if let Ok(batch_data) = batch_info.to_bytes() {
                let req = NamingRouteRequest::SyncBatchInstances(batch_data);
                manage_addr.do_send(NodeManageRequest::SendToOtherNodes(req));
            } else {
                log::error!("SyncBatchDataInfo to_bytes error!");
            }
        }
    }

    pub fn notify_heartbeat(&self, ctx: &mut actix::Context<Self>) {
        ctx.run_later(Duration::from_millis(self.delay), |act, ctx| {
            act.do_notify();
            act.do_notify_beat();
            act.notify_heartbeat(ctx);
        });
    }
}

#[derive(Message, Debug)]
#[rtype(result = "anyhow::Result<InstanceDelayNotifyResponse>")]
pub enum InstanceDelayNotifyRequest {
    UpdateInstance(Arc<Instance>),
    ///更新实例心跳,用于http心跳接口
    UpdateInstanceBeat(Arc<Instance>),
    RemoveInstance(Arc<Instance>),
}

pub enum InstanceDelayNotifyResponse {
    None,
}

impl Handler<InstanceDelayNotifyRequest> for ClusterInstanceDelayNotifyActor {
    type Result = anyhow::Result<InstanceDelayNotifyResponse>;

    fn handle(
        &mut self,
        msg: InstanceDelayNotifyRequest,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        #[cfg(feature = "debug")]
        log::info!("ClusterInstanceDelayNotifyActor handle msg:{:?}", &msg);
        match msg {
            InstanceDelayNotifyRequest::UpdateInstance(instance) => {
                self.delay_notify(instance, true);
            }
            InstanceDelayNotifyRequest::UpdateInstanceBeat(instance) => {
                self.delay_beat_notify(instance);
            }
            InstanceDelayNotifyRequest::RemoveInstance(instance) => {
                self.delay_notify(instance, false);
            }
        };
        Ok(InstanceDelayNotifyResponse::None)
    }
}
