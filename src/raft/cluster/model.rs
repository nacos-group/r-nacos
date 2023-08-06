
use std::{sync::Arc, collections::HashMap};

use serde::{Serialize, Deserialize};

use crate::config::core::ConfigKey;

pub enum RouteAddr {
    Local,
    Remote(u64,Arc<String>),
    Unknown,
}

#[derive(Clone, Debug)]
pub struct SetConfigReq {
    pub config_key: ConfigKey,
    pub value: Arc<String>, 
    //pub can_route_to_remote: bool,
    //pub extend_info: Option<HashMap<String,String>>,
}

impl SetConfigReq {
    pub fn new(config_key:ConfigKey,value:Arc<String>) -> Self {
        Self {
            config_key,
            value,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DelConfigReq {
    pub config_key: ConfigKey,
    //pub can_route_to_remote: bool,
    //pub extend_info: Option<HashMap<String,String>>,
}

impl DelConfigReq {
    pub fn new(config_key: ConfigKey) -> Self {
        Self {
            config_key
        }
    }
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RouterRequest {
    ConfigSet {
        key: String,
        value: Arc<String>,
        extend_info: HashMap<String,String>,
    },
    ConfigDel {
        key: String,
        extend_info: HashMap<String,String>,
    },
}

impl From<SetConfigReq> for RouterRequest {
    fn from(req: SetConfigReq) -> Self {
        Self::ConfigSet { 
            key:req.config_key.build_key(), 
            value: req.value, 
            extend_info: Default::default() 
        }
    }
}

impl From<DelConfigReq> for RouterRequest {
    fn from(req: DelConfigReq) -> Self {
        Self::ConfigDel {
            key:req.config_key.build_key(), 
            extend_info: Default::default() 
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RouterResponse {
    None,
}