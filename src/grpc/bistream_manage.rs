use std::{collections::{HashMap, HashSet}, sync::Arc, time::Duration};

use crate::{now_millis, config::config::{ConfigKey, ConfigActor, ConfigCmd}, naming::{model::{ServiceInfo, ServiceKey}, core::{NamingActor, NamingCmd}}};

use super::{bistream_conn::{BiStreamConn, BiStreamSenderCmd}, PayloadUtils, nacos_proto::Payload, api_model::{ConfigChangeNotifyRequest, NotifySubscriberRequest}, handler::converter::ModelConverter};
use actix::prelude::*;
use inner_mem_cache::TimeoutSet;


struct ConnCacheItem {
    last_active_time:u64,
    conn:Addr<BiStreamConn>,
}

impl ConnCacheItem {
    fn new(last_active_time:u64,conn:Addr<BiStreamConn>) -> Self {
        Self { last_active_time, conn: conn }
    }
}

#[derive(Default)]
pub struct BiStreamManage{
    conn_cache:HashMap<Arc<String>,ConnCacheItem>,
    active_time_set: TimeoutSet<Arc<String>>,
    response_time_set : TimeoutSet<Arc<String>>,
    detection_time_out: u64,
    response_time_out: u64,
    request_id:u64,
    config_addr:Option<Addr<ConfigActor>>,
    naming_addr:Option<Addr<NamingActor>>,
}

impl BiStreamManage {

    pub fn new() -> Self {
        let mut this= BiStreamManage::default();
        this.detection_time_out = 5000;
        this.response_time_out = 5000;
        this
    }

    pub fn set_config_addr(&mut self,addr:Addr<ConfigActor>) {
        self.config_addr = Some(addr);
    }

    pub fn set_naming_addr(&mut self,addr:Addr<NamingActor>) {
        self.naming_addr = Some(addr);
    }

    pub fn add_conn(&mut self,client_id:Arc<String>,sender:Addr<BiStreamConn>) {
        log::info!("add_conn client_id:{}",&client_id);
        let now = now_millis();
        let item = ConnCacheItem::new(now,sender);
        if let Some(old_conn) = self.conn_cache.insert(client_id.clone(), item) {
            log::info!("add_conn remove old conn:{}",&client_id);
            old_conn.conn.do_send(BiStreamSenderCmd::Close);
        }
        self.active_time_set.add(now+self.detection_time_out, client_id);
    }

    fn active_client(&mut self,client_id:Arc<String>) -> anyhow::Result<()> {
        let now = now_millis();
        if let Some(item)=self.conn_cache.get_mut(&client_id) {
            //log::info!("active_client success client_id:{}",&client_id);
            item.last_active_time = now;
            Ok(())
        }
        else{
            //log::info!("active_client empty client_id:{}",&client_id);
            Err(anyhow::anyhow!("Connection is unregistered."))
        }
    }

    fn check_active_time_set(&mut self,now:u64){
        let keys=self.active_time_set.timeout(now);
        /*
        if keys.len()>0 {
            log::info!("check_active_time_set time size:{}",keys.len());
        }
         */
        for key in keys{
            if let Some(item)=self.conn_cache.get(&key) {
                item.conn.do_send(BiStreamSenderCmd::Detection(self.request_id.to_string()));
                self.request_id+=1;
                self.response_time_set.add(now+self.response_time_out, key);
            }
        }
    }

    fn check_response_time_set(&mut self,now:u64){
        let keys=self.response_time_set.timeout(now);
        let mut del_keys = vec![];
        for key in keys {
            if let Some(item)=self.conn_cache.get(&key) {
                if item.last_active_time + self.detection_time_out + self.response_time_out <= now {
                    del_keys.push(key);
                }
                else{
                    self.active_time_set.add(item.last_active_time+self.detection_time_out, key.clone());
                }
            }
        }
        if del_keys.len()>0 {
            log::info!("check_response_time_set time size:{}",del_keys.len());
        }
        for key in &del_keys {
            if let Some(item) = self.conn_cache.remove(key){
                item.conn.do_send(BiStreamSenderCmd::Close);
            }
        }
        if let Some(config_addr) = &self.config_addr {
            for key in &del_keys {
                config_addr.do_send(ConfigCmd::RemoveSubscribeClient(key.clone()));
            }
        }
        if let Some(naming_addr) = &self.naming_addr{
            for key in &del_keys {
                naming_addr.do_send(NamingCmd::RemoveClient(key.clone()));
            }
        }
    }


    pub fn time_out_heartbeat(&self,ctx:&mut actix::Context<Self>) {
        ctx.run_later(Duration::new(3,0), |act,ctx|{
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

#[derive(Message)]
#[rtype(result = "anyhow::Result<BiStreamManageResult>")]
pub enum BiStreamManageCmd {
    Response(Arc<String>,Payload),
    ConnClose(Arc<String>),
    AddConn(Arc<String>,BiStreamConn),
    ActiveClinet(Arc<String>),
    NotifyConfig(ConfigKey,HashSet<Arc<String>>),
    NotifyNaming(ServiceKey,HashSet<Arc<String>>,ServiceInfo),
}

pub enum BiStreamManageResult {
    None,
}

impl Handler<BiStreamManageCmd> for BiStreamManage {
    type Result = anyhow::Result<BiStreamManageResult>;

    fn handle(&mut self, msg: BiStreamManageCmd, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            BiStreamManageCmd::Response(client_id,payload)=> {
                //println!("BiStreamManageCmd payload:{},client_id:{}",PayloadUtils::get_payload_string(&payload),&client_id);
                if let Some(t)=PayloadUtils::get_payload_type(&payload) {
                    if "ClientDetectionResponse"== t {
                        self.active_client(client_id);
                    }
                }
            },
            BiStreamManageCmd::ConnClose(client_id) => {
                self.conn_cache.remove(&client_id);
                if let Some(config_addr)=&self.config_addr {
                    config_addr.do_send(ConfigCmd::RemoveSubscribeClient(client_id.clone()))
                }
                if let Some(naming_addr)=&self.naming_addr {
                    naming_addr.do_send(NamingCmd::RemoveClient(client_id));
                }
            },
            BiStreamManageCmd::AddConn(client_id, conn) => {
                self.add_conn(client_id, conn.start());
            },
            BiStreamManageCmd::ActiveClinet(client_id) => {
                self.active_client(client_id)?;
            },
            BiStreamManageCmd::NotifyConfig(config_key, client_id_set) => {
                let mut request = ConfigChangeNotifyRequest::default();
                request.group = config_key.group;
                request.data_id= config_key.data_id;
                request.tenant= config_key.tenant;
                let payload = Arc::new(PayloadUtils::build_payload(
                    "ConfigChangeNotifyRequest",
                    serde_json::to_string(&request).unwrap(),
                ));
                for item in &client_id_set {
                    if let Some(item) = self.conn_cache.get(item){
                        item.conn.do_send(BiStreamSenderCmd::Send(payload.clone()));
                    }
                }
            },
            BiStreamManageCmd::NotifyNaming(service_key,client_id_set,service_info) => {
                let service_info = ModelConverter::to_api_service_info(service_info);
                let mut request = NotifySubscriberRequest::default();
                request.namespace = Some(service_key.namespace_id);
                request.group_name = Some(service_key.group_name);
                request.service_name = Some(service_key.service_name);
                request.service_info = Some(service_info);
                let payload = Arc::new(PayloadUtils::build_payload(
                    "NotifySubscriberRequest",
                    serde_json::to_string(&request).unwrap(),
                ));
                for item in &client_id_set {
                    if let Some(item) = self.conn_cache.get(item){
                        item.conn.do_send(BiStreamSenderCmd::Send(payload.clone()));
                    }
                }
            },
        }
        Ok(BiStreamManageResult::None)
    }
}


