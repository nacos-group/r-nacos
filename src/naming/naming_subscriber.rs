
use std::{collections::{HashMap, HashSet}, sync::Arc};

use actix::prelude::*;
use crate::grpc::bistream_manage::{BiStreamManage, BiStreamManageCmd};

use super::{model::{ServiceKey, Instance, ServiceInfo}, naming_delay_nofity::{DelayNotifyActor, DelayNotifyCmd}};

#[derive(Debug, Clone, Hash, Eq)]
pub enum ListenerClusterType {
    All,
    One(Arc<String>)
}

impl Default for ListenerClusterType {
    fn default() -> Self {
        Self::All
    }
}

impl PartialEq for ListenerClusterType {
    fn eq(&self, o:&Self) -> bool {
        match &self {
            ListenerClusterType::All => {
                match &o{
                    ListenerClusterType::All => {
                        true
                    },
                    ListenerClusterType::One(_) => {
                        false
                    },
                }
            },
            ListenerClusterType::One(a) => {
                match &o{
                    ListenerClusterType::All => {
                        false
                    },
                    ListenerClusterType::One(b) => {
                        a==b
                    },
                }
            },
        }
    }
}

#[derive(Debug,Clone,Default,Hash,Eq)]
pub struct ListenerKey{
    pub namespace_id:String,
    pub group_name:String,
    pub service_name:String,
    pub cluster:ListenerClusterType,
}

impl PartialEq for ListenerKey {
    fn eq(&self, o:&Self) -> bool {
        self.namespace_id == o.namespace_id
        && self.group_name == o.group_name
        && self.service_name == o.service_name
        && self.cluster == o.cluster
    }
}

#[derive(Debug)]
pub struct NamingListenerItem {
    pub service_key:ServiceKey,
    pub clusters:Option<HashSet<String>>,
}


#[derive(Default)]
pub struct Subscriber {
    listener: HashMap<ServiceKey,HashMap<Arc<String>,Option<HashSet<String>>>>,
    client_keys: HashMap<Arc<String>,HashSet<ServiceKey>>,
    notify_addr: Option<Addr<DelayNotifyActor>>,
}

impl Subscriber {
    pub fn new() -> Self {
        Self {
            listener: Default::default(),
            client_keys: Default::default(),
            notify_addr:Default::default(),
        }
    }

    pub fn set_notify_addr(&mut self,notify_addr:Addr<DelayNotifyActor>) {
        self.notify_addr = Some(notify_addr);
    }

    pub fn add_subscribe(&mut self,client_id:Arc<String>,items:Vec<NamingListenerItem>) {
        match self.client_keys.get_mut(&client_id) {
            Some(set) => {
                for item in &items {
                    set.insert(item.service_key.clone());
                }
            }
            None => {
                let mut set = HashSet::new();
                for item in &items {
                    set.insert(item.service_key.clone());
                }
                self.client_keys.insert(client_id.clone(),set);
            }
        }
        for item in items {
            match self.listener.get_mut(&item.service_key) {
                Some(set) => {
                    set.insert(client_id.clone(),item.clusters);
                },
                None => {
                    let mut set = HashMap::new();
                    set.insert(client_id.clone(),item.clusters);
                    self.listener.insert(item.service_key,set);
                }
            };
        }
        
    }

    pub fn remove_subscribe(&mut self,client_id:Arc<String>,items:Vec<NamingListenerItem>) {
        let mut remove_keys = vec![];
        for item in &items {
            match self.listener.get_mut(&item.service_key) {
                Some(set) => {
                    set.remove(&client_id);
                    if set.len() == 0 {
                        remove_keys.push(item.service_key.clone());
                    }
                },
                None => {}
            };
        }
        for key in &remove_keys {
            self.listener.remove(key);
        }

        let mut remove_empty_client = false;
        match self.client_keys.get_mut(&client_id) {
            Some(set) => {
                for item in items {
                    set.remove(&item.service_key);
                }
                if set.len() == 0 {
                    remove_empty_client=true;
                }
            }
            None => {}
        };
        if remove_empty_client {
            self.client_keys.remove(&client_id);
        }
    }

    pub fn remove_client_subscribe(&mut self,client_id:Arc<String>) {
        if let Some(set)=self.client_keys.remove(&client_id) {
            let mut remove_keys = vec![];
            for key in set{
                match self.listener.get_mut(&key) {
                    Some(set) => {
                        set.remove(&client_id);
                        if set.len() == 0 {
                            remove_keys.push(key);
                        }
                    },
                    None => {}
                };
            }
            for key in &remove_keys {
                self.listener.remove(key);
            }
        }
    }

    pub fn remove_key(&mut self,key:ServiceKey) {
        if let Some(set) = self.listener.remove(&key) {
            let mut remove_keys = vec![];
            for (client_id,_) in set {
                match self.client_keys.get_mut(&client_id) {
                    Some(set) => {
                        set.remove(&key);
                        if set.len() == 0 {
                            remove_keys.push(client_id);
                        }
                    },
                    None => {}
                }
            }
            for key in &remove_keys {
                self.client_keys.remove(key);
            }
        }
    }

    pub fn notify(&self,key:ServiceKey,service_info:ServiceInfo) {
        if let Some(notify_addr) = &self.notify_addr {
            if let Some(set) = self.listener.get(&key) {
                let mut client_id_set = HashSet::new();
                for item in set.keys() {
                    client_id_set.insert(item.clone());
                }
                notify_addr.do_send(DelayNotifyCmd::Notify(key, client_id_set,service_info));
            }
        }
    }

}
