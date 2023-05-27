#![allow(unused_imports,unused_assignments,unused_variables,unused_mut,dead_code,unused_must_use)]

use std::sync::Arc;
use std::time::Duration;
use chrono::Local;
use std::collections::HashMap;
use std::collections::BTreeMap;

use crate::grpc::bistream_manage::BiStreamManage;
use crate::utils::get_md5;
use serde::{Serialize, Deserialize};

use actix::prelude::*;

use super::config_db::ConfigDB;
use super::config_subscribe::Subscriber;
use crate::config::config_index::{TenantIndex, ConfigQueryParam};

#[derive(Debug,Hash,Eq,Clone)]
pub struct ConfigKey {
    pub(crate) data_id:Arc<String>,
    pub(crate) group:Arc<String>,
    pub(crate) tenant:Arc<String>,
}

impl ConfigKey {
    pub fn new(data_id:&str,group:&str,tenant:&str) -> ConfigKey {
        ConfigKey {
            data_id:Arc::new(data_id.to_owned()),
            group:Arc::new(group.to_owned()),
            tenant:Arc::new(tenant.to_owned()),
        }
    }

    pub fn new_by_arc(data_id:Arc<String>,group:Arc<String>,tenant:Arc<String>) -> ConfigKey {
        ConfigKey {
            data_id,
            group,
            tenant,
        }
    }

    pub fn build_key(&self) -> String {
        if self.tenant.len()==0 {
            return format!("{}\x02{}",self.data_id,self.group)
        }
        format!("{}\x02{}\x02{}",self.data_id,self.group,self.tenant)
    }
}

impl PartialEq for ConfigKey {
    fn eq(&self,o:&Self) -> bool {
        self.data_id==o.data_id && self.group==o.group && self.tenant==self.tenant
    }
}

pub struct ConfigValue {
    pub(crate) content:Arc<String>,
    pub(crate) md5:Arc<String>,
}

impl ConfigValue {
    fn new(content:Arc<String>) -> Self{
        let md5 = get_md5(&content);
        Self {
            content,
            md5:Arc::new(md5),
        }
    }
}

#[derive(Debug,Serialize,Deserialize,Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigInfoDto {
    pub tenant:Arc<String>,
    pub group:Arc<String>,
    pub data_id:Arc<String>,
    pub content:Option<Arc<String>>,
    pub md5:Option<Arc<String>>,
}

#[derive(Debug)]
pub struct ListenerItem {
    pub key:ConfigKey,
    pub md5:Arc<String>,
}

impl ListenerItem {
    pub fn new(key:ConfigKey,md5:Arc<String>) -> Self {
        Self {
            key,
            md5,
        }
    }

    pub fn decode_listener_items(configs:&str) -> Vec::<Self> {
        let mut list = vec![];
        let mut start = 0;
        let bytes = configs.as_bytes();
        let mut tmp_list = vec![];
        for i in 0..bytes.len(){
            let char = bytes[i];
            if char == 2 {
                if tmp_list.len() > 2{
                    continue;
                }
                tmp_list.push(String::from_utf8(bytes[start..i].to_vec()).unwrap());
                start = i+1;
            }
            else if char == 1 {
                let mut end_value = String::new();
                if start+1 <=i {
                    end_value = String::from_utf8(bytes[start..i].to_vec()).unwrap();
                }
                start = i+1;
                if tmp_list.len() == 2 {
                    let key = ConfigKey::new(&tmp_list[0],&tmp_list[1],"");
                    list.push(ListenerItem::new(key,Arc::new(end_value)));
                }
                else{
                    if end_value=="public" {
                        end_value="".to_owned();
                    }
                    let key = ConfigKey::new(&tmp_list[0],&tmp_list[1],&end_value);
                    list.push(ListenerItem::new(key,Arc::new(tmp_list[2].to_owned())));
                }
                tmp_list.clear();
            }
        }
        list
    }

    pub fn decode_listener_change_keys(configs:&str) -> Vec::<ConfigKey> {
        let mut list = vec![];
        let mut start = 0;
        let bytes = configs.as_bytes();
        let mut tmp_list = vec![];
        for i in 0..bytes.len(){
            let char = bytes[i];
            if char == 2 {
                if tmp_list.len() > 2{
                    continue;
                }
                tmp_list.push(String::from_utf8(bytes[start..i].to_vec()).unwrap());
                start = i+1;
            }
            else if char == 1 {
                let mut end_value = String::new();
                if start+1 <=i {
                    end_value = String::from_utf8(bytes[start..i].to_vec()).unwrap();
                }
                start = i+1;
                if tmp_list.len() == 1 {
                    let key = ConfigKey::new(&tmp_list[0],&end_value,"");
                    list.push(key);
                }
                else{
                    let key = ConfigKey::new(&tmp_list[0],&tmp_list[1],&end_value);
                    list.push(key);
                }
                tmp_list.clear();
            }
        }
        list
    }
}

struct OnceListener{
    version:u64,
    time:i64,
    list:Vec<ListenerItem>,
}

pub enum ListenerResult {
    NULL,
    DATA(Vec<ConfigKey>),
}

type ListenerSenderType = tokio::sync::oneshot::Sender<ListenerResult>;
//type ListenerReceiverType = tokio::sync::oneshot::Receiver<ListenerResult>;

struct ConfigListener {
    version:u64,
    listener: HashMap<ConfigKey,Vec<u64>>,
    time_listener: BTreeMap<i64,Vec<OnceListener>>,
    sender_map:HashMap<u64,ListenerSenderType>,
}

impl ConfigListener {
    fn new() -> Self {
        Self {
            version:0,
            listener: Default::default(),
            time_listener: Default::default(),
            sender_map: Default::default(),
        }
    }

    fn add(&mut self,items:Vec<ListenerItem>,sender:ListenerSenderType,time:i64) {
        self.version+=1;
        for item in &items {
            let key = item.key.clone();
            match self.listener.get_mut(&key) {
                Some(list) => {
                    list.push(self.version);
                },
                None => {
                    self.listener.insert(key,vec![self.version]);
                }
            };
        }
        self.sender_map.insert(self.version,sender);
        let once_listener = OnceListener {
            version: self.version,
            time:time,
            list:items
        };
        match self.time_listener.get_mut(&time) {
            Some(list) => {
                list.push(once_listener);
            }
            None => {
                self.time_listener.insert(time,vec![once_listener]);
            }
        }
    }

    fn notify(&mut self,key:ConfigKey) {
        if let Some(list) = self.listener.remove(&key) {
            for v in list {
                if let Some(sender) = self.sender_map.remove(&v) {
                    sender.send(ListenerResult::DATA(vec![key.clone()]));
                }
            }
        }
    }

    fn timeout(&mut self) {
        let current_time = Local::now().timestamp_millis();
        let mut keys:Vec<i64> = Vec::new();
        for (key,list) in self.time_listener.iter().take(10000) {
            if *key < current_time {
                keys.push(*key);
                for item in list {
                    let v = item.version;
                    if let Some(sender) = self.sender_map.remove(&v) {
                        sender.send(ListenerResult::NULL);
                    }
                }
            }
            else{
                break;
            }
        }
        for key in keys {
            self.time_listener.remove(&key);
        }
    }
}

pub struct ConfigActor {
    cache: HashMap<ConfigKey,ConfigValue>,
    listener: ConfigListener,
    subscriber: Subscriber,
    tenant_index:TenantIndex,
    config_db: ConfigDB,

}

impl ConfigActor {
    pub fn new() -> Self {
        let mut s=Self {
            cache: Default::default(),
            subscriber: Subscriber::new(),
            listener: ConfigListener::new(),
            tenant_index: Default::default(),
            config_db: ConfigDB::new(),
        };
        s.load_config();
        s
    }

    fn load_config(&mut self) {
        for item in self.config_db.query_config_list(){
            let key = ConfigKey::new(
                item.data_id.as_ref().unwrap(),
                item.group.as_ref().unwrap(),
                item.tenant.as_ref().unwrap()
            );
            let val = ConfigValue{
                content:Arc::new(item.content.unwrap()),
                md5:Arc::new(item.content_md5.unwrap()),
            };
            self.tenant_index.insert_config(key.clone());
            self.cache.insert(key, val);

        }
    }

    pub fn get_config_info_page(&self,param:&ConfigQueryParam) -> (usize,Vec<ConfigInfoDto>){
        let (size,list)=self.tenant_index.query_config_page(param);
        let mut info_list = Vec::with_capacity(size);
        for item in &list {
            if let Some(value)=self.cache.get(item){
                let mut info = ConfigInfoDto{
                    tenant:item.tenant.clone(),
                    group:item.group.clone(),
                    data_id:item.data_id.clone(),
                    //md5:Some(value.md5.clone()),
                    //content:Some(value.content.clone()),
                    ..Default::default()
                };
                if param.query_context {
                    info.content = Some(value.content.clone());
                    info.md5 = Some(value.md5.clone());
                }
                info_list.push(info);
            }
        }
        (size,info_list)
    }

    pub fn hb(&self,ctx:&mut actix::Context<Self>) {
        ctx.run_later(Duration::from_millis(500), |act,ctx|{
            act.listener.timeout();
            act.hb(ctx);
        });
    }
}

#[derive(Message)]
#[rtype(result= "Result<ConfigResult, std::io::Error>")]
pub enum ConfigCmd {
    ADD(ConfigKey,Arc<String>),
    GET(ConfigKey),
    DELETE(ConfigKey),
    QueryPageInfo(Box<ConfigQueryParam>),
    LISTENER(Vec<ListenerItem>,ListenerSenderType,i64),
    SetConnManage(Addr<BiStreamManage>),
    Subscribe(Vec<ListenerItem>,Arc<String>),
    RemoveSubscribe(Vec<ListenerItem>,Arc<String>),
    RemoveSubscribeClient(Arc<String>),
}

pub enum ConfigResult {
    DATA(Arc<String>,Arc<String>),
    NULL,
    ChangeKey(Vec<ConfigKey>),
    ConfigInfoPage(usize,Vec<ConfigInfoDto>),
}

impl Actor for ConfigActor {
    type Context = Context<Self>;

    fn started(&mut self,ctx: &mut Self::Context) {
        log::info!("ConfigActor started");
        self.hb(ctx);
    }
}

impl Supervised for ConfigActor {
    fn restarting(&mut self, _ctx: &mut <Self as Actor>::Context) {
        log::warn!("ConfigActor restart ...");
    }
}

impl Handler<ConfigCmd> for ConfigActor{
    type Result = Result<ConfigResult,std::io::Error>;

    fn handle(&mut self,msg: ConfigCmd, _ctx:&mut Context<Self>) -> Self::Result {
        match msg {
            ConfigCmd::ADD(key,val) =>{
                let config_val = ConfigValue::new(val);
                if let Some(v) = self.cache.get(&key) {
                    if v.md5 == config_val.md5 {
                        return Ok(ConfigResult::NULL);
                    }
                }
                self.config_db.update_config(&key, &config_val);
                self.cache.insert(key.clone(),config_val);
                self.tenant_index.insert_config(key.clone());
                self.listener.notify(key.clone());
                self.subscriber.notify(key);
            },
            ConfigCmd::DELETE(key) =>{
                self.cache.remove(&key);
                self.tenant_index.remove_config(&key);
                self.listener.notify(key.clone());
                self.subscriber.notify(key.clone());
                self.subscriber.remove_config_key(key);
            },
            ConfigCmd::GET(key) =>{
                if let Some(v) = self.cache.get(&key) {
                    return Ok(ConfigResult::DATA(v.content.clone(),v.md5.clone()));
                }
            },
            ConfigCmd::LISTENER(items,sender,time) => {
                let mut changes = vec![];
                for item in &items {
                    if let Some(v) = self.cache.get(&item.key) {
                        if v.md5!=item.md5 {
                            changes.push(item.key.clone());
                        }
                    }
                    else{
                        changes.push(item.key.clone());
                    }
                }
                if changes.len()> 0 || time<=0 {
                    sender.send(ListenerResult::DATA(changes));
                    return Ok(ConfigResult::NULL);
                }
                else{
                    self.listener.add(items,sender,time);
                    return Ok(ConfigResult::NULL);
                }
            },
            ConfigCmd::SetConnManage(conn_manage) => {
                self.subscriber.set_conn_manage(conn_manage);
            }
            ConfigCmd::Subscribe(items, client_id) => {
                let mut changes = vec![];
                for item in &items {
                    if let Some(v) = self.cache.get(&item.key) {
                        if v.md5!=item.md5 {
                            changes.push(item.key.clone());
                        }
                    }
                    else{
                        changes.push(item.key.clone());
                    }
                }
                self.subscriber.add_subscribe(client_id, items);
                if changes.len()> 0 {
                    return Ok(ConfigResult::ChangeKey(changes));
                }
            },
            ConfigCmd::RemoveSubscribe(items, client_id) => {
                self.subscriber.remove_subscribe(client_id, items);
            },
            ConfigCmd::RemoveSubscribeClient(client_id) => {
                self.subscriber.remove_client_subscribe(client_id);
            },
            ConfigCmd::QueryPageInfo(config_query_param) => {
                let (size,list) = self.get_config_info_page(config_query_param.as_ref());
                return Ok(ConfigResult::ConfigInfoPage(size,list));
            }
        }
        Ok(ConfigResult::NULL)
    }
}

