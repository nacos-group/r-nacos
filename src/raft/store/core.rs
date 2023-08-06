use std::io::Cursor;
use std::sync::Arc;

use crate::config::core::ConfigActor;

use super::ClientRequest;
use super::ClientResponse;
use super::{
    innerstore::{InnerStore, StoreRequest, StoreResponse},
    ShutdownError,
};

use actix::prelude::*;
use async_raft::raft::{Entry, MembershipConfig};
use async_raft::storage::{CurrentSnapshotData, HardState, InitialState};
use async_raft::RaftStorage;
use async_trait::async_trait;

fn init_inner_store(inner: InnerStore) -> Addr<InnerStore> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let rt = System::new();
        let addrs = rt.block_on(async { inner.start() });
        tx.send(addrs).unwrap();
        rt.run().unwrap();
    });
    rx.recv().unwrap()
}

#[derive(Clone)]
pub struct RaftStore {
    inner_addr: Addr<InnerStore>,
}

impl RaftStore {
    pub fn new(id: u64, db: Arc<sled::Db>, config_addr: Addr<ConfigActor>) -> Self {
        let inner_addr = init_inner_store(InnerStore::new(id, db, config_addr));
        Self { inner_addr }
    }

    fn store_response_err(&self) -> anyhow::Error {
        anyhow::anyhow!("store_response_err")
    }

    pub async fn send_store_msg(&self, msg: StoreRequest) -> anyhow::Result<StoreResponse> {
        self.inner_addr.send(msg).await?
    }

    pub async fn get_state_value(&self, _key: String) -> anyhow::Result<Option<Arc<String>>> {
        Ok(None)
    }

    pub async fn get_target_addr(&self, id: u64) -> anyhow::Result<Arc<String>> {
        match self.send_store_msg(StoreRequest::GetTargetAddr(id)).await? {
            StoreResponse::TargetAddr(Some(v)) => Ok(v),
            _ => Err(anyhow::anyhow!("get_state_value error")),
        }
    }
}

#[async_trait]
impl RaftStorage<ClientRequest, ClientResponse> for RaftStore {
    type Snapshot = Cursor<Vec<u8>>;
    type ShutdownError = ShutdownError;

    async fn get_membership_config(&self) -> anyhow::Result<MembershipConfig> {
        match self
            .send_store_msg(StoreRequest::GetMembershipConfig)
            .await?
        {
            StoreResponse::MembershipConfig(v) => Ok(v),
            _ => Err(self.store_response_err()),
        }
    }

    async fn get_initial_state(&self) -> anyhow::Result<InitialState> {
        match self.send_store_msg(StoreRequest::GetInitialState).await? {
            StoreResponse::InitialState(v) => Ok(v),
            _ => Err(self.store_response_err()),
        }
    }

    async fn save_hard_state(&self, hs: &HardState) -> anyhow::Result<()> {
        match self
            .send_store_msg(StoreRequest::SaveHardState(hs.to_owned()))
            .await?
        {
            StoreResponse::None => Ok(()),
            _ => Err(self.store_response_err()),
        }
    }

    async fn get_log_entries(
        &self,
        start: u64,
        stop: u64,
    ) -> anyhow::Result<Vec<Entry<ClientRequest>>> {
        match self
            .send_store_msg(StoreRequest::GetLogEntries { start, stop })
            .await?
        {
            StoreResponse::LogEntries(v) => Ok(v),
            _ => Err(self.store_response_err()),
        }
    }

    async fn delete_logs_from(&self, start: u64, stop: Option<u64>) -> anyhow::Result<()> {
        match self
            .send_store_msg(StoreRequest::DeleteLogsFrom { start, stop })
            .await?
        {
            StoreResponse::None => Ok(()),
            _ => Err(self.store_response_err()),
        }
    }

    async fn append_entry_to_log(&self, entry: &Entry<ClientRequest>) -> anyhow::Result<()> {
        match self
            .send_store_msg(StoreRequest::AppendEntryToLog(entry.to_owned()))
            .await?
        {
            StoreResponse::None => Ok(()),
            _ => Err(self.store_response_err()),
        }
    }

    async fn replicate_to_log(&self, entries: &[Entry<ClientRequest>]) -> anyhow::Result<()> {
        let entries = entries.iter().map(|e| e.to_owned()).collect();
        match self
            .send_store_msg(StoreRequest::ReplicateToLog(entries))
            .await?
        {
            StoreResponse::None => Ok(()),
            _ => Err(self.store_response_err()),
        }
    }

    async fn apply_entry_to_state_machine(
        &self,
        index: &u64,
        _data: &ClientRequest,
    ) -> anyhow::Result<ClientResponse> {
        //match self.send_store_msg(StoreRequest::ApplyEntryToStateMachine{index:index.to_owned(),request:data.to_owned()}).await? {
        match self
            .send_store_msg(StoreRequest::ApplyEntryToStateMachineRange {
                start: *index,
                end: (*index) + 1,
            })
            .await?
        {
            StoreResponse::Response(v) => Ok(v),
            StoreResponse::None => Ok(ClientResponse { value: None }),
            _ => Err(self.store_response_err()),
        }
    }

    async fn replicate_to_state_machine(
        &self,
        entries: &[(&u64, &ClientRequest)],
    ) -> anyhow::Result<()> {
        //let entries = entries.iter().map(|(i,r)|((*i).to_owned(),(*r).to_owned())).collect();
        //match self.send_store_msg(StoreRequest::ReplicateToStateMachine(entries)).await? {
        let len = entries.len();
        if len == 0 {
            return Ok(());
        }
        match self
            .send_store_msg(StoreRequest::ApplyEntryToStateMachineRange {
                start: *entries[0].0,
                end: *entries[len - 1].0 + 1,
            })
            .await?
        {
            StoreResponse::None => Ok(()),
            _ => Err(self.store_response_err()),
        }
    }

    async fn do_log_compaction(&self) -> anyhow::Result<CurrentSnapshotData<Self::Snapshot>> {
        match self.send_store_msg(StoreRequest::DoLogCompaction).await? {
            StoreResponse::Snapshot(v) => Ok(v),
            _ => Err(self.store_response_err()),
        }
    }

    async fn create_snapshot(&self) -> anyhow::Result<(String, Box<Self::Snapshot>)> {
        //Ok((uuid::Uuid::new_v4().to_string().replace("-",""), Box::new(Cursor::new(Vec::new())))) // Snapshot IDs are insignificant to this storage engine.
        Ok(("".to_owned(), Box::new(Cursor::new(Vec::new())))) // Snapshot IDs are insignificant to this storage engine.
    }

    async fn finalize_snapshot_installation(
        &self,
        index: u64,
        term: u64,
        delete_through: Option<u64>,
        id: String,
        snapshot: Box<Self::Snapshot>,
    ) -> anyhow::Result<()> {
        match self
            .send_store_msg(StoreRequest::FinalizeSnapshotInstallation {
                index,
                term,
                delete_through,
                id,
                snapshot,
            })
            .await?
        {
            StoreResponse::None => Ok(()),
            _ => Err(self.store_response_err()),
        }
    }

    async fn get_current_snapshot(
        &self,
    ) -> anyhow::Result<Option<CurrentSnapshotData<Self::Snapshot>>> {
        match self
            .send_store_msg(StoreRequest::GetCurrentSnapshot)
            .await?
        {
            StoreResponse::OptionSnapshot(v) => Ok(v),
            _ => Err(self.store_response_err()),
        }
    }
}
