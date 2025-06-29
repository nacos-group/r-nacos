use crate::ldap::ldap_conn::LdapConnActor;
use crate::ldap::model::LdapConfig;
use actix::prelude::*;
use actix_http::body::BodyLimitExceeded;
use std::sync::Arc;

pub struct LdapManager {
    ldap_config: Arc<LdapConfig>,
    ldap_conn: Option<Addr<LdapConnActor>>,
    enable_ldap: bool,
}

impl LdapManager {
    pub fn new(ldap_config: Arc<LdapConfig>, enable_ldap: bool) -> Self {
        Self {
            ldap_config,
            ldap_conn: None,
            enable_ldap,
        }
    }

    pub fn init(&mut self, ctx: &mut Context<Self>) {
        if self.enable_ldap {
            self.ldap_conn = Some(LdapConnActor::new(self.ldap_config.clone()).start());
        }
    }
}

impl Actor for LdapManager {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!("LdapManager started");
        self.init(ctx);
    }
}
