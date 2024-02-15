use actix::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;
use async_raft_ext::raft::{Entry, MembershipConfig};
use async_raft_ext::RaftStorage;
use async_raft_ext::storage::{CurrentSnapshotData, HardState, InitialState};
use async_trait::async_trait;
use crate::raft::filestore::model::{ApplyRequestDto, LogIndexInfo};
use crate::raft::filestore::raftapply::{StateApplyAsyncRequest, StateApplyManager, StateApplyRequest, StateApplyResponse};
use crate::raft::filestore::raftindex::{RaftIndexManager, RaftIndexRequest, RaftIndexResponse};
use crate::raft::filestore::raftlog::{RaftLogManager, RaftLogManagerAsyncRequest, RaftLogManagerRequest, RaftLogResponse};
use crate::raft::filestore::raftsnapshot::{RaftSnapshotManager, RaftSnapshotRequest, RaftSnapshotResponse};
use crate::raft::filestore::StoreUtils;
use crate::raft::store::{ClientRequest, ClientResponse, ShutdownError};

pub fn vec_to_set(list: &Vec<u64>) -> HashSet<u64> {
    let mut set = HashSet::new();
    for item in list {
        set.insert(item.to_owned());
    }
    set
}

#[derive(Clone)]
pub struct FileStore {
    node_id: u64,
    //inner_addr: Addr<InnerStore>,
    index_manager: Addr<RaftIndexManager>,
    snapshot_manager: Addr<RaftSnapshotManager>,
    log_manager: Addr<RaftLogManager>,
    //data_store: Addr<RaftDataStore>,
    apply_manager: Addr<StateApplyManager>,
}

impl FileStore {
    pub fn new(
        node_id: u64,
        index_manager: Addr<RaftIndexManager>,
        snapshot_manager: Addr<RaftSnapshotManager>,
        log_manager: Addr<RaftLogManager>,
        //data_store: Addr<RaftDataStore>,
        apply_manager: Addr<StateApplyManager>,
    ) -> Self {
        Self {
            node_id,
            index_manager,
            snapshot_manager,
            log_manager,
            //data_store,
            apply_manager,
        }
    }

    async fn get_last_log_index(&self) -> anyhow::Result<LogIndexInfo> {
        match self
            .log_manager
            .send(RaftLogManagerAsyncRequest::GetLastLogIndex)
            .await??
        {
            RaftLogResponse::LastLogIndex(last_index) => Ok(last_index),
            _ => Ok(LogIndexInfo::default()),
        }
    }

    /*
    pub async fn get_state_value(&self, key: String) -> anyhow::Result<Option<Arc<String>>> {
        if let RaftDataStoreResponse::Value(v) = self
            .data_store
            .send(crate::common::raftstore::RaftDataStoreRequest::Get(
                Arc::new(key),
            ))
            .await??
        {
            Ok(v)
        } else {
            Err(anyhow::anyhow!("get_state_value error"))
        }
    }
     */

    pub async fn get_target_addr(&self, id: u64) -> anyhow::Result<Arc<String>> {
        if let RaftIndexResponse::TargetAddr(Some(addr)) = self
            .index_manager
            .send(RaftIndexRequest::GetTargetAddr(id))
            .await??
        {
            Ok(addr)
        } else {
            Err(anyhow::anyhow!("get_target_addr error"))
        }
    }
}

#[async_trait]
impl RaftStorage<ClientRequest, ClientResponse> for FileStore {
    type Snapshot = tokio::fs::File;
    type ShutdownError = ShutdownError;

    async fn get_membership_config(&self) -> anyhow::Result<MembershipConfig> {
        match self
            .index_manager
            .send(RaftIndexRequest::LoadMember)
            .await??
        {
            RaftIndexResponse::MemberShip {
                member,
                member_after_consensus,
                node_addrs: _node_addrs,
            } => {

                let membership = MembershipConfig {
                    members: vec_to_set(&member),
                    members_after_consensus: if member_after_consensus.is_empty() {
                        None
                    }
                    else {
                        Some(vec_to_set(&member_after_consensus))
                    },
                };
                Ok(membership)
            }
            _ => {
                log::warn!("get_membership_config init");
                Ok(MembershipConfig::new_initial(self.node_id))
            },
        }
    }

    async fn get_initial_state(&self) -> anyhow::Result<InitialState> {
        let last_log_index = self.get_last_log_index().await.unwrap_or_default();
        match self
            .index_manager
            .send(RaftIndexRequest::LoadIndexInfo)
            .await??
        {
            RaftIndexResponse::RaftIndexInfo {
                raft_index,
                last_applied_log,
            } => {
                let voted_for = if raft_index.voted_for > 0 {
                    Some(raft_index.voted_for)
                } else {
                    None
                };
                let membership = MembershipConfig {
                    members: vec_to_set(&raft_index.member),
                    members_after_consensus: if raft_index.member_after_consensus.is_empty() {
                        None
                    }
                    else {
                        Some(vec_to_set(&raft_index.member_after_consensus))
                    },
                };
                log::info!("get_initial_state from index_manager,{:?},{:?}",&last_log_index,&membership);
                Ok(InitialState {
                    last_log_index: last_log_index.index,
                    last_log_term: last_log_index.term,
                    last_applied_log,
                    hard_state: HardState {
                        current_term: raft_index.current_term,
                        voted_for,
                    },
                    membership,
                })
            }
            _ => Ok(InitialState::new_initial(self.node_id)),
        }
    }

    async fn save_hard_state(&self, hs: &HardState) -> anyhow::Result<()> {
        let req = RaftIndexRequest::SaveHardState {
            current_term: hs.current_term,
            voted_for: hs.voted_for.clone().unwrap_or_default(),
        };
        self.index_manager.send(req).await??;
        Ok(())
    }

    async fn get_log_entries(
        &self,
        start: u64,
        stop: u64,
    ) -> anyhow::Result<Vec<Entry<ClientRequest>>> {
        match self
            .log_manager
            .send(RaftLogManagerAsyncRequest::Query{ start, end: stop })
            .await??
        {
            //RaftLogResponse::QueryEntryResult(records) => Ok(records),
            RaftLogResponse::QueryResult(records) => {
                let mut entrys = Vec::with_capacity(records.len());
                for item in records.into_iter() {
                    if let Ok(entry) = StoreUtils::log_record_to_entry(item) {
                        entrys.push(entry);
                    }
                }
                Ok(entrys)
            }
            _ => Err(anyhow::anyhow!("RaftLogResponse result is error")),
        }
    }

    async fn delete_logs_from(&self, start: u64, _stop: Option<u64>) -> anyhow::Result<()> {
        self.log_manager
            .send(RaftLogManagerRequest::StripLogToIndex(start))
            .await??;
        Ok(())
    }

    async fn append_entry_to_log(&self, entry: &Entry<ClientRequest>) -> anyhow::Result<()> {
        let record = StoreUtils::entry_to_record(entry)?;
        self.log_manager
            .send(RaftLogManagerRequest::Write(record))
            .await??;
        Ok(())
    }

    async fn replicate_to_log(&self, entries: &[Entry<ClientRequest>]) -> anyhow::Result<()> {
        let mut records = Vec::with_capacity(entries.len());
        for item in entries {
            let record = StoreUtils::entry_to_record(item)?;
            records.push(record);
        }
        self.log_manager
            .send(RaftLogManagerRequest::WriteBatch(records))
            .await??;
        Ok(())
    }

    async fn apply_entry_to_state_machine(
        &self,
        index: &u64,
        data: &ClientRequest,
    ) -> anyhow::Result<ClientResponse> {
        match self.apply_manager
            .send(StateApplyAsyncRequest::ApplyRequest(ApplyRequestDto::new(*index,data.clone())))
            .await?? {
            StateApplyResponse::RaftResponse(resp) => {
                Ok(resp)
            }
            _ => {
                Err(anyhow::anyhow!("apply_entry_to_state_machine response is error"))
            }
        }
    }

    async fn replicate_to_state_machine(
        &self,
        entries: &[(&u64, &ClientRequest)],
    ) -> anyhow::Result<()> {
        let mut list = Vec::with_capacity(entries.len());
        for (index,data) in entries {
            list.push(ApplyRequestDto::new((*index).to_owned(),(*data).clone()))
        }
        self.apply_manager
            .send(StateApplyRequest::ApplyBatchRequest(list))
            .await??;
        Ok(())
    }

    async fn do_log_compaction(&self) -> anyhow::Result<CurrentSnapshotData<Self::Snapshot>> {
        match self
            .apply_manager
            .send(StateApplyAsyncRequest::BuildSnapshot)
            .await??
        {
            StateApplyResponse::Snapshot(header, path,snapshot_id) => {
                let membership_config = MembershipConfig {
                    members: vec_to_set(&header.member),
                    members_after_consensus: if header.member_after_consensus.is_empty() {
                        None
                    }
                    else {
                        Some(vec_to_set(&header.member_after_consensus))
                    },
                };
                let file = tokio::fs::OpenOptions::new()
                    .read(true)
                    .open(path.as_str())
                    .await?;
                let snapshot = CurrentSnapshotData {
                    term: header.last_term,
                    index: header.last_index,
                    membership: membership_config.clone(),
                    snapshot: Box::new(file),
                };
                let entry = Entry::new_snapshot_pointer(header.last_index, header.last_term, snapshot_id.to_string(), membership_config);
                let record = StoreUtils::entry_to_record(&entry)?;
                self.log_manager.send(RaftLogManagerRequest::BuildSnapshotPointerLog(record)).await??;
                Ok(snapshot)
            }
            _ => Err(anyhow::anyhow!("StateApplyResponse result is error")),
        }
    }

    async fn create_snapshot(&self) -> anyhow::Result<(String, Box<Self::Snapshot>)> {
        log::info!("filestore create_snapshot");
        match self
            .snapshot_manager
            .send(RaftSnapshotRequest::NewSnapshotForLoad)
            .await??
        {
            RaftSnapshotResponse::NewSnapshotForLoad(path, snapshot_id) => {
                let file = tokio::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(path.as_str())
                    .await?;
                Ok((snapshot_id.to_string(), Box::new(file)))
            }
            _ => todo!(),
        }
    }

    async fn finalize_snapshot_installation(
        &self,
        index: u64,
        term: u64,
        delete_through: Option<u64>,
        id: String,
        snapshot: Box<Self::Snapshot>,
    ) -> anyhow::Result<()> {
        let snapshot_id: u64 = id.parse()?;
        self.snapshot_manager
            .send(RaftSnapshotRequest::InstallSnapshot {
                end_index: index,
                snapshot_id,
            })
            .await??;
        self.apply_manager
            .send(StateApplyRequest::ApplySnapshot { snapshot })
            .await??;
        //清除废弃日志
        let split_off_index = if let Some(v) = delete_through {
            v + 1
        } else {
            0
        };
        self.log_manager
            .send(RaftLogManagerRequest::SplitOff(split_off_index))
            .await??;
        //add new_snapshot_pointer
        let membership_config = self.get_membership_config().await?;
        let entry = Entry::new_snapshot_pointer(index, term, id, membership_config);
        let record = StoreUtils::entry_to_record(&entry)?;
        self.log_manager.send(RaftLogManagerRequest::InstallSnapshotPointerLog(record)).await??;
        Ok(())
    }

    async fn get_current_snapshot(
        &self,
    ) -> anyhow::Result<Option<CurrentSnapshotData<Self::Snapshot>>> {
        if let RaftSnapshotResponse::LastSnapshot(Some(path), Some(header)) = self
            .snapshot_manager
            .send(RaftSnapshotRequest::GetLastSnapshot)
            .await??
        {
            let membership_config = MembershipConfig {
                members: vec_to_set(&header.member),
                members_after_consensus: if header.member_after_consensus.is_empty() {
                    None
                }
                else {
                    Some(vec_to_set(&header.member_after_consensus))
                },
            };
            let file = tokio::fs::OpenOptions::new()
                .read(true)
                .open(path.as_str())
                .await?;
            let snapshot = CurrentSnapshotData {
                term: header.last_term,
                index: header.last_index,
                membership: membership_config,
                snapshot: Box::new(file),
            };
            Ok(Some(snapshot))
        } else {
            Ok(None)
        }
    }
}