use std::sync::Arc;

use actix::prelude::*;
use inner_mem_cache::MemCache;

use crate::now_millis;

use self::model::UserDto;

pub mod model;

pub struct UserManager {
    cache: MemCache<Arc<String>, UserDto>,
    cache_sec: i32,
}

impl UserManager {
    pub fn new() -> Self {
        Self {
            cache: MemCache::new(),
            cache_sec: 1200,
        }
    }
}

impl Actor for UserManager {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("UserManager started")
    }
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<UserManagerResult>")]
pub enum UserManagerReq {
    AddUser {
        name: Arc<String>,
        nickname: String,
        password: String,
    },
    UpdateUser {
        name: Arc<String>,
        nickname: Option<String>,
        password: Option<String>,
    },
    CheckUser {
        name: Arc<String>,
        password: String,
    },
    Query {
        name: Arc<String>,
    },
    //分页查询用户列表
}

pub enum UserManagerResult {
    None,
    User(UserDto),
}

impl Handler<UserManagerReq> for UserManager {
    type Result = anyhow::Result<UserManagerResult>;

    fn handle(&mut self, msg: UserManagerReq, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            UserManagerReq::AddUser {
                name,
                nickname,
                password,
            } => {
                let now = (now_millis() / 1000) as u32;
                let user = UserDto {
                    password,
                    nickname,
                    gmt_create: now,
                    gmt_modified: now,
                };
                //let user_data = user.to_bytes();
                //todo save user
                self.cache.set(name.clone(), user, self.cache_sec);
                Ok(UserManagerResult::None)
            }
            UserManagerReq::UpdateUser {
                name,
                nickname,
                password,
            } => todo!(),
            UserManagerReq::CheckUser { name, password } => todo!(),
            UserManagerReq::Query { name } => todo!(),
        }
    }
}
