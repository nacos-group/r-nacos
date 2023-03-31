
use std::collections::HashMap;

use serde::{Serialize,Deserialize};

use crate::now_millis_i64;

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct Instance {
    pub id:String,
    pub ip:String,
    pub port:u32,
    pub weight:f32,
    pub enabled:bool,
    pub healthy:bool,
    pub ephemeral: bool,
    pub cluster_name:String,
    pub service_name:String,
    pub group_name:String,
    pub metadata:HashMap<String,String>,
    pub last_modified_millis:i64,
    pub namespace_id:String,
    pub app_name:String,
}

impl Instance{
    pub fn new(ip:String,port:u32) -> Self {
        let mut s = Self::default();
        s.ip = ip;
        s.port = port;
        s
    }

    pub fn generate_key(&mut self) {
        self.id = format!("{}#{}#{}#{}#{}",&self.ip,&self.port,&self.cluster_name,&self.service_name,&self.group_name)
    }

    pub fn init(&mut self) {
        self.last_modified_millis = now_millis_i64();
        if self.id.len()==0 {
            self.generate_key();
        }
    }

    pub fn check_vaild(&self) -> bool {
        if self.id.len()==0 || self.port<=0 || self.service_name.len()==0 || self.cluster_name.len()==0 
        {
            return false;
        }
        true
    }

    pub fn update_info(&mut self,o:Self,tag:Option<InstanceUpdateTag>) -> bool {
        let is_update=self.enabled != o.enabled 
        || self.healthy != o.healthy
        || self.weight != o.weight
        || self.ephemeral != o.ephemeral
        || self.metadata != o.metadata
        ;
        self.healthy = o.healthy;
        self.last_modified_millis = o.last_modified_millis;
        if let Some(tag) = tag {
            if tag.weight {
                self.weight = o.weight;
            }
            if tag.metadata {
                self.metadata = o.metadata;
            }
            if tag.enabled {
                self.enabled = o.enabled;
            }
            if tag.ephemeral {
                self.ephemeral = o.ephemeral;
            }
        }
        else{
            self.weight = o.weight;
            self.metadata = o.metadata;
            self.enabled = o.enabled;
            self.ephemeral = o.ephemeral;
        }
        is_update
    }

    pub fn get_service_key(&self) -> ServiceKey {
        ServiceKey::new(&self.namespace_id,&self.group_name,&self.service_name)
    }

    pub(crate) fn get_time_info(&self) -> InstanceTimeInfo {
        InstanceTimeInfo::new(self.id.to_owned(),self.last_modified_millis)
    }
}

impl Default for Instance {
    fn default() -> Self {
        Self {
            id:Default::default(),
            ip:Default::default(),
            port:Default::default(),
            weight:1f32,
            enabled:true,
            healthy:true,
            ephemeral:true,
            cluster_name:"DEFAULT".to_owned(),
            service_name:Default::default(),
            group_name:Default::default(),
            metadata:Default::default(),
            last_modified_millis:Default::default(),
            namespace_id:Default::default(),
            app_name:Default::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default,Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    pub name: Option<String>,
    pub group_name: Option<String>,
    pub clusters: Option<String>,
    pub cache_millis: i64,
    pub hosts: Vec<Instance>,
    pub last_ref_time: i64,
    pub checksum: i64,
    #[serde(rename = "allIPs")]
    pub all_ips:bool,
    pub reach_protection_threshold: bool,
}

#[derive(Debug,Clone)]
pub struct InstanceUpdateTag{
    pub weight: bool,
    pub metadata: bool,
    pub enabled: bool,
    pub ephemeral: bool,
}

impl InstanceUpdateTag {
    pub fn is_al(&self) -> bool {
        self.weight && self.metadata && self.enabled && self.ephemeral
    }
    pub fn is_none(&self) -> bool {
        !self.weight && !self.metadata && !self.enabled && !self.ephemeral
    }
}

impl Default for InstanceUpdateTag {
    fn default() -> Self {
        Self {
            weight: true,
            metadata: true,
            enabled: true,
            ephemeral: true,
        }
    }
}


#[derive(Debug,Clone,Default,Hash,Eq)]
pub struct ServiceKey {
    pub namespace_id:String,
    pub group_name:String,
    pub service_name:String,
}

impl PartialEq for ServiceKey {
    fn eq(&self, o:&Self) -> bool {
        self.namespace_id == o.namespace_id
        && self.group_name == o.group_name
        && self.service_name == o.service_name
    }
}


impl ServiceKey {
    pub fn new(namespace_id:&str,group_name:&str,serivce_name:&str) -> Self {
        Self {
            namespace_id:namespace_id.to_owned(),
            group_name:group_name.to_owned(),
            service_name:serivce_name.to_owned(),
        }
    }

    pub fn get_join_service_name(&self) -> String {
        format!("{}@@{}",self.group_name,self.service_name)
    }

    pub fn get_namespace_group(&self) -> String {
        format!("{}#{}",self.namespace_id,self.group_name)
    }
}


#[derive(Debug,Clone,Default,Hash,Eq)]
pub(crate) struct InstanceTimeInfo {
    pub(crate) time:i64,
    pub(crate) instance_id:String,
    pub(crate) enable:bool,
}

impl InstanceTimeInfo {
    pub(crate) fn new(instance_id:String,time:i64) -> Self {
        Self {
            time,
            instance_id,
            enable:true,
        }
    }
}

impl PartialEq for InstanceTimeInfo {
    fn eq(&self, o:&Self) -> bool {
        self.time == o.time
    }
}

pub enum UpdateInstanceType{
    None,
    New,
    Remove,
    UpdateTime,
    UpdateValue,
}
