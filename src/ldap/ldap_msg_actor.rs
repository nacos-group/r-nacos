use crate::common::string_utils::StringUtils;
use crate::ldap::model::actor_model::{LdapMsgActorReq, LdapMsgReq, LdapMsgResult};
use crate::ldap::model::{LdapConfig, LdapUserMeta};
use crate::user::permission;
use actix::prelude::*;
use ldap3::{Ldap, SearchEntry};
use std::sync::Arc;

pub struct LdapMsgActor {
    ldap: Option<Ldap>,
    ldap_version: u64,
    /// ldap备份；ldap每次使用时使用值传递，减少clone次数；ldap_backup用于并发时ldap为None后可以使用；
    ldap_backup: Option<Ldap>,
    ldap_config: Arc<LdapConfig>,
}

impl LdapMsgActor {
    pub(crate) fn new(ldap: Ldap, ldap_config: Arc<LdapConfig>) -> Self {
        let ldap_backup = Some(ldap.clone());
        Self {
            ldap: Some(ldap),
            ldap_version: 0,
            ldap_backup,
            ldap_config,
        }
    }

    fn set_ldap(&mut self, ldap: Ldap) {
        self.ldap_backup = Some(ldap.clone());
        self.ldap = Some(ldap);
        self.ldap_version += 1;
    }

    async fn handle_req(
        ldap: &mut Ldap,
        ldap_config: Arc<LdapConfig>,
        msg: LdapMsgReq,
    ) -> anyhow::Result<LdapMsgResult> {
        match msg {
            LdapMsgReq::Bind(bind_req) => {
                let bind_dn = format!(
                    "cn={},{}",
                    bind_req.user_name, &ldap_config.ldap_user_base_dn
                );
                let bind_result = ldap
                    .simple_bind(&bind_dn, &bind_req.password)
                    .await?
                    .success()?;
                let filter = format!(
                    "(&{}(cn={}))",
                    ldap_config.ldap_user_filter, &bind_req.user_name
                );
                let (mut rs, _res) = ldap
                    .search(
                        &ldap_config.ldap_user_base_dn,
                        ldap3::Scope::Subtree,
                        &filter,
                        vec!["cn", "memberOf"],
                    )
                    .await?
                    .success()?;
                if rs.len() > 0 {
                    let entry = rs.remove(0);
                    let mut entry = SearchEntry::construct(entry);
                    let mut role = ldap_config.ldap_user_default_role.clone();
                    let groups = entry.attrs.remove("memberOf").unwrap_or_default();
                    let groups = groups
                        .iter()
                        .map(|s| StringUtils::extract_ldap_value_cn(s))
                        .filter(|s| s.is_some())
                        .map(|s| s.unwrap())
                        .collect::<Vec<_>>();
                    for group in groups.iter() {
                        if ldap_config.ldap_user_developer_groups.contains(group) {
                            role = permission::USER_ROLE_DEVELOPER.clone();
                            break;
                        }
                    }
                    for group in groups.iter() {
                        if ldap_config.ldap_user_admin_groups.contains(group) {
                            role = permission::USER_ROLE_MANAGER.clone();
                            break;
                        }
                    }
                    let meta = LdapUserMeta::new(bind_req.user_name, groups, role);
                    Ok(LdapMsgResult::UserMeta(meta))
                } else {
                    Err(anyhow::anyhow!("search user result is empty"))
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

impl Handler<LdapMsgReq> for LdapMsgActor {
    type Result = ResponseActFuture<Self, anyhow::Result<LdapMsgResult>>;

    fn handle(&mut self, msg: LdapMsgReq, _ctx: &mut Self::Context) -> Self::Result {
        let mut ldap = self.ldap.take();
        if ldap.is_none() {
            ldap = self.ldap_backup.clone();
        }
        let version = self.ldap_version;
        let config = self.ldap_config.clone();
        let fut = async move {
            let r = if let Some(ldap) = ldap.as_mut() {
                Self::handle_req(ldap, config, msg).await
            } else {
                Err(anyhow::anyhow!("ldap not init"))
            };
            (ldap, version, r)
        }
        .into_actor(self)
        .map(|(ldap, version, r), act, ctx| {
            if act.ldap_version == version {
                act.ldap = ldap;
            }
            r
        });
        Box::pin(fut)
    }
}

impl Handler<LdapMsgActorReq> for LdapMsgActor {
    type Result = anyhow::Result<()>;
    fn handle(&mut self, msg: LdapMsgActorReq, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            LdapMsgActorReq::SetLdap(ldap) => {
                self.set_ldap(ldap);
            }
        }
        Ok(())
    }
}
