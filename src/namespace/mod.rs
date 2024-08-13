use crate::config::core::ConfigActor;
use crate::naming::core::NamingActor;
use actix::prelude::*;
use bean_factory::{bean, BeanFactory, FactoryData, Inject};
use std::collections::{HashMap, LinkedList};
use std::sync::Arc;

pub struct Namespace {
    pub namespace_id: Option<String>,
    pub namespace_name: Option<String>,
    pub r#type: Option<String>,
}

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
        todo!();
    }
}
