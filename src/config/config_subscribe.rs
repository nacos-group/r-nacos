use std::{collections::{HashMap, HashSet}, sync::Arc};

use actix::prelude::*;
use crate::grpc::bistream_manage::{BiStreamManage, BiStreamManageCmd};
use super::core::{ConfigKey, ListenerItem};



#[derive(Default)]
pub struct Subscriber {
    listener: HashMap<ConfigKey,HashSet<Arc<String>>>,
    client_keys: HashMap<Arc<String>,HashSet<ConfigKey>>,
    conn_manage: Option<Addr<BiStreamManage>>,
}

impl Subscriber {
    pub fn new() -> Self {
        Self {
            listener: Default::default(),
            client_keys: Default::default(),
            conn_manage:Default::default(),
        }
    }

    pub fn set_conn_manage(&mut self,conn_manage:Addr<BiStreamManage>) {
        self.conn_manage = Some(conn_manage);
    }

    pub fn add_subscribe(&mut self,client_id:Arc<String>,items:Vec<ListenerItem>) {
        for item in &items {
            match self.listener.get_mut(&item.key) {
                Some(set) => {
                    set.insert(client_id.clone());
                },
                None => {
                    let mut set = HashSet::new();
                    set.insert(client_id.clone());
                    self.listener.insert(item.key.clone(),set);
                }
            };
        }
        match self.client_keys.get_mut(&client_id) {
            Some(set) => {
                for item in items {
                    set.insert(item.key);
                }
            }
            None => {
                let mut set = HashSet::new();
                for item in items {
                    set.insert(item.key);
                }
                self.client_keys.insert(client_id,set);
            }
        }
    }

    pub fn remove_subscribe(&mut self,client_id:Arc<String>,items:Vec<ListenerItem>) {
        let mut remove_keys = vec![];
        for item in &items {
            if let Some(set) = self.listener.get_mut(&item.key) {
                set.remove(&client_id);
                if set.is_empty() {
                    remove_keys.push(item.key.clone());
                }
            };
        }
        for key in &remove_keys {
            self.listener.remove(key);
        }

        let mut remove_empty_client = false;
        if let Some(set) = self.client_keys.get_mut(&client_id) {
            for item in items {
                set.remove(&item.key);
            }
            if set.is_empty() {
                remove_empty_client=true;
            }
        };
        if remove_empty_client {
            self.client_keys.remove(&client_id);
        }
    }

    pub fn remove_client_subscribe(&mut self,client_id:Arc<String>) {
        if let Some(set)=self.client_keys.remove(&client_id) {
            let mut remove_keys = vec![];
            for key in set{
                if let Some(set) = self.listener.get_mut(&key) {
                    set.remove(&client_id);
                    if set.is_empty() {
                        remove_keys.push(key);
                    }
                }
            }
            for key in &remove_keys {
                self.listener.remove(key);
            }
        }
    }

    pub fn remove_config_key(&mut self,key:ConfigKey) {
        if let Some(set) = self.listener.remove(&key) {
            let mut remove_keys = vec![];
            for client_id in set {
                if let Some(set) = self.client_keys.get_mut(&client_id) { 
                    set.remove(&key);
                    if set.is_empty() {
                        remove_keys.push(client_id);
                    }
                }
            }
            for key in &remove_keys {
                self.client_keys.remove(key);
            }
        }
    }

    pub fn notify(&self,key:ConfigKey) {
        if let Some(conn_manage) = &self.conn_manage {
            if let Some(set) = self.listener.get(&key) {
                conn_manage.do_send(BiStreamManageCmd::NotifyConfig(key, set.clone()));
            }
        }
    }

}