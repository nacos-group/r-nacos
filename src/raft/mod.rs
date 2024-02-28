use async_raft_ext::{Raft, RaftStorage};
use async_raft_ext::raft::ClientWriteRequest;
use crate::raft::filestore::core::FileStore;

use self::network::core::RaftRouter;
use self::store::{core::RaftStore, ClientRequest, ClientResponse};

pub mod cache;
pub mod cluster;
pub mod db;
pub mod filestore;
pub mod network;
pub mod store;

//pub type NacosRaft = Raft<ClientRequest, ClientResponse, RaftRouter, RaftStore>;
pub type NacosRaft = Raft<ClientRequest, ClientResponse, RaftRouter, FileStore>;

pub async fn join_node(
    raft: &NacosRaft,
    raft_store: &FileStore,
    node_id: u64,
) -> anyhow::Result<()> {
    let membership = raft_store.get_membership_config().await?;
    if !membership.contains(&node_id) {
        let mut all_node = membership.all_nodes();
        if all_node.contains(&node_id) {
            return Ok(())
        }
        all_node.insert(node_id);
        let members = all_node.clone().into_iter().collect();
        log::info!("join_node membership,{:?}",&all_node);
        raft.change_membership(all_node).await.ok();
        raft.client_write(ClientWriteRequest::new(ClientRequest::Members(members)))
            .await
            .unwrap();
    }
    Ok(())
}
