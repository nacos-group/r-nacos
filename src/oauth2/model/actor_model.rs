use crate::oauth2::model::{OAuth2UserMeta, OAuth2UserParam};
use actix::prelude::*;

#[derive(Clone, Debug, Message)]
#[rtype(result = "anyhow::Result<OAuth2MsgResult>")]
pub enum OAuth2MsgReq {
    Authenticate(OAuth2UserParam),
    GetAuthorizeUrl,
}

#[derive(Clone, Debug)]
pub enum OAuth2MsgResult {
    None,
    UserMeta(OAuth2UserMeta),
    AuthorizeUrl(String),
}
