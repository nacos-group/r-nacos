use crate::ldap::ldap_msg_actor::LdapMsgActor;
use crate::ldap::model::actor_model::{LdapConnReq, LdapConnResult, LdapMsgActorReq};
use crate::ldap::model::LdapConfig;
use crate::user::UserManager;
use actix::prelude::*;
use ldap3::LdapConnAsync;
use std::sync::Arc;
use std::time::Duration;

pub struct LdapConnActor {
    ldap_config: Arc<LdapConfig>,
    ldap_msg_actor: Option<Addr<LdapMsgActor>>,
    user_manager_addr: Option<Addr<UserManager>>,
    stop: bool,
}

impl LdapConnActor {
    pub fn new(ldap_config: Arc<LdapConfig>, user_manager_addr: Option<Addr<UserManager>>) -> Self {
        Self {
            ldap_config,
            ldap_msg_actor: None,
            stop: false,
            user_manager_addr,
        }
    }

    async fn drive_conn(conn: LdapConnAsync) -> anyhow::Result<()> {
        if let Err(e) = conn.drive().await {
            log::warn!("LDAP connection error: {}", e);
        }
        Ok(())
    }

    fn init_conn(&mut self, ctx: &mut Context<Self>) {
        let ldap_url = self.ldap_config.ldap_url.clone();
        async move { LdapConnAsync::new(&ldap_url).await }
            .into_actor(self)
            .map(|result, act, ctx| match result {
                Ok((conn, ldap)) => {
                    if let Some(ldap_msg) = &act.ldap_msg_actor {
                        //更新ldap
                        ldap_msg.do_send(LdapMsgActorReq::SetLdap(ldap));
                    } else {
                        //创建ldap
                        act.ldap_msg_actor = Some(
                            LdapMsgActor::new(
                                ldap,
                                act.ldap_config.clone(),
                                act.user_manager_addr.clone(),
                            )
                            .start(),
                        );
                    }
                    log::info!("LDAP connection established");
                    Self::drive_conn(conn)
                        .into_actor(act)
                        .map(|_, act, ctx| {
                            log::warn!("LDAP connection closed");
                            if act.stop {
                                log::error!("LDAP connection stopped");
                                ctx.stop();
                            } else {
                                log::warn!("LDAP reconnecting");
                                ctx.run_later(Duration::from_millis(1000), |act, ctx| {
                                    act.init_conn(ctx);
                                });
                            }
                        })
                        .spawn(ctx);
                }
                Err(e) => {
                    log::error!("LDAP connection error: {}", e);
                    log::warn!("LDAP reconnecting");
                    ctx.run_later(Duration::from_millis(2000), |act, ctx| {
                        act.init_conn(ctx);
                    });
                }
            })
            .wait(ctx);
    }
}

impl Actor for LdapConnActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!("LdapConnActor started");
        self.init_conn(ctx);
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        self.stop = true;
        Running::Stop
    }
}

impl Handler<LdapConnReq> for LdapConnActor {
    type Result = anyhow::Result<LdapConnResult>;
    fn handle(&mut self, msg: LdapConnReq, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            LdapConnReq::GetMsgActor => {
                if let Some(addr) = self.ldap_msg_actor.as_ref() {
                    Ok(LdapConnResult::MsgActor(addr.clone()))
                } else {
                    Err(anyhow::anyhow!("ldap_msg_actor is None"))
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct LdapConnActorAddr {
    pub ldap_conn_actor: Addr<LdapConnActor>,
    pub ldap_msg_actor: Addr<LdapMsgActor>,
}

impl LdapConnActorAddr {
    pub fn new(ldap_conn_actor: Addr<LdapConnActor>, ldap_msg_actor: Addr<LdapMsgActor>) -> Self {
        Self {
            ldap_conn_actor,
            ldap_msg_actor,
        }
    }

    pub async fn new_from_config(
        config: Arc<LdapConfig>,
        user_manager_addr: Option<Addr<UserManager>>,
    ) -> anyhow::Result<Self> {
        let ldap_conn_actor = LdapConnActor::new(config, user_manager_addr).start();
        if let LdapConnResult::MsgActor(ldap_msg_actor) =
            ldap_conn_actor.send(LdapConnReq::GetMsgActor).await??
        {
            Ok(Self::new(ldap_conn_actor, ldap_msg_actor))
        } else {
            Err(anyhow::anyhow!("Get ldap msg actor error"))
        }
    }
}
