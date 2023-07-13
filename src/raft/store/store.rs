use openraft::{AnyError, StorageIOError, ErrorSubject};
use openraft::{LogId, Entry, Vote, RaftLogReader, StorageError, RaftSnapshotBuilder, Snapshot as RSnapshot, RaftStorage, LogState, StoredMembership, SnapshotMeta, BasicNode};
 
use std::io::Cursor;

use actix::prelude::*;


use super::NodeId;
use super::TypeConfig;
use super::{Response};
use std::ops::{RangeBounds, Bound};
use std::fmt::{self, Debug};
use super::innerstore::{InnerStore, StoreRequest, StoreResponse};
use std::sync::Arc;
use openraft::async_trait::async_trait;


type SnapshotData = Cursor<Vec<u8>>;
type Snapshot = RSnapshot<NodeId,BasicNode,SnapshotData>;

fn init_inner_store() -> Addr<InnerStore> {
    let (tx,rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let rt = System::new();
        let addrs = rt.block_on(async {
            InnerStore::new().start()
        });
        tx.send(addrs).unwrap();
        rt.run().unwrap();
    });
    let addrs = rx.recv().unwrap();
    addrs
}

#[derive(Clone)]
pub struct Store{
    inner_addr:Addr<InnerStore>,
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

impl Store {
    pub fn new() -> Self {
        let inner_addr = init_inner_store();
        Self {
            inner_addr
        }
    }
    fn reponse_error(&self) -> StorageError<NodeId> {
        let io_err = StorageIOError::new(
            ErrorSubject::None,
            openraft::ErrorVerb::Read,
            AnyError::new(&fmt::Error{}).add_context(|| "store response error")
        );
        StorageError::IO{source:io_err}
    }

    pub async fn send_store_msg(&self,msg:StoreRequest) -> anyhow::Result<StoreResponse> {
        if let Ok(res ) = self.inner_addr.send(msg).await {
            return Ok((res as Result<StoreResponse,StorageError<NodeId>>)?)
        };
        Err(self.reponse_error())?
    }

    pub async fn get_state_value(&self,key:String) -> anyhow::Result<Option<String>> {
        match self.send_store_msg(StoreRequest::ReadStateValue(key)).await? {
            StoreResponse::StateValue(v) => {
                Ok(v)
            },
            _ => {
                Err(self.reponse_error())?
            },
        }
    }
}

fn to_owned_bound<T:ToOwned<Owned=T>>(b:Bound<&T>) -> Bound<T> {
    match b {
        Bound::Included(v) => {
            Bound::Included(v.to_owned())
        }
        Bound::Excluded(v) => {
            Bound::Excluded(v.to_owned())
        }
        Bound::Unbounded => {
            Bound::Unbounded
        }
    }
}

#[async_trait]
impl RaftLogReader<TypeConfig> for Arc<Store> {
    async fn try_get_log_entries<RB: RangeBounds<u64> + Clone + Debug + Send + Sync>(
        &mut self,
        range: RB,
    ) -> Result<Vec<Entry<TypeConfig>>, StorageError<NodeId>> {
        let msg = StoreRequest::GetLogEntities(to_owned_bound(range.start_bound()),to_owned_bound(range.end_bound()));
        if let Ok(res ) = self.inner_addr.send(msg).await {
            match (res as Result<StoreResponse,StorageError<NodeId>>)? {
                StoreResponse::Entities(v) => {
                    return Ok(v)
                },
                _ => {}
            };
        };
        Err(self.reponse_error())
    }

    async fn get_log_state(&mut self) -> Result<LogState<TypeConfig>, StorageError<NodeId>> {
        let msg = StoreRequest::GetLogState;
        if let Ok(res ) = self.inner_addr.send(msg).await {
            match (res as Result<StoreResponse,StorageError<NodeId>>)? {
                StoreResponse::LogState(v)=> {
                    return Ok(v)
                },
                _ => {}
            };
        };
        Err(self.reponse_error())
    }
}

#[async_trait]
impl RaftSnapshotBuilder<TypeConfig,Cursor<Vec<u8>>> for Arc<Store> {
    async fn build_snapshot(&mut self) -> Result<Snapshot, StorageError<NodeId>> {
        let msg = StoreRequest::BuildSnapshot;
        if let Ok(res ) = self.inner_addr.send(msg).await {
            match (res as Result<StoreResponse,StorageError<NodeId>>)? {
                StoreResponse::Snapshot(v) => {
                    return Ok(v)
                },
                _ => {}
            };
        };
        Err(self.reponse_error())
    }
}

#[async_trait]
impl RaftStorage<TypeConfig> for Arc<Store> {
    type LogReader = Self;
    type SnapshotBuilder = Self;
    type SnapshotData = SnapshotData;

    async fn save_vote(&mut self, vote: &Vote<NodeId>) -> Result<(), StorageError<NodeId>> {
        let msg = StoreRequest::SaveVote(vote.to_owned());
        if let Ok(res ) = self.inner_addr.send(msg).await {
            match (res as Result<StoreResponse,StorageError<NodeId>>)? {
                StoreResponse::None => {
                    return Ok(())
                },
                _ => {}
            };
        };
        Err(self.reponse_error())
    }

    async fn read_vote(&mut self) -> Result<Option<Vote<NodeId>>, StorageError<NodeId>> {
        let msg = StoreRequest::ReadVote;
        if let Ok(res ) = self.inner_addr.send(msg).await {
            match (res as Result<StoreResponse,StorageError<NodeId>>)? {
                StoreResponse::Vote(v)=> {
                    return Ok(v)
                },
                _ => {}
            };
        };
        Err(self.reponse_error())
    }

/*
    
 */
    async fn get_log_reader(&mut self) -> Self::LogReader {
        self.clone()
    }

    async fn append_to_log(&mut self, entries: &[&Entry<TypeConfig>]) -> Result<(), StorageError<NodeId>> {
        let msg = StoreRequest::AppendToLog(entries.into_iter().map(|e|(*e).to_owned()).collect());
        if let Ok(res ) = self.inner_addr.send(msg).await {
            match (res as Result<StoreResponse,StorageError<NodeId>>)? {
                StoreResponse::None => {
                    return Ok(())
                },
                _ => {}
            };
        };
        Err(self.reponse_error())
    }

    async fn delete_conflict_logs_since(&mut self, log_id: LogId<NodeId>) -> Result<(), StorageError<NodeId>> {
        let msg = StoreRequest::DeleteConflictLogs(log_id);
        if let Ok(res ) = self.inner_addr.send(msg).await {
            match (res as Result<StoreResponse,StorageError<NodeId>>)? {
                StoreResponse::None => {
                    return Ok(())
                },
                _ => {}
            };
        };
        Err(self.reponse_error())
    }

    async fn purge_logs_upto(&mut self, log_id: LogId<NodeId>) -> Result<(), StorageError<NodeId>> {
        let msg = StoreRequest::PurgeLogs(log_id);
        if let Ok(res ) = self.inner_addr.send(msg).await {
            match (res as Result<StoreResponse,StorageError<NodeId>>)? {
                StoreResponse::None => {
                    return Ok(())
                },
                _ => {}
            };
        };
        Err(self.reponse_error())
    }

    async fn last_applied_state(&mut self) -> Result<(Option<LogId<NodeId>>, StoredMembership<NodeId, BasicNode>), StorageError<NodeId>> {
        let msg = StoreRequest::LastAppliedState;
        if let Ok(res ) = self.inner_addr.send(msg).await {
            match (res as Result<StoreResponse,StorageError<NodeId>>)? {
                StoreResponse::LastAppliedState(id,mem) => {
                    return Ok((id,mem))
                },
                _ => {}
            };
        };
        Err(self.reponse_error())
    }

    async fn apply_to_state_machine(&mut self, entries: &[&Entry<TypeConfig>]) -> Result<Vec<Response>, StorageError<NodeId>> {
        let msg = StoreRequest::ApplyToStateMachine(entries.into_iter().map(|e|(*e).to_owned()).collect());
        //let msg = StoreRequest::ApplyToStateMachine(entries.to_owned());
        if let Ok(res ) = self.inner_addr.send(msg).await {
            match (res as Result<StoreResponse,StorageError<NodeId>>)? {
                StoreResponse::ApplyResult(v) => {
                    return Ok(v)
                },
                _ => {}
            };
        };
        Err(self.reponse_error())
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        self.clone()
    }

    async fn begin_receiving_snapshot(&mut self) -> Result< Box<SnapshotData>, StorageError<NodeId>> {
        Ok(Box::new(Cursor::new(Vec::new())))
    }

    async fn install_snapshot(&mut self, meta: &SnapshotMeta<NodeId, BasicNode>, snapshot:  Box<SnapshotData>) -> Result<(), StorageError<NodeId>> {
        let msg = StoreRequest::InstallSnapshot(meta.to_owned(),snapshot);
        if let Ok(res ) = self.inner_addr.send(msg).await {
            match (res as Result<StoreResponse,StorageError<NodeId>>)? {
                StoreResponse::None => {
                    return Ok(())
                },
                _ => {}
            };
        };
        Err(self.reponse_error())
    }

    async fn get_current_snapshot(&mut self) -> Result<Option<Snapshot>, StorageError<NodeId>> {
        let msg = StoreRequest::GetCurrentSnapshot;
        if let Ok(res ) = self.inner_addr.send(msg).await {
            match (res as Result<StoreResponse,StorageError<NodeId>>)? {
                StoreResponse::Snapshot(v)=> {
                    return Ok(Some(v))
                },
                StoreResponse::None=> {
                    return Ok(None)
                },
                _ => {}
            };
        };
        Err(self.reponse_error())
    }
}