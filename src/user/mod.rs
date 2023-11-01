use std::sync::Arc;

use actix::prelude::*;
use bean_factory::{bean, Inject};
use inner_mem_cache::MemCache;

use crate::{
    now_millis,
    raft::db::{
        route::TableRoute,
        table::{TableManager, TableManagerReq},
    },
};

use self::model::UserDto;

pub mod model;

#[bean(inject)]
pub struct UserManager {
    cache: MemCache<Arc<String>, UserDto>,
    cache_sec: i32,
    raft_table_route: Option<Arc<TableRoute>>,
    table_name: Arc<String>,
}

impl UserManager {
    pub fn new() -> Self {
        Self {
            cache: MemCache::new(),
            cache_sec: 1200,
            raft_table_route: Default::default(),
            table_name: Arc::new("user".to_string()),
        }
    }
}

impl Inject for UserManager {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        factory_data: bean_factory::FactoryData,
        _factory: bean_factory::BeanFactory,
        _ctx: &mut Self::Context,
    ) {
        self.raft_table_route = factory_data.get_bean();
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

    fn handle(&mut self, msg: UserManagerReq, ctx: &mut Self::Context) -> Self::Result {
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
                let user_data = user.to_bytes();
                let raft_table_route = self.raft_table_route.clone();
                let cmd = TableManagerReq::Set {
                    table_name: self.table_name.clone(),
                    key: name.as_bytes().to_owned(),
                    value: user_data,
                    last_seq_id: None,
                };
                async move {
                    if let Some(raft_table_route) = raft_table_route {
                        raft_table_route.request(cmd).await.ok();
                    }
                    ()
                }
                .into_actor(self)
                .map(|_r, _act, _ctx| {})
                .wait(ctx);
                self.cache.set(name.clone(), user, self.cache_sec);
                Ok(UserManagerResult::None)
            }
            UserManagerReq::UpdateUser {
                name,
                nickname,
                password,
            } => {}
            UserManagerReq::CheckUser { name, password } => todo!(),
            UserManagerReq::Query { name } => todo!(),
        }
    }
}
