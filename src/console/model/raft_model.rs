use std::collections::BTreeSet;

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfo{
    pub node_id: Option<u64>,
    pub node_addr: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NodeMember{
    pub node_ids: Vec<u64>,
}