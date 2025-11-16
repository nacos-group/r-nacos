use crate::common::get_app_version;
use crate::oauth2::model::actor_model::{OAuth2MsgReq, OAuth2MsgResult};
use crate::oauth2::model::OAuth2Config;
use crate::user::model::{UserDto, UserSourceType};
use crate::user::{UserManager, UserManagerReq, UserManagerResult};
use actix::prelude::*;
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthorizationCode, ClientId, ClientSecret,
    RedirectUrl, Scope, TokenResponse,
};
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone)]
pub struct OAuth2MsgActor {
    oauth2_config: Arc<OAuth2Config>,
    user_manager_addr: Option<Addr<UserManager>>,
    http_client: Client,
    oauth2_client: BasicClient,
    scopes: Vec<Scope>,
}

impl OAuth2MsgActor {
    pub(crate) fn new(
        oauth2_config: Arc<OAuth2Config>,
        user_manager_addr: Option<Addr<UserManager>>,
    ) -> anyhow::Result<Self> {
        let client_id = ClientId::new(oauth2_config.oauth2_client_id.as_ref().clone());
        let client_secret = ClientSecret::new(oauth2_config.oauth2_client_secret.as_ref().clone());
        // Use full URLs directly
        let auth_url = oauth2_config.oauth2_authorization_url.as_ref().clone();
        let token_url = oauth2_config.oauth2_token_url.as_ref().clone();

        let redirect_url = RedirectUrl::new(oauth2_config.oauth2_redirect_uri.as_ref().clone())?;

        let oauth2_client = BasicClient::new(
            client_id,
            Some(client_secret),
            oauth2::AuthUrl::new(auth_url)?,
            Some(oauth2::TokenUrl::new(token_url)?),
        )
        .set_redirect_uri(redirect_url);

        // Parse scopes
        let scopes: Vec<Scope> = oauth2_config
            .oauth2_scopes
            .split_whitespace()
            .map(|s| Scope::new(s.to_string()))
            .collect();

        Ok(Self {
            oauth2_config,
            user_manager_addr,
            http_client: Client::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap_or_else(|_| Client::new()),
            oauth2_client,
            scopes,
        })
    }

    async fn handle_req(&self, msg: OAuth2MsgReq) -> anyhow::Result<OAuth2MsgResult> {
        match msg {
            OAuth2MsgReq::GetAuthorizeUrl => {
                let (auth_url, _csrf_state) = self
                    .oauth2_client
                    .authorize_url(oauth2::CsrfToken::new_random)
                    .add_scopes(self.scopes.clone())
                    .url();

                Ok(OAuth2MsgResult::AuthorizeUrl(auth_url.to_string()))
            }
            OAuth2MsgReq::Authenticate(param) => {
                // Exchange authorization code for token
                let token = self
                    .oauth2_client
                    .exchange_code(AuthorizationCode::new(param.code.clone()))
                    .request_async(async_http_client)
                    .await
                    .map_err(|e| {
                        log::error!("OAuth2 token exchange failed: {}", e);
                        anyhow::anyhow!("OAuth2 token exchange failed: {}", e)
                    })?;

                let access_token = token.access_token().secret();

                // Get user info - use full URL directly
                let userinfo_url = self.oauth2_config.oauth2_userinfo_url.as_ref().clone();

                // Build request with bearer token
                // Use bearer_auth which sets: Authorization: Bearer <token>
                // GitHub and some OAuth2 providers require User-Agent header
                let user_agent = format!("r-nacos/{}", get_app_version());
                let request = self
                    .http_client
                    .get(&userinfo_url)
                    .bearer_auth(access_token.clone())
                    .header("Accept", "application/json")
                    .header("User-Agent", user_agent);

                let userinfo_response = request.send().await?;

                // Check response status
                let status = userinfo_response.status();
                if !status.is_success() {
                    let error_text = userinfo_response.text().await.unwrap_or_default();
                    log::error!(
                        "OAuth2 userinfo request failed: status={}, url={}, body={}",
                        status,
                        userinfo_url,
                        error_text
                    );
                    return Err(anyhow::anyhow!(
                        "OAuth2 userinfo request failed: status={}, body={}",
                        status,
                        error_text
                    ));
                }

                // Parse JSON directly
                let userinfo: Value = userinfo_response.json().await.map_err(|e| {
                    log::error!("OAuth2 userinfo JSON parse error: {}", e);
                    anyhow::anyhow!("Failed to parse OAuth2 userinfo response: {}", e)
                })?;

                // Log the complete userinfo response for debugging
                log::info!(
                    "OAuth2 userinfo response: {}",
                    serde_json::to_string_pretty(&userinfo)
                        .unwrap_or_else(|_| "Failed to serialize".to_string())
                );

                // Extract username from claims
                let username_claim_name = self.oauth2_config.oauth2_username_claim_name.as_ref();
                let user_name = userinfo
                    .get(username_claim_name)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        log::error!("OAuth2 username claim '{}' not found", username_claim_name);
                        anyhow::anyhow!("username claim '{}' not found", username_claim_name)
                    })?
                    .to_string();

                // Extract nickname from claims
                let nickname_claim_name = self.oauth2_config.oauth2_nickname_claim_name.as_ref();
                let nickname = userinfo
                    .get(nickname_claim_name)
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .ok_or_else(|| {
                        log::error!("OAuth2 nickname claim '{}' not found", nickname_claim_name);
                        anyhow::anyhow!("nickname claim '{}' not found", nickname_claim_name)
                    })?;

                // Use configured default role
                let role = self.oauth2_config.oauth2_user_default_role.clone();
                let mut namespace_privilege = None;

                // Initialize user
                if let Some(user_manager_addr) = &self.user_manager_addr {
                    let user = UserDto {
                        username: Arc::new(user_name.clone()),
                        nickname: Some(nickname),
                        source: Some(UserSourceType::Inner.to_str().to_owned()),
                        roles: Some(vec![role.clone()]),
                        ..Default::default()
                    };
                    if let Ok(Ok(UserManagerResult::QueryUser(Some(user_dto)))) = user_manager_addr
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
                    crate::oauth2::model::OAuth2UserMeta::new(user_name, role, namespace_privilege);
                Ok(OAuth2MsgResult::UserMeta(meta))
            }
        }
    }
}

impl Actor for OAuth2MsgActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("OAuth2MsgActor started");
    }
}

impl Handler<OAuth2MsgReq> for OAuth2MsgActor {
    type Result = ResponseActFuture<Self, anyhow::Result<OAuth2MsgResult>>;

    fn handle(&mut self, msg: OAuth2MsgReq, _ctx: &mut Self::Context) -> Self::Result {
        let actor = self.clone();
        let fut = async move { actor.handle_req(msg).await }
            .into_actor(self)
            .map(|result, _act, _ctx| result);
        Box::pin(fut)
    }
}
