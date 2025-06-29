use crate::ldap::model::actor_model::{LdapConnReq, LdapMsgResult};
use crate::ldap::model::{LdapConfig, LdapUserMeta};
use actix::prelude::*;
use ldap3::{Ldap, SearchEntry};
use std::sync::Arc;

pub struct LdapMsgActor {
    ldap: Option<Ldap>,
    ldap_config: Arc<LdapConfig>,
}

impl LdapMsgActor {
    fn new(ldap: Ldap, ldap_config: Arc<LdapConfig>) -> Self {
        Self {
            ldap: Some(ldap),
            ldap_config,
        }
    }

    async fn handle_req(
        ldap: &mut Ldap,
        ldap_config: Arc<LdapConfig>,
        msg: LdapConnReq,
    ) -> anyhow::Result<LdapMsgResult> {
        match msg {
            LdapConnReq::Bind(bind_req) => {
                let bind_dn = format!(
                    "cn={},{}",
                    bind_req.user_name, &ldap_config.ldap_user_base_dn
                );
                let bind_result = ldap
                    .simple_bind(&bind_dn, &bind_req.password)
                    .await?
                    .success()?;
                let (mut rs, _res) = ldap
                    .search(
                        &ldap_config.ldap_user_base_dn,
                        ldap3::Scope::Subtree,
                        &format!(
                            "(&{}(cn={})",
                            ldap_config.ldap_user_filter, &bind_req.user_name
                        ),
                        vec!["cn", "memberOf"],
                    )
                    .await?
                    .success()?;
                if rs.len() > 0 {
                    let entry = rs.remove(0);
                    let mut entry = SearchEntry::construct(entry);
                    let groups = entry.attrs.remove("memberOf").unwrap_or_default();
                    let meta = LdapUserMeta::new(bind_req.user_name, groups);
                    Ok(LdapMsgResult::UserMeta(meta))
                } else {
                    Err(anyhow::anyhow!("no user"))
                }
            }
        }
    }
}

impl Actor for LdapMsgActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("LdapMsgActor started");
    }
}

impl Handler<LdapConnReq> for LdapMsgActor {
    type Result = ResponseActFuture<Self, anyhow::Result<LdapMsgResult>>;

    fn handle(&mut self, msg: LdapConnReq, _ctx: &mut Self::Context) -> Self::Result {
        let mut ldap = self.ldap.take();
        let config = self.ldap_config.clone();
        let fut = async move {
            let r = if let Some(ldap) = ldap.as_mut() {
                Self::handle_req(ldap, config, msg).await
            } else {
                Err(anyhow::anyhow!("ldap not init"))
            };
            (ldap, r)
        }
        .into_actor(self)
        .map(|(ldap, r), act, ctx| {
            act.ldap = ldap;
            r
        });
        Box::pin(fut)
    }
}
