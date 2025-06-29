use std::sync::Arc;

pub mod actor_model;

#[derive(Clone, Debug, Default)]
pub struct LdapConfig {
    pub ldap_url: Arc<String>,
    pub ldap_user_base_dn: Arc<String>,
    pub ldap_user_filter: Arc<String>,
}

#[derive(Clone, Debug)]
pub struct LdapUserParam {
    pub user_name: String,
    pub password: String,
    pub query_meta: bool,
}

#[derive(Clone, Debug)]
pub struct LdapUserBindParam {
    pub user_name: String,
    pub password: String,
}

#[derive(Clone, Debug)]
pub struct LdapUserMeta {
    pub user_name: String,
    pub groups: Vec<String>,
}

impl LdapUserMeta {
    pub fn new(user_name: String, groups: Vec<String>) -> Self {
        Self { user_name, groups }
    }
}
