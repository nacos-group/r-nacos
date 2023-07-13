use openraft::{LogId, Entry, Vote, StorageError, Snapshot as RSnapshot, LogState,  StoredMembership, SnapshotMeta, EntryPayload, BasicNode, StorageIOError, ErrorSubject, ErrorVerb, AnyError};
use std::collections::{BTreeMap, Bound};

use actix::prelude::*;
use super::NodeId;
use super::TypeConfig;
use super::{StateMachine, StoredSnapshot,Request,Response};
use std::ops::RangeBounds;
use std::io::Cursor;

type SnapshotData = Cursor<Vec<u8>>;

type Snapshot = RSnapshot<NodeId,BasicNode,SnapshotData>;

#[derive(Debug, Default)]
pub struct InnerStore {
    last_purged_log_id: Option<LogId<NodeId>>,

    /// The Raft log.
    log: BTreeMap<u64, Entry<TypeConfig>>,

    /// The Raft state machine.
    pub state_machine: StateMachine,

    /// The current granted vote.
    vote: Option<Vote<NodeId>>,

    snapshot_idx: u64,

    current_snapshot: Option<StoredSnapshot>,
}

impl InnerStore {

    pub fn new() -> Self {
        Self{
            ..Default::default()
        }
    }

    fn try_get_log_entries<RB: RangeBounds<u64>>(&mut self, range: RB) -> Result<Vec<Entry<TypeConfig>>, StorageError<NodeId>> {
        let response = self.log.range(range).map(|(_, val)| val.clone()).collect::<Vec<_>>();
        Ok(response)
    }

    fn build_snapshot(&mut self) -> Result<Snapshot, StorageError<NodeId>> {
        // Serialize the data of the state machine.
        let data = serde_json::to_vec(&self.state_machine).map_err(|e| StorageIOError::new(ErrorSubject::StateMachine, ErrorVerb::Read, AnyError::new(&e)))?;

        self.snapshot_idx +=1;

        let snapshot_id = if let Some(last) = self.state_machine.last_applied_log.as_ref() {
            format!("{}-{}-{}", last.leader_id, last.index, self.snapshot_idx)
        } else {
            format!("--{}", self.snapshot_idx)
        };

        let meta = SnapshotMeta {
            last_log_id: self.state_machine.last_applied_log.to_owned(),
            last_membership: self.state_machine.last_membership.to_owned(),
            snapshot_id,
        };

        let snapshot = StoredSnapshot {
            meta: meta.clone(),
            data: data.clone(),
        };

        self.current_snapshot = Some(snapshot);

        Ok(Snapshot {
            meta,
            snapshot: Box::new(Cursor::new(data)),
        })
    }

    fn save_vote(&mut self, vote: Vote<NodeId>) -> Result<(), StorageError<NodeId>> {
        self.vote = Some(vote);
        Ok(())
    }

    fn read_vote(&mut self) -> Result<Option<Vote<NodeId>>, StorageError<NodeId>> {
        Ok(self.vote.clone())
    }

    fn get_log_state(&mut self) -> Result<LogState<TypeConfig>, StorageError<NodeId>> {
        let last = self.log.iter().next_back().map(|(_, ent)| ent.log_id);

        let last_purged = self.last_purged_log_id.clone();

        let last = match last {
            None => last_purged.clone(),
            Some(x) => Some(x),
        };

        Ok(LogState {
            last_purged_log_id: last_purged,
            last_log_id: last,
        })
    }

    fn append_to_log(&mut self, entries: Vec<Entry<TypeConfig>>) -> Result<(), StorageError<NodeId>> {
        for entry in entries {
            self.log.insert(entry.log_id.index.to_owned(), entry);
        }
        Ok(())
    }

    fn delete_conflict_logs_since(&mut self, log_id: LogId<NodeId>) -> Result<(), StorageError<NodeId>> {
        //TODO 删除日志需要恢复machine历史记录
        let keys = self.log.range(log_id.index..).map(|(k, _v)| *k).collect::<Vec<_>>();
        for key in keys {
            self.log.remove(&key);
        }
        Ok(())
    }

    fn purge_logs_upto(&mut self, log_id: LogId<NodeId>) -> Result<(), StorageError<NodeId>> {
        //assert!(self.last_purged_log_id <= Some(log_id));
        if let Some(last_purged_log_id)  = &self.last_purged_log_id {
            if *last_purged_log_id > log_id {
                //正常不会出现，不处理
                return Ok(())
            }
        }
        self.last_purged_log_id = Some(log_id);
        let keys = self.log.range(..=log_id.index).map(|(k, _v)| *k).collect::<Vec<_>>();
        for key in keys {
            self.log.remove(&key);
        }
        Ok(())
    }

    fn last_applied_state(&mut self) -> Result<(Option<LogId<NodeId>>, StoredMembership<NodeId, BasicNode>), StorageError<NodeId>> {
        Ok((self.state_machine.last_applied_log.clone(), self.state_machine.last_membership.clone()))
    }

    fn apply_to_state_machine(&mut self, entries: Vec<Entry<TypeConfig>>) -> Result<Vec<Response>, StorageError<NodeId>>
    {
        let mut res = Vec::with_capacity(entries.len());

        for entry in entries {
            //tracing::debug!(%entry.log_id, "replicate to sm");

            self.state_machine.last_applied_log = Some(entry.log_id.to_owned());

            match entry.payload {
                EntryPayload::Blank => res.push(Response { value: None }),
                EntryPayload::Normal(req) => match req {
                    Request::Set { key, value } => {
                        self.state_machine.data.insert(key, value.clone());
                        res.push(Response {
                            //value: Some(value)
                            value: None,
                        })
                    }
                },
                EntryPayload::Membership(mem) => {
                    self.state_machine.last_membership = StoredMembership::new(Some(entry.log_id.to_owned()), mem);
                    res.push(Response { value: None })
                }
            };
        }
        Ok(res)
    }

    fn install_snapshot(&mut self, meta: SnapshotMeta<NodeId, BasicNode>, snapshot: Box<SnapshotData>) -> Result<(), StorageError<NodeId>> {
        let new_snapshot = StoredSnapshot {
            meta ,
            data: snapshot.into_inner(),
        };
        let updated_state_machine: StateMachine = serde_json::from_slice(&new_snapshot.data)
            .map_err(|e| 
                StorageIOError::new(
                    ErrorSubject::Snapshot(new_snapshot.meta.signature()),
                    ErrorVerb::Read,
                    AnyError::new(&e),
                )
            )?;
        self.state_machine = updated_state_machine;
        Ok(())
    }

    fn get_current_snapshot(&mut self) -> Result<Option<Snapshot>, StorageError<NodeId>> {
        match self.current_snapshot.as_ref() {
            None => {Ok(None)}
            Some(snapshot) => {
                let data = snapshot.data.clone();
                Ok(Some(Snapshot {
                    meta: snapshot.meta.clone(),
                    snapshot: Box::new(Cursor::new(data)),
                }))
            }
        }
    }
}

impl Actor for InnerStore {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("InnerStore started")
    }
}

#[derive(Message, Debug)]
#[rtype(result = "Result<StoreResponse,StorageError<NodeId>>")]
pub enum StoreRequest {
    GetLogEntities(Bound<u64>,Bound<u64>),
    BuildSnapshot,
    SaveVote(Vote<NodeId>),
    GetLogState,
    AppendToLog(Vec<Entry<TypeConfig>>),
    DeleteConflictLogs(LogId<NodeId>),
    PurgeLogs(LogId<NodeId>),
    LastAppliedState,
    ApplyToStateMachine(Vec<Entry<TypeConfig>>),
    InstallSnapshot(SnapshotMeta<NodeId,BasicNode>,Box<SnapshotData>),
    GetCurrentSnapshot,
    ReadVote,
    ReadStateValue(String),
}

pub enum StoreResponse {
    Entities(Vec<Entry<TypeConfig>>),
    Snapshot(Snapshot),
    Vote(Option<Vote<NodeId>>),
    LogState(LogState<TypeConfig>),
    LastAppliedState(Option<LogId<NodeId>>, StoredMembership<NodeId, BasicNode>),
    ApplyResult(Vec<Response>),
    StateValue(Option<String>),
    None,
}


impl Handler<StoreRequest> for InnerStore {
    type Result = Result<StoreResponse,StorageError<NodeId>>;
    fn handle(&mut self, msg: StoreRequest, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            StoreRequest::GetLogEntities(start,end) => {
                let v = self.try_get_log_entries((start,end))?;
                Ok(StoreResponse::Entities(v))
            }
            StoreRequest::BuildSnapshot => {
                let v = self.build_snapshot()?;
                Ok(StoreResponse::Snapshot(v))
            }
            StoreRequest::SaveVote(v) => {
                self.save_vote(v)?;
                Ok(StoreResponse::None)
            }
            StoreRequest::GetLogState => {
                let v = self.get_log_state()?;
                Ok(StoreResponse::LogState(v))
            }
            StoreRequest::AppendToLog(items) => {
                self.append_to_log(items)?;
                Ok(StoreResponse::None)
            }
            StoreRequest::DeleteConflictLogs(idx) => {
                self.delete_conflict_logs_since(idx)?;
                Ok(StoreResponse::None)
            }
            StoreRequest::PurgeLogs(idx) => {
                self.purge_logs_upto(idx)?;
                Ok(StoreResponse::None)
            }
            StoreRequest::LastAppliedState => {
                let (idx,mem)= self.last_applied_state()?;
                Ok(StoreResponse::LastAppliedState(idx,mem))
            }
            StoreRequest::ApplyToStateMachine(items) => {
                let v = self.apply_to_state_machine(items)?;
                Ok(StoreResponse::ApplyResult(v))
            }
            StoreRequest::InstallSnapshot(node, snapshot) => {
                self.install_snapshot(node,snapshot)?;
                Ok(StoreResponse::None)
            }
            StoreRequest::GetCurrentSnapshot => {
                match self.get_current_snapshot()? {
                    None => {
                        Ok(StoreResponse::None)
                    }
                    Some(v) => {
                        Ok(StoreResponse::Snapshot(v))
                    }
                }
            }
            StoreRequest::ReadVote => {
                let v = self.read_vote()?;
                Ok(StoreResponse::Vote(v))
            }
            StoreRequest::ReadStateValue(key) => {
                let v= self.state_machine.data.get(&key).map(|v|v.to_owned());
                Ok(StoreResponse::StateValue(v))
            }
        }
    }
}