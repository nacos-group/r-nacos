
use std::{time::Duration, collections::HashSet, sync::Arc};

use actix::prelude::*;

use crate::{common::delay_notify::{NotifyEvent, DelayNotify}, grpc::bistream_manage::{BiStreamManage, BiStreamManageCmd}};

use super::model::{ServiceKey, ServiceInfo};

#[derive(Clone,Default)]
pub struct NamingDelayEvent {
    pub key:ServiceKey,
    pub client_id_set: HashSet<Arc<String>>,
    pub service_info:ServiceInfo,
    pub conn_manage: Option<Addr<BiStreamManage>>,
}

impl NotifyEvent for NamingDelayEvent {
    fn on_event(self) -> anyhow::Result<()> {
        if let Some(conn_manage) = self.conn_manage.as_ref() {
            conn_manage.do_send(BiStreamManageCmd::NotifyNaming(self.key, self.client_id_set,self.service_info));
        }
        Ok(())
    }

    fn merge(&mut self,other:Self) -> anyhow::Result<()> {
        self.service_info = other.service_info;
        self.client_id_set = other.client_id_set;
        self.conn_manage = other.conn_manage;
        Ok(())
    }
}

pub struct DelayNotifyActor {
    inner_delay_notify: DelayNotify<ServiceKey,NamingDelayEvent>,
    conn_manage: Option<Addr<BiStreamManage>>,
    delay:u64,
}

impl DelayNotifyActor {
    pub fn new() -> Self{
        DelayNotifyActor {
            inner_delay_notify: Default::default(),
            conn_manage: None,
            delay:500,
        }
    }

    pub fn notify_heartbeat(&self,ctx:&mut actix::Context<Self>) {
        ctx.run_later(Duration::from_millis(500), |act,ctx|{
            act.inner_delay_notify.notify_timeout().unwrap();
            act.notify_heartbeat(ctx);
        });
    }
}

impl Actor for DelayNotifyActor {
    type Context = Context<Self>;

    fn started(&mut self,ctx: &mut Self::Context) {
        log::info!(" DelayNotifyActor started");
        //self.heartbeat(ctx);
    }
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<DelayNotifyResult>")]
pub enum DelayNotifyCmd {
    SetConnManageAddr(Addr<BiStreamManage>),
    Notify(ServiceKey,HashSet<Arc<String>>,ServiceInfo),
}

pub enum DelayNotifyResult {
    None,
}

impl Handler<DelayNotifyCmd> for DelayNotifyActor {
    type Result = anyhow::Result<DelayNotifyResult>;

    fn handle(&mut self, msg: DelayNotifyCmd, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            DelayNotifyCmd::Notify(key,client_id_set,service_info) => {
                let event = NamingDelayEvent {
                    key,
                    client_id_set,
                    service_info,
                    conn_manage:self.conn_manage.to_owned(),
                };
                self.inner_delay_notify.add_event(self.delay, event.key.clone(), event)?;
            },
            DelayNotifyCmd::SetConnManageAddr(conn_manage) => {
                self.conn_manage = Some(conn_manage);
            },
        }
        Ok(DelayNotifyResult::None)
    }
}