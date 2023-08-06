use std::collections::BTreeSet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfo{
    pub node_id: Option<u64>,
    pub node_addr: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NodeMember{
    pub node_ids: String,
}

impl NodeMember {
    pub fn get_member(&self)  -> BTreeSet<u64> {
        let ids :Vec<u64> =self.node_ids.split(',').map(|e|e.parse().unwrap_or_default()).collect();
        let mut set = BTreeSet::new();
        for id in ids {
            if id > 0 {
                set.insert(id);
            }
        }
        set
    }
}