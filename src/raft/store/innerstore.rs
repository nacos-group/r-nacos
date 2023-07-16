use openraft::{LogId, Entry, Vote, StorageError, Snapshot as RSnapshot, LogState,  StoredMembership, SnapshotMeta, EntryPayload, BasicNode, StorageIOError, ErrorSubject, ErrorVerb, AnyError};
use std::collections::{Bound};
use std::sync::Arc;
use serde::Deserialize;
use serde::Serialize;

use actix::prelude::*;
use crate::common::byte_utils::{id_to_bin,bin_to_id};
use crate::common::sled_utils::TABLE_SEQUENCE_TREE_NAME;
use crate::config::core::ConfigActor;

use super::NodeId;
use super::TypeConfig;
use super::*;
use super::model::{SnapshotDataInfo, SnapshotItem};
use super::{Request,Response};
use std::ops::RangeBounds;
use std::io::Cursor;
use crate::config::model::{ConfigRaftCmd};

type SnapshotData = Cursor<Vec<u8>>;

type Snapshot = RSnapshot<NodeId,BasicNode,SnapshotData>;

type StorageResult<T> = Result<T, StorageError<NodeId>>;


#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StateMachine {
    pub last_applied_log: Option<LogId<NodeId>>,
    pub last_membership: StoredMembership<NodeId, BasicNode>,
    pub config_data: Vec<(String,Arc<String>)>,
    pub config_history_table_id: u64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StoredSnapshot {
    pub meta: SnapshotMeta<NodeId, BasicNode>,

    /// The data of the state machine at the time of this snapshot.
    pub data: Vec<u8>,
}


#[derive(Debug)]
pub struct InnerStore {
    db : Arc<sled::Db>,
    config_addr: Addr<ConfigActor>,
    last_purged_log_id: Option<LogId<NodeId>>,

    /// The Raft state machine.
    //pub state_machine: StateMachine,
    last_applied_log: Option<LogId<NodeId>>,

    last_membership: StoredMembership<NodeId, BasicNode>,

    /// The current granted vote.
    vote: Option<Vote<NodeId>>,

    snapshot_idx: u64,
}

impl InnerStore {

    pub fn new(db:Arc<sled::Db>,config_addr: Addr<ConfigActor>) -> Self {
        Self {
            db,
            config_addr,
            last_purged_log_id:Default::default(),
            last_applied_log: Default::default(),
            last_membership: Default::default(),
            vote:Default::default(),
            snapshot_idx:Default::default(),
        }
    }

    fn save_last_applied_log(&self,log_id: &LogId<NodeId>) -> StorageResult<()> {
        let state_machine = store(&self.db);
        let value = serde_json::to_vec(log_id).map_err(sm_w_err)?;
        state_machine.insert(b"last_applied_log", value).map_err(l_r_err)?;
        state_machine.flush().map_err(sm_w_err).map(|_| ())
    }

    fn save_last_membership(&self,membership: &StoredMembership<NodeId, BasicNode>) -> StorageResult<()> {
        let state_machine = store(&self.db);
        let value = serde_json::to_vec(membership).map_err(sm_w_err)?;
        state_machine.insert(b"last_membership", value).map_err(l_r_err)?;
        state_machine.flush().map_err(sm_w_err).map(|_| ())
    }

    fn save_snapshot_data(&self,items:Vec<SnapshotItem>) -> anyhow::Result<()> {
        let config_tree = self.db.open_tree("config")?;
        let table_seq_tree = self.db.open_tree(TABLE_SEQUENCE_TREE_NAME)?;
        let mut last_history_tree = None;
        for item in items {
            match item.r#type {
                1 => { //config
                    let mut key = item.key.clone();
                    config_tree.insert(item.key, item.value)?;
                    let history_name = crate::config::config_sled::Config::build_config_history_tree_name(&mut key);
                    last_history_tree = Some(self.db.open_tree(history_name)?);
                },
                2 => { //config history
                    if let Some(last_history_tree) = &last_history_tree {
                        last_history_tree.insert(item.key, item.value)?;
                    }
                },
                3 => { //table seq
                    table_seq_tree.insert(item.key, item.value)?;
                },
                _ => {} //ignore
            }
        }
        Ok(())
    }

    fn init(&mut self) -> anyhow::Result<()> {
        self.last_purged_log_id = self.get_last_purged_()?;
        self.snapshot_idx = self.get_snapshot_index_()?;
        self.vote = self.get_vote_()?;

        let state_machine = store(&self.db);
        self.last_membership= state_machine.get(b"last_membership").map_err(m_r_err).and_then(|value| {
            value
                .map(|v| serde_json::from_slice::<StoredMembership<NodeId, BasicNode>>(&v).map_err(sm_r_err))
                .unwrap_or_else(|| Ok(StoredMembership::default()))
        })?;
        self.last_applied_log = state_machine
            .get(b"last_applied_log")
            .map_err(l_r_err)
            .and_then(|value| value.map(|v| serde_json::from_slice(&v).map_err(sm_r_err)).transpose())
            ?;
        Ok(())
    }

    fn get_last_purged_(&self) -> StorageResult<Option<LogId<u64>>> {
        let store_tree = store(&self.db);
        let val = store_tree
            .get(b"last_purged_log_id")
            .map_err(|e| StorageIOError::new(ErrorSubject::Store, ErrorVerb::Read, AnyError::new(&e)))?
            .and_then(|v| serde_json::from_slice(&v).ok());

        Ok(val)
    }

    fn set_last_purged_(&mut self, log_id: LogId<u64>) -> StorageResult<()> {
        let store_tree = store(&self.db);
        let val = serde_json::to_vec(&log_id).unwrap();
        self.last_purged_log_id = Some(log_id);
        store_tree.insert(b"last_purged_log_id", val.as_slice()).map_err(s_w_err)?;
        store_tree.flush().map_err(l_w_err).map(|_| ())
    }

    fn get_snapshot_index_(&self) -> StorageResult<u64> {
        let store_tree = store(&self.db);
        let val = store_tree
            .get(b"snapshot_index")
            .map_err(s_r_err)?
            .and_then(|v| serde_json::from_slice(&v).ok())
            .unwrap_or(0);

        Ok(val)
    }

    fn set_snapshot_index_(&self, snapshot_index: u64) -> StorageResult<()> {
        let store_tree = store(&self.db);
        let val = serde_json::to_vec(&snapshot_index).unwrap();
        store_tree.insert(b"snapshot_index", val.as_slice()).map_err(s_w_err)?;
        store_tree.flush().map_err(s_w_err).map(|_| ())
    }

    fn set_vote_(&self, vote: &Vote<NodeId>) -> StorageResult<()> {
        let store_tree = store(&self.db);
        let val = serde_json::to_vec(vote).unwrap();
        store_tree.insert(b"vote", val).map_err(v_w_err).map(|_| ())?;

        store_tree.flush().map_err(v_w_err).map(|_| ())
    }

    fn get_vote_(&self) -> StorageResult<Option<Vote<NodeId>>> {
        let store_tree = store(&self.db);
        let val = store_tree.get(b"vote").map_err(v_r_err)?.and_then(|v| serde_json::from_slice(&v).ok());

        Ok(val)
    }

    fn get_current_snapshot_(&self) -> StorageResult<Option<StoredSnapshot>> {
        let store_tree = store(&self.db);
        let val = store_tree.get(b"snapshot").map_err(s_r_err)?.and_then(|v| serde_json::from_slice(&v).ok());

        Ok(val)
    }

    fn set_current_snapshot_(&self, snap: StoredSnapshot) -> StorageResult<()> {
        let store_tree = store(&self.db);
        let val = serde_json::to_vec(&snap).unwrap();
        let meta = snap.meta.clone();
        store_tree.insert(b"snapshot", val.as_slice()).map_err(|e| StorageError::IO {
            source: StorageIOError::new(
                ErrorSubject::Snapshot(snap.meta.signature()),
                ErrorVerb::Write,
                AnyError::new(&e),
            ),
        })?;

        store_tree
            .flush()
            .map_err(|e| {
                StorageIOError::new(
                    ErrorSubject::Snapshot(meta.signature()),
                    ErrorVerb::Write,
                    AnyError::new(&e),
                )
                .into()
            })
            .map(|_| ())
    }

    fn try_get_log_entries<RB: RangeBounds<u64>>(&mut self, range: RB) -> Result<Vec<Entry<TypeConfig>>, StorageError<NodeId>> {
        let start_bound = range.start_bound();
        let start = match start_bound {
            std::ops::Bound::Included(x) => id_to_bin(*x),
            std::ops::Bound::Excluded(x) => id_to_bin(*x + 1),
            std::ops::Bound::Unbounded => id_to_bin(0),
        };
        let logs_tree = logs(&self.db);
        let logs = logs_tree
            .range::<&[u8], _>(start.as_slice()..)
            .map(|el_res| {
                let el = el_res.expect("Failed read log entry");
                let id = el.0;
                let val = el.1;
                let entry: StorageResult<Entry<_>> = serde_json::from_slice(&val).map_err(|e| StorageError::IO {
                    source: StorageIOError::new(ErrorSubject::Logs, ErrorVerb::Read, AnyError::new(&e)),
                });
                let id = bin_to_id(&id);

                assert_eq!(Ok(id), entry.as_ref().map(|e| e.log_id.index));
                (id, entry)
            })
            .take_while(|(id, _)| range.contains(id))
            .map(|x| x.1)
            .collect();
        logs
        //let response = self.log.range(range).map(|(_, val)| val.clone()).collect::<Vec<_>>();
        //Ok(response)
    }

    fn build_snapshot(&mut self,ctx: &mut Context<Self>) -> Result<Snapshot, StorageError<NodeId>> {
        // Serialize the data of the state machine.
        self.snapshot_idx +=1;
        self.set_snapshot_index_(self.snapshot_idx)?;

        let snapshot_id = if let Some(last) = self.last_applied_log.as_ref() {
            format!("{}-{}-{}", last.leader_id, last.index, self.snapshot_idx)
        } else {
            format!("--{}", self.snapshot_idx)
        };

        let meta = SnapshotMeta {
            last_log_id: self.last_applied_log.to_owned(),
            last_membership: self.last_membership.to_owned(),
            snapshot_id,
        };

        let data = self.load_snapshot_data(&meta,ctx).unwrap_or_default();

        let snapshot = StoredSnapshot {
            meta: meta.clone(),
            data: data.clone(),
        };

        self.set_current_snapshot_(snapshot)?;

        Ok(Snapshot {
            meta,
            snapshot: Box::new(Cursor::new(data)),
        })
    }

    fn save_vote(&mut self, vote: Vote<NodeId>) -> Result<(), StorageError<NodeId>> {
        let r = self.set_vote_(&vote);
        self.vote = Some(vote);
        r
    }

    fn read_vote(&mut self) -> Result<Option<Vote<NodeId>>, StorageError<NodeId>> {
        Ok(self.vote.clone())
    }

    fn get_log_state(&mut self) -> Result<LogState<TypeConfig>, StorageError<NodeId>> {
        let last_purged_log_id = self.last_purged_log_id.to_owned();

        let logs_tree = logs(&self.db);
        let last_res = logs_tree.last();
        if last_res.is_err() {
            return Ok(LogState {
                last_purged_log_id,
                last_log_id: last_purged_log_id,
            });
        }

        let last = last_res
            .unwrap()
            .and_then(|(_, ent)| Some(serde_json::from_slice::<Entry<TypeConfig>>(&ent).ok()?.log_id));

        let last_log_id = match last {
            None => last_purged_log_id,
            Some(x) => Some(x),
        };
        Ok(LogState {
            last_purged_log_id,
            last_log_id,
        })
    }

    fn append_to_log(&mut self, entries: Vec<Entry<TypeConfig>>) -> Result<(), StorageError<NodeId>> {
        let logs_tree = logs(&self.db);
        let mut batch = sled::Batch::default();
        for entry in entries {
            let id = id_to_bin(entry.log_id.index);
            let value = serde_json::to_vec(&entry).map_err(l_w_err)?;
            batch.insert(id.as_slice(), value);
        }
        logs_tree.apply_batch(batch).map_err(l_w_err)?;
        logs_tree.flush().map_err(l_w_err).map(|_| ())
    }

    fn delete_conflict_logs_since(&mut self, log_id: LogId<NodeId>) -> Result<(), StorageError<NodeId>> {
        let from = id_to_bin(log_id.index);
        let to = id_to_bin(0xff_ff_ff_ff_ff_ff_ff_ff);
        let logs_tree = logs(&self.db);
        let entries = logs_tree.range::<&[u8], _>(from.as_slice()..to.as_slice());
        let mut batch_del = sled::Batch::default();
        for entry_res in entries {
            let entry = entry_res.expect("Read db entry failed");
            batch_del.remove(entry.0);
        }
        logs_tree.apply_batch(batch_del).map_err(l_w_err)?;
        logs_tree.flush().map_err(l_w_err).map(|_| ())
    }

    fn purge_logs_upto(&mut self, log_id: LogId<NodeId>) -> Result<(), StorageError<NodeId>> {
        if let Some(last_purged_log_id)  = &self.last_purged_log_id {
            if *last_purged_log_id > log_id {
                //正常不会出现，不处理
                return Ok(())
            }
        }
        self.set_last_purged_(log_id)?;
        let from = id_to_bin(0);
        let to = id_to_bin(log_id.index);
        let logs_tree = logs(&self.db);
        let entries = logs_tree.range::<&[u8], _>(from.as_slice()..=to.as_slice());
        let mut batch_del = sled::Batch::default();
        for entry_res in entries {
            let entry = entry_res.expect("Read db entry failed");
            batch_del.remove(entry.0);
        }
        logs_tree.apply_batch(batch_del).map_err(l_w_err)?;
        logs_tree.flush().map_err(l_w_err).map(|_| ())
    }

    fn last_applied_state(&mut self) -> Result<(Option<LogId<NodeId>>, StoredMembership<NodeId, BasicNode>), StorageError<NodeId>> {
        Ok((self.last_applied_log.clone(), self.last_membership.clone()))
    }

    fn apply_to_state_machine(&mut self, entries: Vec<Entry<TypeConfig>>,ctx: &mut Context<Self>) -> Result<Vec<Response>, StorageError<NodeId>>
    {
        let mut res = Vec::with_capacity(entries.len());
        for entry in entries {
            match entry.payload {
                EntryPayload::Blank => res.push(Response { value: None }),
                EntryPayload::Normal(req) => {
                    match req {
                        Request::ConfigSet { key, value, history_id, history_table_id } => {
                            let cmd = ConfigRaftCmd::ConfigAdd { key, value, history_id, history_table_id};
                            self.wait_send_config_raft_cmd(cmd,ctx).ok();
                        },
                        Request::ConfigRemove { key } => {
                            let cmd = ConfigRaftCmd::ConfigRemove { key };
                            self.wait_send_config_raft_cmd(cmd,ctx).ok();
                        },
                    };
                    res.push(Response { value: Some(true) })
                },
                EntryPayload::Membership(mem) => {
                    let membership = StoredMembership::new(Some(entry.log_id.to_owned()), mem);
                    self.save_last_membership(&membership)?;
                    self.last_membership = membership;
                    res.push(Response { value: None })
                }
            };
            self.save_last_applied_log(&entry.log_id)?;
            self.last_applied_log = Some(entry.log_id);
        }
        Ok(res)
    }

    fn wait_send_config_raft_cmd(&mut self,cmd:ConfigRaftCmd,ctx: &mut Context<Self>) -> anyhow::Result<()> {
        let config_addr = self.config_addr.clone();
        async move {
            config_addr.send(cmd).await.ok();
        }
        .into_actor(self).map(|_,_,_|{})
        .wait(ctx);
        Ok(())
    }

    fn install_snapshot(&mut self, meta: SnapshotMeta<NodeId, BasicNode>, snapshot: Box<SnapshotData>,ctx: &mut Context<Self>) -> Result<(), StorageError<NodeId>> {
        let new_snapshot = StoredSnapshot {
            meta ,
            data: snapshot.into_inner(),
        };
        let snapshot_data =SnapshotDataInfo::from_bytes(&new_snapshot.data).unwrap_or_default();
        let meta = snapshot_data.build_snapshot().unwrap();
        self.last_applied_log = meta.last_log_id;
        self.last_membership = meta.last_membership;
        self.save_snapshot_data(snapshot_data.items).ok();
        let cmd = ConfigRaftCmd::ApplySnaphot;
        self.wait_send_config_raft_cmd(cmd,ctx).ok();
        Ok(())
    }

    fn get_current_snapshot(&mut self) -> Result<Option<Snapshot>, StorageError<NodeId>> {
        match self.get_current_snapshot_()? {
            Some(snapshot) => {
                let data = snapshot.data.clone();
                Ok(Some(Snapshot {
                    meta: snapshot.meta,
                    snapshot: Box::new(Cursor::new(data)),
                }))
            }
            None => Ok(None),
        }
    }

    /// load config history data
    fn load_config_history(&self,history_name:Vec<u8>,items:&mut Vec<SnapshotItem>) -> anyhow::Result<()> {
        let config_history_tree = self.db.open_tree(history_name)?;
        let mut iter = config_history_tree.iter();
        while let Some(Ok((k, v))) = iter.next() {
            let item = SnapshotItem {
                r#type:2,
                key:k.to_vec(),
                value:v.to_vec(),
            };
            items.push(item);
        }
        Ok(())
    }

    /// load table data
    fn load_table_seq(&self,items:&mut Vec<SnapshotItem>) -> anyhow::Result<()>  {
        let tree = self.db.open_tree(TABLE_SEQUENCE_TREE_NAME)?;
        let mut iter = tree.iter();
        while let Some(Ok((k, v))) = iter.next() {
            let item = SnapshotItem {
                r#type:3,
                key:k.to_vec(),
                value:v.to_vec(),
            };
            items.push(item);
        }
        Ok(())
    }

    fn load_snapshot_data(&mut self,meta:&SnapshotMeta<NodeId,BasicNode>, ctx: &mut Context<Self>) -> anyhow::Result<Vec<u8>> {
        let snapshot_meta_json = serde_json::to_string(meta)?;
        
        // load config
        let mut items = vec![];
        let config_tree = self.db.open_tree("config").unwrap();
        let mut iter = config_tree.iter();
        while let Some(Ok((k, v))) = iter.next() {
            let mut key = k.to_vec();
            let item = SnapshotItem {
                r#type:1,
                key:key.clone(),
                value:v.to_vec(),
            };
            items.push(item);
            let history_name = crate::config::config_sled::Config::build_config_history_tree_name(&mut key);
            self.load_config_history(history_name, &mut items)?;
        }
        self.load_table_seq(&mut items)?;

        let snapshot_data = SnapshotDataInfo{
            snapshot_meta_json,
            items,
        };

        snapshot_data.to_bytes()

    }

}

impl Actor for InnerStore {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("InnerStore started");
        self.init().ok();
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
    fn handle(&mut self, msg: StoreRequest, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            StoreRequest::GetLogEntities(start,end) => {
                let v = self.try_get_log_entries((start,end))?;
                Ok(StoreResponse::Entities(v))
            }
            StoreRequest::BuildSnapshot => {
                let v = self.build_snapshot(ctx)?;
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
                let v = self.apply_to_state_machine(items,ctx)?;
                Ok(StoreResponse::ApplyResult(v))
            }
            StoreRequest::InstallSnapshot(node, snapshot) => {
                self.install_snapshot(node,snapshot,ctx)?;
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
                //let v= self.state_machine.data.get(&key).map(|v|v.to_owned());
                Ok(StoreResponse::StateValue(None))
            }
        }
    }
}


fn store(db: &sled::Db) -> sled::Tree {
    db.open_tree("raft_store").expect("store open failed")
}
fn logs(db: &sled::Db) -> sled::Tree {
    db.open_tree("raft_logs").expect("logs open failed")
}
