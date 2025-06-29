use crate::ldap::model::actor_model::LdapConnReq;
use crate::ldap::model::{LdapConfig, LdapUserParam};
use actix::prelude::*;
use futures_util::task::SpawnExt;
use futures_util::StreamExt;
use ldap3::{Ldap, LdapConnAsync};
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;

pub struct LdapConnActor {
    ldap_config: Arc<LdapConfig>,
    ldap_inner: Option<Ldap>,
    stop: bool,
}

impl LdapConnActor {
    pub fn new(ldap_config: Arc<LdapConfig>) -> Self {
        Self {
            ldap_config,
            ldap_inner: None,
            stop: false,
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
                    act.ldap_inner = Some(ldap);
                    log::info!("LDAP connection established");
                    Self::drive_conn(conn)
                        .into_actor(act)
                        .map(|_, act, ctx| {
                            log::info!("LDAP connection closed");
                            if act.stop {
                                log::error!("LDAP connection stopped");
                                ctx.stop();
                            } else {
                                log::warn!("LDAP reconnecting");
                                act.ldap_inner = None;
                                act.init_conn(ctx);
                            }
                        })
                        .spawn(ctx);
                }
                Err(e) => {
                    log::error!("LDAP connection error: {}", e);
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

    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        self.stop = true;
        Running::Stop
    }
}
