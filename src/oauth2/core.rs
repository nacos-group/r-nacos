use crate::oauth2::model::actor_model::{OAuth2MsgReq, OAuth2MsgResult};
use crate::oauth2::model::OAuth2Config;
use crate::oauth2::oauth2_msg_actor::OAuth2MsgActor;
use crate::user::UserManager;
use actix::prelude::*;
use bean_factory::{bean, BeanFactory, FactoryData, Inject};
use std::sync::Arc;

#[bean(inject)]
pub struct OAuth2Manager {
    oauth2_config: Arc<OAuth2Config>,
    oauth2_msg_actor: Option<Addr<OAuth2MsgActor>>,
    enable_oauth2: bool,
    user_manager_addr: Option<Addr<UserManager>>,
}

impl OAuth2Manager {
    pub fn new(oauth2_config: Arc<OAuth2Config>, enable_oauth2: bool) -> Self {
        Self {
            oauth2_config,
            oauth2_msg_actor: None,
            enable_oauth2,
            user_manager_addr: None,
        }
    }

    pub fn init(&mut self, _ctx: &mut Context<Self>) {
        if !self.enable_oauth2 {
            return;
        }
        let oauth2_config = self.oauth2_config.clone();
        let user_manager_addr = self.user_manager_addr.clone();
        match OAuth2MsgActor::new(oauth2_config, user_manager_addr) {
            Ok(actor) => {
                let oauth2_msg_actor = actor.start();
                self.oauth2_msg_actor = Some(oauth2_msg_actor);
            }
            Err(err) => {
                log::error!("init oauth2 error:{}", err);
            }
        }
    }

    async fn handle_req(
        req: OAuth2MsgReq,
        oauth2_msg_addr: Option<Addr<OAuth2MsgActor>>,
    ) -> anyhow::Result<OAuth2MsgResult> {
        let oauth2_msg_addr = if let Some(v) = oauth2_msg_addr {
            v
        } else {
            return Err(anyhow::anyhow!("oauth2_msg_addr is None"));
        };
        oauth2_msg_addr.send(req).await?
    }
}

impl Actor for OAuth2Manager {
    type Context = Context<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("OAuth2Manager started");
    }
}

impl Inject for OAuth2Manager {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        factory_data: FactoryData,
        _factory: BeanFactory,
        ctx: &mut Self::Context,
    ) {
        self.user_manager_addr = factory_data.get_actor();
        self.init(ctx);
    }
}

impl Handler<OAuth2MsgReq> for OAuth2Manager {
    type Result = ResponseActFuture<Self, anyhow::Result<OAuth2MsgResult>>;

    fn handle(&mut self, msg: OAuth2MsgReq, _ctx: &mut Self::Context) -> Self::Result {
        let msg_addr = self.oauth2_msg_actor.clone();
        let fut = Self::handle_req(msg, msg_addr)
            .into_actor(self)
            .map(|result, _act, _ctx| result);
        Box::pin(fut)
    }
}

