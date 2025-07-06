use crate::common::string_utils::StringUtils;
use crate::ldap::model::actor_model::{LdapMsgActorReq, LdapMsgReq, LdapMsgResult};
use crate::ldap::model::{LdapConfig, LdapUserMeta};
use crate::user::model::{UserDto, UserSourceType};
use crate::user::{permission, UserManager, UserManagerReq, UserManagerResult};
use actix::prelude::*;
use ldap3::{Ldap, SearchEntry};
use std::sync::Arc;

pub struct LdapMsgActor {
    ldap: Option<Ldap>,
    ldap_version: u64,
    /// ldap备份；ldap每次使用时使用值传递，减少clone次数；ldap_backup用于并发时ldap为None后可以使用；
    ldap_backup: Option<Ldap>,
    ldap_config: Arc<LdapConfig>,
    user_manager_addr: Option<Addr<UserManager>>,
}

impl LdapMsgActor {
    pub(crate) fn new(
        ldap: Ldap,
        ldap_config: Arc<LdapConfig>,
        user_manager_addr: Option<Addr<UserManager>>,
    ) -> Self {
        let ldap_backup = Some(ldap.clone());
        Self {
            ldap: Some(ldap),
            ldap_version: 0,
            ldap_backup,
            ldap_config,
            user_manager_addr,
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
        user_manager_addr: Option<Addr<UserManager>>,
    ) -> anyhow::Result<LdapMsgResult> {
        match msg {
            LdapMsgReq::Bind(bind_req) => {
                let bind_dn = format!(
                    "cn={},{}",
                    bind_req.user_name, &ldap_config.ldap_user_base_dn
                );
                ldap.simple_bind(&bind_dn, &bind_req.password)
                    .await?
                    .success()?;
                let filter = ldap_config
                    .ldap_user_filter
                    .replace("%s", &bind_req.user_name);
                let (mut rs, _res) = ldap
                    .search(
                        &ldap_config.ldap_user_base_dn,
                        ldap3::Scope::Subtree,
                        &filter,
                        vec!["cn", "memberOf"],
                    )
                    .await?
                    .success()?;
                if !rs.is_empty() {
                    let entry = rs.remove(0);
                    let mut entry = SearchEntry::construct(entry);
                    let mut role = ldap_config.ldap_user_default_role.clone();
                    let mut namespace_privilege = None;
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
                    if let Some(user_manager_addr) = user_manager_addr {
                        //init user
                        let user = UserDto {
                            username: Arc::new(bind_req.user_name.clone()),
                            nickname: Some(bind_req.user_name.clone()),
                            source: Some(UserSourceType::Inner.to_str().to_owned()),
                            roles: Some(vec![role.clone()]),
                            ..Default::default()
                        };
                        if let Ok(Ok(UserManagerResult::QueryUser(Some(user_dto)))) =
                            user_manager_addr
                                .send(UserManagerReq::InitUser {
                                    user,
                                    namespace_privilege_param: None,
                                })
                                .await
                        {
                            namespace_privilege = user_dto.namespace_privilege;
                        }
                    }
                    let meta =
                        LdapUserMeta::new(bind_req.user_name, groups, role, namespace_privilege);
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

    fn handle(&mut self, msg: LdapMsgReq, ctx: &mut Self::Context) -> Self::Result {
        let (tx, rx) = tokio::sync::oneshot::channel();

        let mut ldap = self.ldap.take();
        if ldap.is_none() {
            ldap = self.ldap_backup.clone();
        }
        let version = self.ldap_version;
        let config = self.ldap_config.clone();
        let user_manager_addr = self.user_manager_addr.clone();
        async move {
            let r = if let Some(ldap) = ldap.as_mut() {
                Self::handle_req(ldap, config, msg, user_manager_addr).await
            } else {
                Err(anyhow::anyhow!("ldap not init"))
            };
            (ldap, version, tx, r)
        }
        .into_actor(self)
        .map(|(ldap, version, tx, r), act, _ctx| {
            if act.ldap_version == version {
                act.ldap = ldap;
            }
            tx.send(r).ok();
        })
        .wait(ctx); //使用wait执行，保证handle_req内能依次执行，避免出现异步插队可能引发的问题。

        let fut = rx.into_actor(self).map(|r, _act, _ctx| match r {
            Ok(v) => v,
            Err(r) => Err(anyhow::anyhow!(r.to_string())),
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
