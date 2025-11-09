use crate::common::model::privilege::PrivilegeGroup;
use std::sync::Arc;

pub mod actor_model;

#[derive(Clone, Debug, Default)]
pub struct OAuth2Config {
    pub oauth2_server_url: Arc<String>,
    pub oauth2_client_id: Arc<String>,
    pub oauth2_client_secret: Arc<String>,
    pub oauth2_authorization_url: Arc<String>,
    pub oauth2_token_url: Arc<String>,
    pub oauth2_userinfo_url: Arc<String>,
    pub oauth2_redirect_uri: Arc<String>,
    pub oauth2_scopes: Arc<String>,
    pub oauth2_username_claim_name: Arc<String>,
    pub oauth2_nickname_claim_name: Arc<String>,
    pub oauth2_user_default_role: Arc<String>,
}

#[derive(Clone, Debug)]
pub struct OAuth2UserParam {
    pub code: String,
    pub state: Option<String>,
}

#[derive(Clone, Debug)]
pub struct OAuth2UserMeta {
    pub user_name: String,
    pub role: Arc<String>,
    pub namespace_privilege: Option<PrivilegeGroup<Arc<String>>>,
}

impl OAuth2UserMeta {
    pub fn new(
        user_name: String,
        role: Arc<String>,
        namespace_privilege: Option<PrivilegeGroup<Arc<String>>>,
    ) -> Self {
        Self {
            user_name,
            role,
            namespace_privilege,
        }
    }
}

