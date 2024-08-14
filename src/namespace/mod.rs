pub mod model;

use crate::common::constant::EMPTY_ARC_STRING;
use crate::config::core::ConfigActor;
use crate::namespace::model::{Namespace, NamespaceParam, NamespaceRaftReq, NamespaceRaftResult};
use crate::naming::core::NamingActor;
use actix::prelude::*;
use bean_factory::{bean, BeanFactory, FactoryData, Inject};
use std::collections::{HashMap, LinkedList};
use std::sync::Arc;

pub const DEFAULT_NAMESPACE: &str = "public";

#[bean(inject)]
#[derive(Default, Clone)]
pub struct NamespaceActor {
    data: HashMap<Arc<String>, Arc<Namespace>>,
    id_order_list: LinkedList<Arc<String>>,
    config_addr: Option<Addr<ConfigActor>>,
    naming_addr: Option<Addr<NamingActor>>,
}

impl Actor for NamespaceActor {
    type Context = Context<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("NamespaceActor started");
    }
}

impl Inject for NamespaceActor {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        factory_data: FactoryData,
        _factory: BeanFactory,
        ctx: &mut Self::Context,
    ) {
        self.config_addr = factory_data.get_actor();
        self.naming_addr = factory_data.get_actor();
        self.init(ctx);
    }
}

impl NamespaceActor {
    pub(crate) fn new() -> Self {
        Self {
            data: Default::default(),
            id_order_list: Default::default(),
            config_addr: None,
            naming_addr: None,
        }
    }

    fn init(&mut self, _ctx: &mut Context<Self>) {
        self.set_namespace(
            NamespaceParam {
                namespace_id: EMPTY_ARC_STRING.clone(),
                namespace_name: Some(DEFAULT_NAMESPACE.to_owned()),
                r#type: Some("0".to_owned()),
            },
            false,
        )
    }

    fn set_namespace(&mut self, param: NamespaceParam, add_if_not_exist: bool) {
        let value = if let Some(v) = self.data.get(&param.namespace_id) {
            if add_if_not_exist {
                return;
            }
            let mut value = Namespace::default();
            value.namespace_id = param.namespace_id;
            value.namespace_name = if let Some(name) = param.namespace_name {
                name
            } else {
                v.namespace_name.to_owned()
            };
            value.namespace_name = if let Some(r#type) = param.r#type {
                r#type
            } else {
                v.r#type.to_owned()
            };
            value
        } else {
            Namespace {
                namespace_id: param.namespace_id,
                namespace_name: param.namespace_name.unwrap_or_default(),
                r#type: param.r#type.unwrap_or("2".to_owned()),
            }
        };
        self.data
            .insert(value.namespace_id.clone(), Arc::new(value));
    }

    fn delete(&mut self, id: &Arc<String>) -> bool {
        self.data.remove(id).is_some()
    }
}

impl Handler<NamespaceRaftReq> for NamespaceActor {
    type Result = anyhow::Result<NamespaceRaftResult>;

    fn handle(&mut self, msg: NamespaceRaftReq, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            NamespaceRaftReq::AddIfNotExist(v) => {
                self.set_namespace(v, true);
                Ok(NamespaceRaftResult::None)
            }
            NamespaceRaftReq::Set(v) => {
                self.set_namespace(v, false);
                Ok(NamespaceRaftResult::None)
            }
            NamespaceRaftReq::Delete { id } => {
                self.delete(&id);
                Ok(NamespaceRaftResult::None)
            }
        }
    }
}
