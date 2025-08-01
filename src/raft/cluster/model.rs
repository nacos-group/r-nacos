use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::{collections::HashMap, sync::Arc};

use crate::config::config_type::ConfigType;
use crate::namespace::model::{NamespaceRaftReq, NamespaceRaftResult};
use crate::raft::store::{ClientRequest, ClientResponse};
use crate::transfer::model::{TransferImportParam, TransferImportResponse};
use crate::{
    config::core::ConfigKey,
    raft::{
        cache::{CacheLimiterReq, CacheManagerResult},
        db::table::{TableManagerQueryReq, TableManagerReq, TableManagerResult},
    },
};

pub enum RouteAddr {
    Local,
    Remote(u64, Arc<String>),
    Unknown,
}

#[derive(Clone, Debug)]
pub struct SetConfigReq {
    pub config_key: ConfigKey,
    pub value: Arc<String>,
    pub op_user: Option<Arc<String>>,
    pub config_type: Option<Arc<String>>,
    pub desc: Option<Arc<String>>,
    //pub can_route_to_remote: bool,
    //pub extend_info: Option<HashMap<String,String>>,
}

impl SetConfigReq {
    pub fn new(config_key: ConfigKey, value: Arc<String>) -> Self {
        Self {
            config_key,
            value,
            op_user: None,
            config_type: None,
            desc: None,
        }
    }

    pub fn new_with_op_user(
        config_key: ConfigKey,
        value: Arc<String>,
        op_user: Arc<String>,
    ) -> Self {
        Self {
            config_key,
            value,
            op_user: Some(op_user),
            config_type: None,
            desc: None,
        }
    }

    pub fn detect_config_type(data_id: Arc<String>) -> Option<Arc<String>> {
        if let Some(pos) = data_id.rfind('.') {
            let suffix = &data_id[pos + 1..];

            if !suffix.is_empty() {
                return Some(ConfigType::new_by_value(suffix).get_value());
            }
        }

        None
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
        Self { config_key }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RouterRequest {
    RaftRequest(ClientRequest),
    ConfigSet {
        key: String,
        value: Arc<String>,
        op_user: Option<Arc<String>>,
        config_type: Option<Arc<String>>,
        desc: Option<Arc<String>>,
        extend_info: HashMap<String, String>,
    },
    ConfigDel {
        key: String,
        extend_info: HashMap<String, String>,
    },
    JoinNode {
        node_id: u64,
        node_addr: Arc<String>,
    },
    TableManagerReq {
        req: TableManagerReq,
    },
    TableManagerQueryReq {
        req: TableManagerQueryReq,
    },
    CacheLimiterReq {
        req: CacheLimiterReq,
    },
    NamespaceReq {
        req: NamespaceRaftReq,
    },
    ImportData {
        data: Vec<u8>,
        param: TransferImportParam,
    },
}

impl From<SetConfigReq> for RouterRequest {
    fn from(req: SetConfigReq) -> Self {
        Self::ConfigSet {
            key: req.config_key.build_key(),
            value: req.value,
            op_user: req.op_user,
            config_type: req.config_type,
            desc: req.desc,
            extend_info: Default::default(),
        }
    }
}

impl From<DelConfigReq> for RouterRequest {
    fn from(req: DelConfigReq) -> Self {
        Self::ConfigDel {
            key: req.config_key.build_key(),
            extend_info: Default::default(),
        }
    }
}

impl From<CacheLimiterReq> for RouterRequest {
    fn from(req: CacheLimiterReq) -> Self {
        Self::CacheLimiterReq { req }
    }
}

impl From<NamespaceRaftReq> for RouterRequest {
    fn from(req: NamespaceRaftReq) -> Self {
        Self::NamespaceReq { req }
    }
}

impl From<ClientRequest> for RouterRequest {
    fn from(req: ClientRequest) -> Self {
        RouterRequest::RaftRequest(req)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RouterResponse {
    None,
    RaftResponse(ClientResponse),
    TableManagerResult { result: TableManagerResult },
    CacheManagerResult { result: CacheManagerResult },
    NamespaceResult { result: NamespaceRaftResult },
    ImportResult { result: TransferImportResponse },
}

impl From<ClientResponse> for RouterResponse {
    fn from(resp: ClientResponse) -> Self {
        RouterResponse::RaftResponse(resp)
    }
}

impl TryFrom<RouterResponse> for ClientResponse {
    type Error = anyhow::Error;

    fn try_from(value: RouterResponse) -> Result<Self, Self::Error> {
        match value {
            RouterResponse::RaftResponse(resp) => Ok(resp),
            _ => Err(anyhow::anyhow!("Invalid RaftResponse")),
        }
    }
}
