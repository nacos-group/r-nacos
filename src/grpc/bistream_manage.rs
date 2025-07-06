use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use crate::{
    config::core::{ConfigActor, ConfigCmd, ConfigKey},
    naming::{
        core::{NamingActor, NamingCmd},
        model::{ServiceInfo, ServiceKey},
    },
    now_millis,
};

use super::{
    api_model::{ConfigChangeNotifyRequest, NotifySubscriberRequest, CONFIG_MODEL, NAMING_MODEL},
    bistream_conn::{BiStreamConn, BiStreamSenderCmd},
    handler::converter::ModelConverter,
    nacos_proto::Payload,
    PayloadUtils,
};
use crate::common::constant::EMPTY_CLIENT_VERSION;
use crate::common::model::ClientVersion;
use crate::grpc::api_model::ConnectionSetupRequest;
use actix::prelude::*;
use bean_factory::{bean, Inject};
use inner_mem_cache::TimeoutSet;

pub(crate) struct ConnCacheItem {
    last_active_time: u64,
    conn: Addr<BiStreamConn>,
    pub(crate) client_version: Arc<ClientVersion>,
}

impl ConnCacheItem {
    fn new(last_active_time: u64, conn: Addr<BiStreamConn>) -> Self {
        Self {
            last_active_time,
            conn,
            client_version: EMPTY_CLIENT_VERSION.clone(),
        }
    }
}

#[bean(inject)]
#[derive(Default)]
pub struct BiStreamManage {
    pub(crate) conn_cache: HashMap<Arc<String>, ConnCacheItem>,
    pub(crate) active_time_set: TimeoutSet<Arc<String>>,
    pub(crate) response_time_set: TimeoutSet<Arc<String>>,
    detection_time_out: u64,
    response_time_out: u64,
    request_id: u64,
    config_addr: Option<Addr<ConfigActor>>,
    naming_addr: Option<Addr<NamingActor>>,
}

impl BiStreamManage {
    pub fn new() -> Self {
        Self {
            detection_time_out: 15000,
            response_time_out: 3000,
            ..Default::default()
        }
    }

    pub fn add_conn(&mut self, client_id: Arc<String>, sender: Addr<BiStreamConn>) {
        log::info!("add_conn client_id:{}", &client_id);
        let now = now_millis();
        let item = ConnCacheItem::new(now, sender);
        if let Some(old_conn) = self.conn_cache.insert(client_id.clone(), item) {
            log::info!("add_conn remove old conn:{}", &client_id);
            old_conn.conn.do_send(BiStreamSenderCmd::Close);
        }
        self.active_time_set
            .add(now + self.detection_time_out, client_id);
    }

    fn active_client(&mut self, client_id: Arc<String>) -> anyhow::Result<Arc<ClientVersion>> {
        let now = now_millis();
        if let Some(item) = self.conn_cache.get_mut(&client_id) {
            //log::info!("active_client success client_id:{}",&client_id);
            item.last_active_time = now;
            Ok(item.client_version.clone())
        } else {
            //log::info!("active_client empty client_id:{}",&client_id);
            Err(anyhow::anyhow!("Connection is unregistered."))
        }
    }

    fn next_request_id(&mut self) -> String {
        if self.request_id >= 0x7fff_ffff_ffff_ffff {
            self.request_id = 0;
        } else {
            self.request_id += 1;
        }
        self.request_id.to_string()
    }

    fn check_active_time_set(&mut self, now: u64) {
        let keys = self.active_time_set.timeout(now);
        let mut check_keys = vec![];
        for key in keys {
            if let Some(item) = self.conn_cache.get(&key) {
                let next_time = item.last_active_time + self.detection_time_out;
                if item.last_active_time + self.detection_time_out <= now {
                    check_keys.push((key, self.next_request_id()));
                } else {
                    self.active_time_set.add(next_time, key.clone());
                }
            }
        }
        if !check_keys.is_empty() {
            log::info!("check timeout detection client, size:{}", check_keys.len());
        }
        for (key, request_id) in check_keys {
            if let Some(item) = self.conn_cache.get(&key) {
                item.conn.do_send(BiStreamSenderCmd::Detection(request_id));
                self.response_time_set
                    .add(now + self.response_time_out, key);
            }
        }
    }

    fn check_response_time_set(&mut self, now: u64) {
        let keys = self.response_time_set.timeout(now);
        let mut del_keys = vec![];
        for key in keys {
            if let Some(item) = self.conn_cache.get(&key) {
                if item.last_active_time + self.detection_time_out + self.response_time_out <= now {
                    del_keys.push(key);
                } else {
                    self.active_time_set
                        .add(item.last_active_time + self.detection_time_out, key.clone());
                }
            }
        }
        if !del_keys.is_empty() {
            log::info!("check timeout close client, size:{}", del_keys.len());
        }
        for key in &del_keys {
            if let Some(item) = self.conn_cache.remove(key) {
                //item.conn.do_send(BiStreamSenderCmd::Reset(self.next_request_id(),None,None));
                item.conn.do_send(BiStreamSenderCmd::Close);
            }
        }
        if let Some(config_addr) = &self.config_addr {
            for key in &del_keys {
                config_addr.do_send(ConfigCmd::RemoveSubscribeClient(key.clone()));
            }
        }
        if let Some(naming_addr) = &self.naming_addr {
            for key in &del_keys {
                naming_addr.do_send(NamingCmd::RemoveClient(key.clone()));
            }
        }
    }

    pub fn time_out_heartbeat(&self, ctx: &mut actix::Context<Self>) {
        ctx.run_later(Duration::new(2, 0), |act, ctx| {
            let now = now_millis();
            act.check_active_time_set(now);
            act.check_response_time_set(now);
            act.time_out_heartbeat(ctx);
        });
    }
}

impl Actor for BiStreamManage {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.time_out_heartbeat(ctx);
        log::info!("BiStreamManage started");
    }
}

impl Inject for BiStreamManage {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        factory_data: bean_factory::FactoryData,
        _factory: bean_factory::BeanFactory,
        _ctx: &mut Self::Context,
    ) {
        self.config_addr = factory_data.get_actor();
        self.naming_addr = factory_data.get_actor();
        log::info!("BiStreamManage inject complete");
    }
}

impl Supervised for BiStreamManage {
    fn restarting(&mut self, _ctx: &mut <Self as Actor>::Context) {
        log::warn!("BiStreamManage restart ...");
    }
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<BiStreamManageResult>")]
pub enum BiStreamManageCmd {
    Response(Arc<String>, Payload),
    ConnClose(Arc<String>),
    AddConn(Arc<String>, BiStreamConn),
    ActiveClinet(Arc<String>),
    NotifyConfig(ConfigKey, HashSet<Arc<String>>),
    NotifyNaming(ServiceKey, HashSet<Arc<String>>, ServiceInfo),
    QueryConnList,
}

pub enum BiStreamManageResult {
    ConnList(Vec<Arc<String>>),
    ClientInfo(Arc<ClientVersion>),
    None,
}

impl Handler<BiStreamManageCmd> for BiStreamManage {
    type Result = anyhow::Result<BiStreamManageResult>;

    fn handle(&mut self, msg: BiStreamManageCmd, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            BiStreamManageCmd::Response(client_id, payload) => {
                #[cfg(feature = "debug")]
                log::info!(
                    "BiStreamManageCmd payload:{},client_id:{}",
                    PayloadUtils::get_payload_string(&payload),
                    &client_id
                );
                if let Some(t) = PayloadUtils::get_payload_type(&payload) {
                    if t.as_str() == "ConnectionSetupRequest" {
                        let body_vec = payload.body.unwrap_or_default().value;
                        let request: ConnectionSetupRequest = serde_json::from_slice(&body_vec)?;
                        if let Some(item) = self.conn_cache.get_mut(&client_id) {
                            if let Some(client_version) = request.client_version {
                                item.client_version =
                                    Arc::new(ClientVersion::from_string(&client_version));
                            }
                        }
                    }
                    self.active_client(client_id).ok();
                }
            }
            BiStreamManageCmd::ConnClose(client_id) => {
                self.conn_cache.remove(&client_id);
                if let Some(config_addr) = &self.config_addr {
                    config_addr.do_send(ConfigCmd::RemoveSubscribeClient(client_id.clone()))
                }
                if let Some(naming_addr) = &self.naming_addr {
                    naming_addr.do_send(NamingCmd::RemoveClient(client_id));
                }
                //println!("|ConnClose|conn size: {}",self.conn_cache.len());
            }
            BiStreamManageCmd::AddConn(client_id, conn) => {
                self.add_conn(client_id, conn.start());
                //println!("|AddConn|conn size: {}",self.conn_cache.len());
            }
            BiStreamManageCmd::ActiveClinet(client_id) => {
                let client_version = self.active_client(client_id)?;
                return Ok(BiStreamManageResult::ClientInfo(client_version));
            }
            BiStreamManageCmd::NotifyConfig(config_key, client_id_set) => {
                let request = ConfigChangeNotifyRequest {
                    group: config_key.group,
                    data_id: config_key.data_id,
                    tenant: config_key.tenant,
                    request_id: Some(self.next_request_id()),
                    module: Some(CONFIG_MODEL.to_string()),
                    ..Default::default()
                };
                let payload = Arc::new(PayloadUtils::build_payload(
                    "ConfigChangeNotifyRequest",
                    serde_json::to_string(&request).unwrap(),
                ));
                for item in &client_id_set {
                    if let Some(item) = self.conn_cache.get(item) {
                        item.conn.do_send(BiStreamSenderCmd::Send(payload.clone()));
                    }
                }
            }
            BiStreamManageCmd::NotifyNaming(service_key, client_id_set, service_info) => {
                let service_info = ModelConverter::to_api_service_info(service_info);
                let request = NotifySubscriberRequest {
                    namespace: Some(service_key.namespace_id),
                    group_name: Some(service_key.group_name),
                    service_name: Some(service_key.service_name),
                    service_info: Some(service_info),
                    request_id: Some(self.next_request_id()),
                    module: Some(NAMING_MODEL.to_string()),
                    ..Default::default()
                };
                let payload = Arc::new(PayloadUtils::build_payload(
                    "NotifySubscriberRequest",
                    serde_json::to_string(&request).unwrap(),
                ));
                for item in &client_id_set {
                    if let Some(item) = self.conn_cache.get(item) {
                        item.conn.do_send(BiStreamSenderCmd::Send(payload.clone()));
                    }
                }
            }
            BiStreamManageCmd::QueryConnList => {
                let mut list = Vec::with_capacity(self.conn_cache.len());
                for key in self.conn_cache.keys() {
                    list.push(key.clone());
                }
                return Ok(BiStreamManageResult::ConnList(list));
            }
        }
        Ok(BiStreamManageResult::None)
    }
}
