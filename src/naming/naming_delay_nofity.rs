
use std::time::Duration;

use actix::prelude::*;

use crate::{common::delay_notify::{NotifyEvent, DelayNotify}, grpc::bistream_manage::BiStreamManage};

use super::model::{ServiceKey, ServiceInfo};

#[derive(Clone,Default)]
pub struct NamingDelayEvent {
    pub key:ServiceKey,
    pub data:ServiceInfo,
    pub conn_manage: Option<Addr<BiStreamManage>>,
}

impl NotifyEvent for NamingDelayEvent {
    fn on_event(&self) -> anyhow::Result<()> {
        if let Some(conn) = self.conn_manage.as_ref() {

        }
        Ok(())
    }

    fn merge(&mut self,other:Self) -> anyhow::Result<()> {
        self.data = other.data;
        Ok(())
    }
}

pub struct DelayNotifyActor {
    inner_delay_notify: DelayNotify<ServiceKey,NamingDelayEvent>,
}

impl DelayNotifyActor {
    pub fn new() -> Self{
        DelayNotifyActor {
            inner_delay_notify: Default::default()
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
    AddEvent(NamingDelayEvent,u64),
}

pub enum DelayNotifyResult {
    None,
}

impl Handler<DelayNotifyCmd> for DelayNotifyActor {
    type Result = anyhow::Result<DelayNotifyResult>;

    fn handle(&mut self, msg: DelayNotifyCmd, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            DelayNotifyCmd::AddEvent(event,delay) => {
                self.inner_delay_notify.add_event(delay, event.key.clone(), event)?;
            },
        }
        Ok(DelayNotifyResult::None)
    }
}