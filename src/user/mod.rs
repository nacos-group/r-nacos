use std::sync::Arc;

use actix::prelude::*;
use bean_factory::{bean, Inject};
use inner_mem_cache::MemCache;

use crate::{
    now_millis,
    raft::db::{
        route::TableRoute,
        table::{TableManagerQueryReq, TableManagerReq, TableManagerResult},
    },
};

use self::model::UserDto;

pub mod model;

lazy_static::lazy_static! {
    static ref USER_TABLE_NAME: Arc<String> =  Arc::new("user".to_string());
}
#[bean(inject)]
pub struct UserManager {
    cache: MemCache<Arc<String>, Arc<UserDto>>,
    cache_sec: i32,
    raft_table_route: Option<Arc<TableRoute>>,
}

impl UserManager {
    pub fn new() -> Self {
        Self {
            cache: MemCache::new(),
            cache_sec: 1200,
            raft_table_route: Default::default(),
        }
    }

    fn update_timeout(&mut self, key: &Arc<String>) {
        self.cache.update_time_out(key, self.cache_sec)
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

pub enum UserManagerInnerCtx {
    UpdateUser { key: Arc<String>, value: UserDto },
    CheckUserResult(Arc<String>, bool),
    QueryUser(Arc<String>),
}

pub enum UserManagerResult {
    None,
    CheckUserResult(bool),
    QueryUser(Option<Arc<UserDto>>),
}

impl Handler<UserManagerReq> for UserManager {
    type Result = ResponseActFuture<Self, anyhow::Result<UserManagerResult>>;

    fn handle(&mut self, msg: UserManagerReq, _ctx: &mut Self::Context) -> Self::Result {
        let raft_table_route = self.raft_table_route.clone();
        let fut = async move {
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
                    let req = TableManagerReq::Set {
                        table_name: USER_TABLE_NAME.clone(),
                        key: name.as_bytes().to_owned(),
                        value: user_data,
                        last_seq_id: None,
                    };
                    if let Some(raft_table_route) = raft_table_route {
                        raft_table_route.request(req).await.ok();
                    }
                    Ok(UserManagerInnerCtx::UpdateUser {
                        key: name,
                        value: user,
                    })
                }
                UserManagerReq::UpdateUser {
                    name,
                    nickname,
                    password,
                } => {
                    let mut last_user = if let Some(raft_table_route) = &raft_table_route {
                        let query_req = TableManagerQueryReq::GetByArcKey {
                            table_name: USER_TABLE_NAME.clone(),
                            key: name.clone(),
                        };
                        match raft_table_route.get_leader_data(query_req).await? {
                            TableManagerResult::Value(old_value) => {
                                UserDto::from_bytes(&old_value)?
                            }
                            _ => return Err(anyhow::anyhow!("not found user {}", &name)),
                        }
                    } else {
                        return Err(anyhow::anyhow!("raft_table_route is none "));
                    };
                    if let Some(nickname) = nickname {
                        last_user.nickname = nickname;
                    }
                    if let Some(password) = password {
                        last_user.password = password;
                    }
                    let user_data = last_user.to_bytes();
                    let req = TableManagerReq::Set {
                        table_name: USER_TABLE_NAME.clone(),
                        key: name.as_bytes().to_owned(),
                        value: user_data,
                        last_seq_id: None,
                    };
                    if let Some(raft_table_route) = raft_table_route {
                        raft_table_route.request(req).await.ok();
                    }
                    Ok(UserManagerInnerCtx::UpdateUser {
                        key: name,
                        value: last_user,
                    })
                }
                UserManagerReq::CheckUser { name, password } => {
                    let last_user = if let Some(raft_table_route) = &raft_table_route {
                        let query_req = TableManagerQueryReq::GetByArcKey {
                            table_name: USER_TABLE_NAME.clone(),
                            key: name.clone(),
                        };
                        match raft_table_route.get_leader_data(query_req).await? {
                            TableManagerResult::Value(old_value) => {
                                UserDto::from_bytes(&old_value)?
                            }
                            _ => return Err(anyhow::anyhow!("not found user {}", &name)),
                        }
                    } else {
                        return Err(anyhow::anyhow!("raft_table_route is none "));
                    };
                    Ok(UserManagerInnerCtx::CheckUserResult(
                        name,
                        last_user.password == password,
                    ))
                }
                UserManagerReq::Query { name } => Ok(UserManagerInnerCtx::QueryUser(name)),
            }
        }
        .into_actor(self)
        .map(
            |r: anyhow::Result<UserManagerInnerCtx>, act, _ctx| match r? {
                UserManagerInnerCtx::UpdateUser { key, value } => {
                    act.cache.set(key, Arc::new(value), act.cache_sec);
                    Ok(UserManagerResult::None)
                }
                UserManagerInnerCtx::CheckUserResult(key, v) => {
                    act.update_timeout(&key);
                    Ok(UserManagerResult::CheckUserResult(v))
                }
                UserManagerInnerCtx::QueryUser(key) => {
                    act.update_timeout(&key);
                    let r = act.cache.get(&key).ok();
                    Ok(UserManagerResult::QueryUser(r))
                }
            },
        );
        Box::pin(fut)
    }
}
