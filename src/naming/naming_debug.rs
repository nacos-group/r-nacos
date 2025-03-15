use crate::naming::core::NamingActor;
use crate::naming::model::{InstanceKey, InstanceShortKey, ServiceKey};
use crate::naming::service::Service;
use crate::now_millis_i64;
use actix::prelude::*;

#[derive(Debug, Message)]
#[rtype(result = "anyhow::Result<NamingDebugResult>")]
pub enum NamingDebugCmd {
    SetLocalInstanceIllHealth,
    RandomSetLocalInstanceIllHealth,
    ClearLocalHttpInstance,
    ClearLocalGrpcInstance,
}

pub enum NamingDebugResult {
    None,
}

impl NamingActor {
    pub(crate) fn set_local_instance_ill_health(&mut self) {
        let mut keys = vec![];
        for (service_key, item) in self.service_map.iter() {
            for (short_key, instance) in item.instances.iter() {
                if !instance.from_grpc && instance.healthy {
                    keys.push((service_key.clone(), short_key.clone()));
                }
            }
        }
        for (service_key, short_key) in &keys {
            if let Some(service) = self.service_map.get_mut(service_key) {
                service.update_instance_healthy_invalid(&short_key);
            }
        }
    }

    pub(crate) fn clear_local_http_instance(&mut self) {
        let mut keys = vec![];
        for (service_key, item) in self.service_map.iter() {
            for (short_key, instance) in item.instances.iter() {
                if !instance.from_grpc {
                    keys.push((service_key.clone(), short_key.clone()));
                }
            }
        }
        for (service_key, short_key) in &keys {
            if let Some(service) = self.service_map.get_mut(service_key) {
                service.remove_instance(&short_key, None);
            }
        }
    }

    pub(crate) fn clear_local_grpc_instance(&mut self) {
        let mut keys = vec![];
        for (service_key, item) in self.service_map.iter() {
            for (short_key, instance) in item.instances.iter() {
                if instance.from_grpc && instance.from_cluster != 0 {
                    keys.push((service_key.clone(), short_key.clone()));
                }
            }
        }
        for (service_key, short_key) in &keys {
            if let Some(service) = self.service_map.get_mut(service_key) {
                service.remove_instance(&short_key, None);
            }
        }
    }
}

impl Handler<NamingDebugCmd> for NamingActor {
    type Result = anyhow::Result<NamingDebugResult>;

    fn handle(&mut self, msg: NamingDebugCmd, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            NamingDebugCmd::SetLocalInstanceIllHealth => {
                self.set_local_instance_ill_health();
            }
            NamingDebugCmd::RandomSetLocalInstanceIllHealth => {}
            NamingDebugCmd::ClearLocalHttpInstance => {
                self.clear_local_http_instance();
            }
            NamingDebugCmd::ClearLocalGrpcInstance => {
                self.clear_local_grpc_instance();
            }
        }
        Ok(NamingDebugResult::None)
    }
}
