use crate::ldap::model::{LdapUserBindParam, LdapUserMeta, LdapUserParam};
use actix::prelude::*;

#[derive(Clone, Debug, Message)]
#[rtype(result = "anyhow::Result<LdapMsgResult>")]
pub enum LdapConnReq {
    Bind(LdapUserParam),
}

#[derive(Clone, Debug, Message)]
#[rtype(result = "anyhow::Result<LdapMsgResult>")]
pub enum LdapManagerReq {
    Auth(LdapUserParam),
}

pub enum LdapMsgResult {
    None,
    UserMeta(LdapUserMeta),
}
