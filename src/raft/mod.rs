use async_raft::{Raft, RaftStorage};

use self::asyncraft::{store::{ClientRequest, ClientResponse, store::AStore}};
use self::asyncraft::network::network::RaftRouter;

pub mod asyncraft;
pub mod cluster;
pub type NacosRaft = Raft<ClientRequest, ClientResponse, RaftRouter, AStore>;

pub async fn join_node(raft:&NacosRaft,raft_store:&AStore,node_id:u64) -> anyhow::Result<()> {
    let membership=raft_store.get_membership_config().await?;
    if !membership.contains(&node_id) {
        let mut all_node=  membership.all_nodes();
        all_node.insert(node_id);
        raft.change_membership(all_node).await.unwrap();
    }
    Ok(())
}