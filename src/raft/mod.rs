use async_raft::{Raft, RaftStorage};

use self::{store::{ClientRequest, ClientResponse, store::RaftStore}};
use self::network::network::RaftRouter;

pub mod cluster;
pub mod store;
pub mod network;

pub type NacosRaft = Raft<ClientRequest, ClientResponse, RaftRouter, RaftStore>;

pub async fn join_node(raft:&NacosRaft, raft_store:&RaftStore, node_id:u64) -> anyhow::Result<()> {
    let membership=raft_store.get_membership_config().await?;
    if !membership.contains(&node_id) {
        let mut all_node=  membership.all_nodes();
        all_node.insert(node_id);
        raft.change_membership(all_node).await.unwrap();
    }
    Ok(())
}
