use crate::common::model::privilege::PrivilegeGroup;
use std::collections::HashSet;
use std::sync::Arc;

pub mod actor_model;

#[derive(Clone, Debug, Default)]
pub struct LdapConfig {
    pub ldap_url: Arc<String>,
    pub ldap_user_base_dn: Arc<String>,
    pub ldap_user_filter: Arc<String>,
    pub ldap_user_developer_groups: Arc<HashSet<String>>,
    pub ldap_user_admin_groups: Arc<HashSet<String>>,
    pub ldap_user_default_role: Arc<String>,
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
    pub role: Arc<String>,
    pub namespace_privilege: Option<PrivilegeGroup<Arc<String>>>,
}

impl LdapUserMeta {
    pub fn new(
        user_name: String,
        groups: Vec<String>,
        role: Arc<String>,
        namespace_privilege: Option<PrivilegeGroup<Arc<String>>>,
    ) -> Self {
        Self {
            user_name,
            groups,
            role,
            namespace_privilege,
        }
    }
}
