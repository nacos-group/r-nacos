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

use self::{
    model::{UserDo, UserDto},
    permission::USER_ROLE_MANAGER,
};

pub mod api;
pub mod model;
pub mod permission;

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
                like_key: None,
                limit: Some(1),
                offset: None,
                is_rev: false,
            };
            if let TableManagerResult::PageListResult(count, _) = table_manager.send(req).await?? {
                if count == 0 {
                    let user = UserDto {
                        username: Arc::new("admin".to_owned()),
                        nickname: Some("admin".to_owned()),
                        password: Some("admin".to_owned()),
                        roles: Some(vec![USER_ROLE_MANAGER.clone()]),
                        ..Default::default()
                    };
                    let user_manager_req = UserManagerReq::AddUser { user };
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
        user: UserDto,
    },
    UpdateUser {
        user: UserDto,
    },
    CheckUser {
        name: Arc<String>,
        password: String,
    },
    Remove {
        username: Arc<String>,
    },
    Query {
        name: Arc<String>,
    },
    QueryPageList {
        like_username: Option<String>,
        offset: Option<i64>,
        limit: Option<i64>,
        is_rev: bool,
    },
}

pub enum UserManagerInnerCtx {
    None,
    UpdateUser { key: Arc<String>, value: UserDo },
    CheckUserResult(Arc<String>, bool, UserDo),
    QueryUser(Arc<String>, Option<UserDo>),
    UserPageResult(usize, Vec<UserDto>),
}

pub enum UserManagerResult {
    None,
    CheckUserResult(bool, UserDto),
    QueryUser(Option<UserDto>),
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
                UserManagerReq::AddUser { user } => {
                    let now = (now_millis() / 1000) as u32;
                    let user_do = UserDo {
                        username: user.username.as_ref().to_owned(),
                        password: user.password.unwrap_or_default(),
                        nickname: user.nickname.unwrap_or_default(),
                        gmt_create: now,
                        gmt_modified: now,
                        roles: user
                            .roles
                            .unwrap()
                            .into_iter()
                            .map(|e| e.as_ref().to_owned())
                            .collect(),
                        enable: true,
                        extend_info: user.extend_info.unwrap_or_default(),
                    };
                    let user_data = user_do.to_bytes();
                    let req = TableManagerReq::Set {
                        table_name: USER_TABLE_NAME.clone(),
                        key: user.username.as_bytes().to_owned(),
                        value: user_data,
                        last_seq_id: None,
                    };
                    if let Some(raft_table_route) = raft_table_route {
                        raft_table_route.request(req).await.ok();
                    }
                    Ok(UserManagerInnerCtx::UpdateUser {
                        key: user.username,
                        value: user_do,
                    })
                }
                UserManagerReq::UpdateUser { user } => {
                    let mut last_user = if let Some(raft_table_route) = &raft_table_route {
                        let query_req = TableManagerQueryReq::GetByArcKey {
                            table_name: USER_TABLE_NAME.clone(),
                            key: user.username.clone(),
                        };
                        match raft_table_route.get_leader_data(query_req).await? {
                            TableManagerResult::Value(old_value) => UserDo::from_bytes(&old_value)?,
                            _ => return Err(anyhow::anyhow!("not found user {}", &user.username)),
                        }
                    } else {
                        return Err(anyhow::anyhow!("raft_table_route is none "));
                    };
                    let now = (now_millis() / 1000) as u32;
                    last_user.gmt_modified = now;
                    if let Some(nickname) = user.nickname {
                        if !nickname.is_empty() {
                            last_user.nickname = nickname;
                        }
                    }
                    if let Some(password) = user.password {
                        if !password.is_empty() {
                            last_user.password = password;
                        }
                    }
                    if let Some(enable) = user.enable {
                        last_user.enable = enable;
                    }
                    if let Some(extend_info) = user.extend_info {
                        if !extend_info.is_empty() {
                            last_user.extend_info = extend_info;
                        }
                    }
                    if let Some(roles) = user.roles {
                        if !roles.is_empty() {
                            last_user.roles =
                                roles.into_iter().map(|e| e.as_ref().to_owned()).collect();
                        }
                    }
                    last_user.gmt_modified = now;
                    let user_data = last_user.to_bytes();
                    let req = TableManagerReq::Set {
                        table_name: USER_TABLE_NAME.clone(),
                        key: user.username.as_bytes().to_owned(),
                        value: user_data,
                        last_seq_id: None,
                    };
                    if let Some(raft_table_route) = raft_table_route {
                        raft_table_route.request(req).await.ok();
                    }
                    Ok(UserManagerInnerCtx::UpdateUser {
                        key: user.username,
                        value: last_user,
                    })
                }
                UserManagerReq::CheckUser { name, password } => {
                    if name.is_empty() || password.is_empty() {
                        return Err(anyhow::anyhow!("args is empty"));
                    }
                    let last_user = if let Some(raft_table_route) = &raft_table_route {
                        let query_req = TableManagerQueryReq::GetByArcKey {
                            table_name: USER_TABLE_NAME.clone(),
                            key: name.clone(),
                        };
                        match raft_table_route.get_leader_data(query_req).await? {
                            TableManagerResult::Value(old_value) => UserDo::from_bytes(&old_value)?,
                            _ => return Err(anyhow::anyhow!("not found user {}", &name)),
                        }
                    } else {
                        return Err(anyhow::anyhow!("raft_table_route is none "));
                    };
                    Ok(UserManagerInnerCtx::CheckUserResult(
                        name,
                        last_user.enable && last_user.password == password,
                        last_user,
                    ))
                }
                UserManagerReq::Remove { username } => {
                    let req = TableManagerReq::Remove {
                        table_name: USER_TABLE_NAME.clone(),
                        key: username.as_bytes().to_owned(),
                    };
                    if let Some(raft_table_route) = raft_table_route {
                        raft_table_route.request(req).await.ok();
                    }
                    Ok(UserManagerInnerCtx::None)
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
                                    Some(UserDo::from_bytes(&old_value)?)
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
                    like_username,
                    limit,
                    is_rev,
                } => {
                    if let Some(table_manager) = &table_manager {
                        let query_req = TableManagerQueryReq::QueryPageList {
                            table_name: USER_TABLE_NAME.clone(),
                            like_key: like_username,
                            offset,
                            limit,
                            is_rev,
                        };
                        match table_manager.send(query_req).await?? {
                            TableManagerResult::PageListResult(size, list) => {
                                let mut user_list = Vec::with_capacity(list.len());
                                for (_, v) in list {
                                    user_list.push(UserDo::from_bytes(&v)?.into());
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
                UserManagerInnerCtx::None => Ok(UserManagerResult::None),
                UserManagerInnerCtx::UpdateUser { key: _, value: _ } => {
                    //act.cache.set(key, Arc::new(value), act.cache_sec);
                    Ok(UserManagerResult::None)
                }
                UserManagerInnerCtx::CheckUserResult(_key, v, user) => {
                    //if v {
                    //    act.update_timeout(&key);
                    //}
                    Ok(UserManagerResult::CheckUserResult(v, user.into()))
                }
                UserManagerInnerCtx::QueryUser(_key, user) => match user {
                    Some(user) => Ok(UserManagerResult::QueryUser(Some(user.into()))),
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
