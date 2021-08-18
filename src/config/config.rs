
use chrono::Local;
use std::collections::HashMap;
use std::collections::BTreeMap;

use crate::utils::get_md5;

use actix::prelude::*;

use super::config_db::ConfigDB;

#[derive(Debug,Hash,Eq,Clone)]
pub struct ConfigKey {
    pub(crate) data_id:String,
    pub(crate) group:String,
    pub(crate) tenant:String,
}

impl ConfigKey {
    pub fn new(data_id:&str,group:&str,tenant:&str) -> ConfigKey {
        ConfigKey {
            data_id:data_id.to_owned(),
            group:group.to_owned(),
            tenant:tenant.to_owned(),
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
    pub(crate) content:String,
    pub(crate) md5:String,
}

impl ConfigValue {
    fn new(content:String) -> Self{
        let md5 = get_md5(&content);
        Self {
            content,
            md5,
        }
    }
}

#[derive(Debug)]
pub struct ListenerItem {
    pub key:ConfigKey,
    pub md5:String,
}

impl ListenerItem {
    pub fn new(key:ConfigKey,md5:String) -> Self {
        Self {
            key,
            md5,
        }
    }

    pub fn decode_listener_items(configs:&str) -> Vec::<Self> {
        let mut list = vec![];
        let mut start = 0;
        let bytes = configs.as_bytes();
        let mut tmpList = vec![];
        for i in 0..bytes.len(){
            let char = bytes[i];
            if char == 2 {
                if tmpList.len() > 2{
                    continue;
                }
                tmpList.push(String::from_utf8(bytes[start..i].to_vec()).unwrap());
                start = i+1;
            }
            else if char == 1 {
                let mut endValue = String::new();
                if start+1 <=i {
                    endValue = String::from_utf8(bytes[start..i].to_vec()).unwrap();
                }
                start = i+1;
                if tmpList.len() == 2 {
                    let key = ConfigKey::new(&tmpList[0],&tmpList[1],"");
                    list.push(ListenerItem::new(key,endValue));
                }
                else{
                    if endValue=="public" {
                        endValue="".to_owned();
                    }
                    let key = ConfigKey::new(&tmpList[0],&tmpList[1],&endValue);
                    list.push(ListenerItem::new(key,tmpList[2].to_owned()));
                }
                tmpList.clear();
            }
        }
        list
    }

    pub fn decode_listener_change_keys(configs:&str) -> Vec::<ConfigKey> {
        let mut list = vec![];
        let mut start = 0;
        let bytes = configs.as_bytes();
        let mut tmpList = vec![];
        for i in 0..bytes.len(){
            let char = bytes[i];
            if char == 2 {
                if tmpList.len() > 2{
                    continue;
                }
                tmpList.push(String::from_utf8(bytes[start..i].to_vec()).unwrap());
                start = i+1;
            }
            else if char == 1 {
                let mut endValue = String::new();
                if start+1 <=i {
                    endValue = String::from_utf8(bytes[start..i].to_vec()).unwrap();
                }
                start = i+1;
                if tmpList.len() == 1 {
                    let key = ConfigKey::new(&tmpList[0],&endValue,"");
                    list.push(key);
                }
                else{
                    let key = ConfigKey::new(&tmpList[0],&tmpList[1],&endValue);
                    list.push(key);
                }
                tmpList.clear();
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
type ListenerReceiverType = tokio::sync::oneshot::Receiver<ListenerResult>;

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
    config_db: ConfigDB,
}

impl ConfigActor {
    pub fn new() -> Self {
        Self {
            cache: Default::default(),
            listener: ConfigListener::new(),
            config_db: ConfigDB::new(),
        }
    }
}

#[derive(Message)]
#[rtype(result= "Result<ConfigResult, std::io::Error>")]
pub enum ConfigCmd {
    ADD(ConfigKey,String),
    GET(ConfigKey),
    DELETE(ConfigKey),
    LISTENER(Vec<ListenerItem>,ListenerSenderType,i64),
    PEEK_LISTENER_TIME_OUT,
}

pub enum ConfigResult {
    DATA(String),
    NULL,
}

impl Actor for ConfigActor {
    type Context = Context<Self>;
}

impl Handler<ConfigCmd> for ConfigActor{
    type Result = Result<ConfigResult,std::io::Error>;

    fn handle(&mut self,msg: ConfigCmd, ctx:&mut Context<Self>) -> Self::Result {
        match msg {
            ConfigCmd::ADD(key,val) =>{
                self.cache.insert(key.clone(),ConfigValue::new(val));
                self.listener.notify(key);
            },
            ConfigCmd::DELETE(key) =>{
                self.cache.remove(&key);
                self.listener.notify(key);
            },
            ConfigCmd::GET(key) =>{
                if let Some(v) = self.cache.get(&key) {
                    return Ok(ConfigResult::DATA(v.content.to_owned()));
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
            ConfigCmd::PEEK_LISTENER_TIME_OUT => {
                self.listener.timeout();
            }
        }
        Ok(ConfigResult::NULL)
    }
}

