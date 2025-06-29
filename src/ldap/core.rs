use crate::ldap::ldap_conn::LdapConnActorAddr;
use crate::ldap::ldap_msg_actor::LdapMsgActor;
use crate::ldap::model::actor_model::{LdapMsgReq, LdapMsgResult};
use crate::ldap::model::LdapConfig;
use actix::prelude::*;
use bean_factory::{bean, BeanFactory, FactoryData, Inject};
use std::sync::Arc;

#[bean(inject)]
pub struct LdapManager {
    ldap_config: Arc<LdapConfig>,
    ldap_conn_addr: Option<LdapConnActorAddr>,
    enable_ldap: bool,
}

impl LdapManager {
    pub fn new(ldap_config: Arc<LdapConfig>, enable_ldap: bool) -> Self {
        Self {
            ldap_config,
            ldap_conn_addr: None,
            enable_ldap,
        }
    }

    pub fn init(&mut self, ctx: &mut Context<Self>) {
        if !self.enable_ldap {
            return;
        }
        let ldap_config = self.ldap_config.clone();
        async move { LdapConnActorAddr::new_from_config(ldap_config).await }
            .into_actor(self)
            .map(|res, act, ctx| match res {
                Ok(res) => {
                    act.ldap_conn_addr = Some(res);
                }
                Err(err) => {
                    log::error!("init ldap error:{}", err);
                }
            })
            .wait(ctx);
    }

    async fn handle_req(
        req: LdapMsgReq,
        ldap_msg_addr: Option<Addr<LdapMsgActor>>,
    ) -> anyhow::Result<LdapMsgResult> {
        let ldap_msg_addr = if let Some(v) = ldap_msg_addr {
            v
        } else {
            return Err(anyhow::anyhow!("ldap_msg_addr is None"));
        };
        ldap_msg_addr.send(req).await?
    }
}

impl Actor for LdapManager {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!("LdapManager started");
        self.init(ctx);
    }
}

impl Inject for LdapManager {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        _factory_data: FactoryData,
        _factory: BeanFactory,
        _ctx: &mut Self::Context,
    ) {
    }
}

impl Handler<LdapMsgReq> for LdapManager {
    type Result = ResponseActFuture<Self, anyhow::Result<LdapMsgResult>>;

    fn handle(&mut self, msg: LdapMsgReq, _ctx: &mut Self::Context) -> Self::Result {
        let msg_addr = self
            .ldap_conn_addr
            .as_ref()
            .map(|v| v.ldap_msg_actor.clone());
        let fut = Self::handle_req(msg, msg_addr)
            .into_actor(self)
            .map(|result, act, ctx| result);
        Box::pin(fut)
    }
}
