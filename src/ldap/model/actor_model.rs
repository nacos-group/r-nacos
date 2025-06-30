use crate::ldap::ldap_msg_actor::LdapMsgActor;
use crate::ldap::model::{LdapUserMeta, LdapUserParam};
use actix::prelude::*;
use ldap3::Ldap;

#[derive(Clone, Debug, Message)]
#[rtype(result = "anyhow::Result<LdapMsgResult>")]
pub enum LdapMsgReq {
    Bind(LdapUserParam),
}

#[derive(Clone, Debug, Message)]
#[rtype(result = "anyhow::Result<()>")]
pub enum LdapMsgActorReq {
    SetLdap(Ldap),
}

#[derive(Clone, Debug, Message)]
#[rtype(result = "anyhow::Result<LdapConnResult>")]
pub enum LdapConnReq {
    GetMsgActor,
}

pub enum LdapConnResult {
    None,
    MsgActor(Addr<LdapMsgActor>),
}

#[derive(Clone, Debug, Message)]
#[rtype(result = "anyhow::Result<LdapMsgResult>")]
pub enum LdapManagerReq {
    Auth(LdapUserParam),
}

#[derive(Clone, Debug)]
pub enum LdapMsgResult {
    None,
    UserMeta(LdapUserMeta),
}
