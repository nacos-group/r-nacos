use crate::common::AppSysConfig;
use actix::prelude::*;
use anyhow::Error;
use bean_factory::{bean, Inject};
use std::{sync::Arc, time::Duration};
//use inner_mem_cache::MemCache;

use self::{
    model::{UserDo, UserDto},
    permission::USER_ROLE_MANAGER,
};
use crate::common::constant::USER_TREE_NAME;
use crate::common::model::privilege::{PrivilegeGroup, PrivilegeGroupOptionParam};
use crate::common::string_utils::StringUtils;
use crate::raft::cache::{CacheManager, CacheUserChangeReq};
use crate::user::model::UserSourceType;
use crate::user::permission::UserRole;
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

pub mod api;
pub mod model;
pub mod permission;

pub(crate) fn build_password_hash(password: &str) -> anyhow::Result<String> {
    Ok(bcrypt::hash(password, 10u32)?)
}

pub(crate) fn verify_password_hash(password: &str, password_hash: &str) -> anyhow::Result<bool> {
    Ok(bcrypt::verify(password, password_hash)?)
}

pub(crate) fn verify_password_hash_option(
    password: &str,
    password_hash: &Option<String>,
) -> anyhow::Result<bool> {
    if let Some(password_hash) = password_hash {
        verify_password_hash(password, password_hash)
    } else {
        Err(anyhow::anyhow!("password_hash is empty"))
    }
}

#[bean(inject)]
pub struct UserManager {
    //cache: MemCache<Arc<String>, Arc<UserDto>>,
    //cache_sec: i32,
    raft_table_route: Option<Arc<TableRoute>>,
    table_manager: Option<Addr<TableManager>>,
    cache_manager: Option<Addr<CacheManager>>,
}

impl UserManager {
    pub fn new() -> Self {
        Self {
            //cache: MemCache::new(),
            //cache_sec: 1200,
            raft_table_route: Default::default(),
            table_manager: Default::default(),
            cache_manager: Default::default(),
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
                table_name: USER_TREE_NAME.clone(),
                like_key: None,
                limit: Some(1),
                offset: None,
                is_rev: false,
            };
            if let TableManagerResult::PageListResult(count, _) = table_manager.send(req).await?? {
                if count == 0 {
                    let sys_config = AppSysConfig::init_from_env();
                    let user = UserDto {
                        username: Arc::new(sys_config.init_admin_username.to_string()),
                        nickname: Some(sys_config.init_admin_username.to_owned()),
                        password: Some(sys_config.init_admin_password.to_owned()),
                        roles: Some(vec![USER_ROLE_MANAGER.clone()]),
                        ..Default::default()
                    };
                    let user_manager_req = UserManagerReq::AddUser {
                        user,
                        namespace_privilege_param: None,
                    };
                    self_addr.do_send(user_manager_req);
                }
            }
        }
        Ok(())
    }

    async fn add_user(
        raft_table_route: Option<Arc<TableRoute>>,
        cache_manager: Option<Addr<CacheManager>>,
        user: UserDto,
        namespace_privilege_param: Option<PrivilegeGroupOptionParam<Arc<String>>>,
    ) -> anyhow::Result<UserManagerInnerCtx> {
        let now = (now_millis() / 1000) as u32;
        let password_hash = if let Some(password) = &user.password {
            build_password_hash(password).ok()
        } else {
            None
        };
        let mut namespace_privilege = PrivilegeGroup::all();
        if let Some(namespace_privilege_param) = namespace_privilege_param {
            namespace_privilege.blacklist = namespace_privilege_param.blacklist;
            namespace_privilege.whitelist = namespace_privilege_param.whitelist;
            if let Some(value) = namespace_privilege_param.whitelist_is_all {
                namespace_privilege.whitelist_is_all = value;
            }
            if let Some(value) = namespace_privilege_param.blacklist_is_all {
                namespace_privilege.blacklist_is_all = value;
            }
        }
        let user_do = UserDo {
            username: user.username.as_ref().to_owned(),
            // 新版本不存储原密码,启用后新版数据不支持降级回去使用
            password: String::new(),
            password_hash,
            nickname: user.nickname.unwrap_or_default(),
            gmt_create: now,
            gmt_modified: now,
            roles: user
                .roles
                .unwrap_or_default()
                .into_iter()
                .map(|e| e.as_ref().to_owned())
                .collect(),
            enable: true,
            extend_info: user.extend_info.unwrap_or_default(),
            namespace_privilege_flags: Some(namespace_privilege.get_flags() as u32),
            namespace_white_list: namespace_privilege
                .whitelist
                .unwrap_or_default()
                .iter()
                .map(|e| e.as_ref().to_owned())
                .collect(),
            namespace_black_list: namespace_privilege
                .blacklist
                .unwrap_or_default()
                .iter()
                .map(|e| e.as_ref().to_owned())
                .collect(),
            source: user.source,
        };
        let user_data = user_do.to_bytes();
        let req = TableManagerReq::Set {
            table_name: USER_TREE_NAME.clone(),
            key: user.username.as_bytes().to_owned(),
            value: user_data,
            last_seq_id: None,
        };
        if let Some(raft_table_route) = raft_table_route {
            raft_table_route.request(req).await.ok();
        }
        if let Some(cache_manager) = &cache_manager {
            cache_manager
                .send(CacheUserChangeReq::UserPrivilegeChange {
                    username: user.username.clone(),
                    change_time: now,
                })
                .await
                .ok();
        }
        Ok(UserManagerInnerCtx::UpdateUser {
            key: user.username,
            value: user_do,
        })
    }

    async fn init_user(
        raft_table_route: Option<Arc<TableRoute>>,
        cache_manager: Option<Addr<CacheManager>>,
        user: UserDto,
        namespace_privilege_param: Option<PrivilegeGroupOptionParam<Arc<String>>>,
    ) -> anyhow::Result<UserManagerInnerCtx> {
        if let Some(raft_table_route) = &raft_table_route {
            let query_req = TableManagerQueryReq::GetByArcKey {
                table_name: USER_TREE_NAME.clone(),
                key: user.username.clone(),
            };
            if let TableManagerResult::Value(old_value) =
                raft_table_route.get_leader_data(query_req).await?
            {
                let old_user = UserDo::from_bytes(&old_value)?;
                //exist user
                return Ok(UserManagerInnerCtx::QueryUser(
                    user.username,
                    Some(old_user),
                ));
            }
        };
        Self::add_user(
            raft_table_route,
            cache_manager,
            user,
            namespace_privilege_param,
        )
        .await
    }

    async fn update_user(
        raft_table_route: &Option<Arc<TableRoute>>,
        cache_manager: &Option<Addr<CacheManager>>,
        user: UserDto,
        namespace_privilege_param: Option<PrivilegeGroupOptionParam<Arc<String>>>,
    ) -> Result<UserManagerInnerCtx, Error> {
        let mut last_user = if let Some(raft_table_route) = &raft_table_route {
            let query_req = TableManagerQueryReq::GetByArcKey {
                table_name: USER_TREE_NAME.clone(),
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
        let source = last_user
            .source
            .as_ref()
            .map(|s| UserSourceType::from_name(s).unwrap_or_default())
            .unwrap_or_default();
        last_user.gmt_modified = now;
        if let Some(nickname) = user.nickname {
            if !nickname.is_empty() {
                last_user.nickname = nickname;
            }
        }
        if let Some(password) = user.password {
            if source.is_inner() && !password.is_empty() {
                last_user.password_hash = build_password_hash(&password).ok();
                // 新版本不存储原密码
                last_user.password = String::new();
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
            if source.is_inner() && !roles.is_empty() {
                last_user.roles = roles.into_iter().map(|e| e.as_ref().to_owned()).collect();
            }
        }
        if let Some(namespace_privilege_param) = namespace_privilege_param {
            let mut namespace_privilege = last_user.build_namespace_privilege();
            namespace_privilege.enabled = true;
            if let Some(value) = namespace_privilege_param.blacklist {
                namespace_privilege.blacklist = Some(value);
            }
            if let Some(value) = namespace_privilege_param.whitelist {
                namespace_privilege.whitelist = Some(value);
            }
            if let Some(value) = namespace_privilege_param.whitelist_is_all {
                namespace_privilege.whitelist_is_all = value;
            }
            if let Some(value) = namespace_privilege_param.blacklist_is_all {
                namespace_privilege.blacklist_is_all = value;
            }
            last_user.namespace_privilege_flags = Some(namespace_privilege.get_flags() as u32);
            last_user.namespace_white_list = namespace_privilege
                .whitelist
                .unwrap_or_default()
                .iter()
                .map(|e| e.as_ref().to_owned())
                .collect();
            last_user.namespace_black_list = namespace_privilege
                .blacklist
                .unwrap_or_default()
                .iter()
                .map(|e| e.as_ref().to_owned())
                .collect();
        }
        last_user.gmt_modified = now;
        let user_data = last_user.to_bytes();
        let req = TableManagerReq::Set {
            table_name: USER_TREE_NAME.clone(),
            key: user.username.as_bytes().to_owned(),
            value: user_data,
            last_seq_id: None,
        };
        if let Some(raft_table_route) = raft_table_route {
            raft_table_route.request(req).await.ok();
        }
        if let Some(cache_manager) = &cache_manager {
            cache_manager
                .send(CacheUserChangeReq::UserPrivilegeChange {
                    username: user.username.clone(),
                    change_time: now,
                })
                .await
                .ok();
        }
        Ok(UserManagerInnerCtx::UpdateUser {
            key: user.username,
            value: last_user,
        })
    }

    async fn check_user(
        raft_table_route: &Option<Arc<TableRoute>>,
        name: Arc<String>,
        password: &String,
    ) -> Result<UserManagerInnerCtx, Error> {
        if name.is_empty() || password.is_empty() {
            return Err(anyhow::anyhow!("args is empty"));
        }
        let last_user = if let Some(raft_table_route) = &raft_table_route {
            let query_req = TableManagerQueryReq::GetByArcKey {
                table_name: USER_TREE_NAME.clone(),
                key: name.clone(),
            };
            match raft_table_route.get_leader_data(query_req).await? {
                TableManagerResult::Value(old_value) => UserDo::from_bytes(&old_value)?,
                //_ => return Err(anyhow::anyhow!("not found user {}", &name)),
                _ => return Ok(UserManagerInnerCtx::None),
            }
        } else {
            return Err(anyhow::anyhow!("raft_table_route is none "));
        };
        let source = last_user
            .source
            .as_ref()
            .map(|s| UserSourceType::from_name(s).unwrap_or_default())
            .unwrap_or_default();
        let mut check_success = last_user.enable && source.is_inner();
        if !StringUtils::is_option_empty(&last_user.password_hash) {
            check_success = check_success
                && verify_password_hash_option(password, &last_user.password_hash).unwrap_or(false);
            //debug info
            /*
            println!(
                "login CheckUser use password_hash,hash:{},check_result:{}",
                last_user.password_hash.as_ref().unwrap(),
                &check_success
            );
            */
        } else {
            //兼容老版本数据比较,以支持平滑从老版本升级到新版本
            check_success = check_success && &last_user.password == password;
        }
        Ok(UserManagerInnerCtx::CheckUserResult(
            name,
            check_success,
            last_user,
        ))
    }

    async fn remove(
        raft_table_route: Option<Arc<TableRoute>>,
        table_manager: &Option<Addr<TableManager>>,
        cache_manager: &Option<Addr<CacheManager>>,
        username: Arc<String>,
    ) -> Result<UserManagerInnerCtx, Error> {
        if let Some(table_manager) = &table_manager {
            // 查询该用户信息
            let query_req = TableManagerQueryReq::GetByArcKey {
                table_name: USER_TREE_NAME.clone(),
                key: username.clone(),
            };

            if let TableManagerResult::Value(v) = table_manager.send(query_req).await?? {
                let user_do = UserDo::from_bytes(&v)?;

                // 如果该用户是 admin，进行检查
                if user_do
                    .roles
                    .contains(&UserRole::Manager.to_role_value().to_string())
                {
                    let query_req = TableManagerQueryReq::QueryPageList {
                        table_name: USER_TREE_NAME.clone(),
                        like_key: None,
                        offset: None,
                        limit: None,
                        is_rev: true,
                    };

                    if let TableManagerResult::PageListResult(_, list) =
                        table_manager.send(query_req).await??
                    {
                        let manager_count = list
                            .iter()
                            .filter_map(|(_, v)| UserDo::from_bytes(v).ok())
                            .filter(|user_do| {
                                user_do
                                    .roles
                                    .contains(&UserRole::Manager.to_role_value().to_string())
                            })
                            .count();

                        // 仅剩一个 admin 时，不允许删除
                        if manager_count <= 1 {
                            return Err(anyhow::anyhow!("at least one admin must be reserved!"));
                        }
                    }
                }
            }

            // 移除该用户
            let req = TableManagerReq::Remove {
                table_name: USER_TREE_NAME.clone(),
                key: username.as_bytes().to_owned(),
            };
            if let Some(raft_table_route) = raft_table_route {
                raft_table_route.request(req).await.ok();
            }
            if let Some(cache_manager) = &cache_manager {
                cache_manager
                    .send(CacheUserChangeReq::RemoveUser { username })
                    .await
                    .ok();
            }
        }

        Ok(UserManagerInnerCtx::None)
    }

    async fn query_user(
        table_manager: &Option<Addr<TableManager>>,
        query_info_at_cache: bool,
        name: Arc<String>,
    ) -> Result<UserManagerInnerCtx, Error> {
        if query_info_at_cache {
            Ok(UserManagerInnerCtx::QueryUser(name, None))
        } else {
            let last_user = if let Some(table_manager) = &table_manager {
                let query_req = TableManagerQueryReq::GetByArcKey {
                    table_name: USER_TREE_NAME.clone(),
                    key: name.clone(),
                };
                match table_manager.send(query_req).await?? {
                    TableManagerResult::Value(old_value) => Some(UserDo::from_bytes(&old_value)?),
                    _ => None,
                }
            } else {
                None
            };
            Ok(UserManagerInnerCtx::QueryUser(name, last_user))
        }
    }

    async fn query_user_list(
        table_manager: &Option<Addr<TableManager>>,
        offset: Option<i64>,
        like_username: Option<String>,
        limit: Option<i64>,
        is_rev: bool,
    ) -> Result<UserManagerInnerCtx, Error> {
        if let Some(table_manager) = &table_manager {
            let query_req = TableManagerQueryReq::QueryPageList {
                table_name: USER_TREE_NAME.clone(),
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
        self.cache_manager = factory_data.get_actor();
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
        namespace_privilege_param: Option<PrivilegeGroupOptionParam<Arc<String>>>,
    },
    InitUser {
        user: UserDto,
        namespace_privilege_param: Option<PrivilegeGroupOptionParam<Arc<String>>>,
    },
    UpdateUser {
        user: UserDto,
        namespace_privilege_param: Option<PrivilegeGroupOptionParam<Arc<String>>>,
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
        let cache_manager = self.cache_manager.clone();
        //let query_info_at_cache = match &msg {
        //    UserManagerReq::Query { name } => self.cache.get(name).ok().is_some(),
        //    _ => false,
        //};
        let query_info_at_cache = false;
        let fut = async move {
            match msg {
                UserManagerReq::AddUser {
                    user,
                    namespace_privilege_param,
                } => {
                    Self::add_user(
                        raft_table_route,
                        cache_manager,
                        user,
                        namespace_privilege_param,
                    )
                    .await
                }
                UserManagerReq::InitUser {
                    user,
                    namespace_privilege_param,
                } => {
                    Self::init_user(
                        raft_table_route,
                        cache_manager,
                        user,
                        namespace_privilege_param,
                    )
                    .await
                }
                UserManagerReq::UpdateUser {
                    user,
                    namespace_privilege_param,
                } => {
                    Self::update_user(
                        &raft_table_route,
                        &cache_manager,
                        user,
                        namespace_privilege_param,
                    )
                    .await
                }
                UserManagerReq::CheckUser { name, password } => {
                    Self::check_user(&raft_table_route, name, &password).await
                }
                UserManagerReq::Remove { username } => {
                    Self::remove(raft_table_route, &table_manager, &cache_manager, username).await
                }
                UserManagerReq::Query { name } => {
                    Self::query_user(&table_manager, query_info_at_cache, name).await
                }
                UserManagerReq::QueryPageList {
                    offset,
                    like_username,
                    limit,
                    is_rev,
                } => {
                    Self::query_user_list(&table_manager, offset, like_username, limit, is_rev)
                        .await
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
