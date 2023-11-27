use std::{sync::Arc, time::Duration};

use actix::prelude::*;
use bean_factory::{bean, Inject};
//use inner_mem_cache::MemCache;

use crate::{
    now_millis,
    raft::{
        cluster::{model::RouteAddr, route::RaftAddrRouter},
        db::{
            route::TableRoute,
            table::{TableManager, TableManagerQueryReq, TableManagerReq, TableManagerResult},
        },
    },
};

use self::model::UserDto;

pub mod api;
pub mod model;

lazy_static::lazy_static! {
    static ref USER_TABLE_NAME: Arc<String> =  Arc::new("user".to_string());
}
#[bean(inject)]
pub struct UserManager {
    //cache: MemCache<Arc<String>, Arc<UserDto>>,
    //cache_sec: i32,
    raft_table_route: Option<Arc<TableRoute>>,
    table_manager: Option<Addr<TableManager>>,
}

impl UserManager {
    pub fn new() -> Self {
        Self {
            //cache: MemCache::new(),
            //cache_sec: 1200,
            raft_table_route: Default::default(),
            table_manager: Default::default(),
        }
    }

    //fn update_timeout(&mut self, key: &Arc<String>) {
    //    self.cache.update_time_out(key, self.cache_sec)
    //}

    async fn init_manager_user(
        table_manager: Option<Addr<TableManager>>,
        self_addr: Addr<UserManager>,
    ) -> anyhow::Result<()> {
        if let Some(table_manager) = table_manager {
            let req = TableManagerQueryReq::QueryPageList {
                table_name: USER_TABLE_NAME.clone(),
                limit: Some(1),
                offset: None,
                is_rev: false,
            };
            if let TableManagerResult::PageListResult(count, _) = table_manager.send(req).await?? {
                if count == 0 {
                    let user_manager_req = UserManagerReq::AddUser {
                        name: Arc::new("admin".to_owned()),
                        nickname: "admin".to_owned(),
                        password: "admin".to_owned(),
                    };
                    self_addr.do_send(user_manager_req);
                }
            }
        }
        Ok(())
    }
}

impl Default for UserManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Inject for UserManager {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        factory_data: bean_factory::FactoryData,
        _factory: bean_factory::BeanFactory,
        ctx: &mut Self::Context,
    ) {
        self.raft_table_route = factory_data.get_bean();
        self.table_manager = factory_data.get_actor();
        let raft_addr_route: Option<Arc<RaftAddrRouter>> = factory_data.get_bean();
        ctx.run_later(Duration::from_millis(500), |act, ctx| {
            let self_addr = ctx.address();
            let table_manager = act.table_manager.clone();
            async move {
                if let Some(raft_addr_route) = raft_addr_route {
                    if let Ok(route_res) = raft_addr_route.get_route_addr().await {
                        match route_res {
                            RouteAddr::Local => {
                                //当节点启动后在此处触发
                                Self::init_manager_user(table_manager, self_addr).await.ok();
                            }
                            RouteAddr::Remote(_, _) => {}
                            RouteAddr::Unknown => {
                                // 等待选主节点后尝试触发
                                tokio::time::sleep(Duration::from_secs(10)).await;
                                if let Ok(RouteAddr::Local) = raft_addr_route.get_route_addr().await
                                {
                                    Self::init_manager_user(table_manager, self_addr).await.ok();
                                }
                            }
                        }
                    }
                };
            }
            .into_actor(act)
            .map(|_, _, _| {})
            .spawn(ctx);
        });
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
    QueryPageList {
        offset: Option<i64>,
        limit: Option<i64>,
        is_rev: bool,
    },
}

pub enum UserManagerInnerCtx {
    UpdateUser { key: Arc<String>, value: UserDto },
    CheckUserResult(Arc<String>, bool, UserDto),
    QueryUser(Arc<String>, Option<UserDto>),
    UserPageResult(usize, Vec<UserDto>),
}

pub enum UserManagerResult {
    None,
    CheckUserResult(bool, UserDto),
    QueryUser(Option<Arc<UserDto>>),
    UserPageResult(usize, Vec<UserDto>),
}

impl Handler<UserManagerReq> for UserManager {
    type Result = ResponseActFuture<Self, anyhow::Result<UserManagerResult>>;

    fn handle(&mut self, msg: UserManagerReq, _ctx: &mut Self::Context) -> Self::Result {
        let raft_table_route = self.raft_table_route.clone();
        let table_manager = self.table_manager.clone();
        //let query_info_at_cache = match &msg {
        //    UserManagerReq::Query { name } => self.cache.get(name).ok().is_some(),
        //    _ => false,
        //};
        let query_info_at_cache = false;
        let fut = async move {
            match msg {
                UserManagerReq::AddUser {
                    name,
                    nickname,
                    password,
                } => {
                    let now = (now_millis() / 1000) as u32;
                    let user = UserDto {
                        username: name.as_ref().to_owned(),
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
                    let now = (now_millis() / 1000) as u32;
                    last_user.gmt_modified = now;
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
                        last_user,
                    ))
                }
                UserManagerReq::Query { name } => {
                    if query_info_at_cache {
                        Ok(UserManagerInnerCtx::QueryUser(name, None))
                    } else {
                        let last_user = if let Some(table_manager) = &table_manager {
                            let query_req = TableManagerQueryReq::GetByArcKey {
                                table_name: USER_TABLE_NAME.clone(),
                                key: name.clone(),
                            };
                            match table_manager.send(query_req).await?? {
                                TableManagerResult::Value(old_value) => {
                                    Some(UserDto::from_bytes(&old_value)?)
                                }
                                _ => None,
                            }
                        } else {
                            None
                        };
                        Ok(UserManagerInnerCtx::QueryUser(name, last_user))
                    }
                }
                UserManagerReq::QueryPageList {
                    offset,
                    limit,
                    is_rev,
                } => {
                    if let Some(table_manager) = &table_manager {
                        let query_req = TableManagerQueryReq::QueryPageList {
                            table_name: USER_TABLE_NAME.clone(),
                            offset,
                            limit,
                            is_rev,
                        };
                        match table_manager.send(query_req).await?? {
                            TableManagerResult::PageListResult(size, list) => {
                                let mut user_list = Vec::with_capacity(list.len());
                                for (_, v) in list {
                                    user_list.push(UserDto::from_bytes(&v)?);
                                }
                                Ok(UserManagerInnerCtx::UserPageResult(size, user_list))
                            }
                            _ => Ok(UserManagerInnerCtx::UserPageResult(0, vec![])),
                        }
                    } else {
                        Ok(UserManagerInnerCtx::UserPageResult(0, vec![]))
                    }
                }
            }
        }
        .into_actor(self)
        .map(
            |res: anyhow::Result<UserManagerInnerCtx>, _act, _ctx| match res? {
                UserManagerInnerCtx::UpdateUser { key: _, value: _ } => {
                    //act.cache.set(key, Arc::new(value), act.cache_sec);
                    Ok(UserManagerResult::None)
                }
                UserManagerInnerCtx::CheckUserResult(_key, v, user) => {
                    //if v {
                    //    act.update_timeout(&key);
                    //}
                    Ok(UserManagerResult::CheckUserResult(v, user))
                }
                UserManagerInnerCtx::QueryUser(_key, user) => match user {
                    Some(user) => {
                        let user = Arc::new(user);
                        Ok(UserManagerResult::QueryUser(Some(user)))
                    }
                    None => Ok(UserManagerResult::QueryUser(None)),
                },
                UserManagerInnerCtx::UserPageResult(size, list) => {
                    Ok(UserManagerResult::UserPageResult(size, list))
                }
            },
        );
        Box::pin(fut)
    }
}
