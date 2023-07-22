use std::collections::{BTreeMap, HashMap};

use crate::common::byte_utils::{id_to_bin, bin_to_id};
use crate::config::core::ConfigActor;

use super::{ClientRequest, ClientResponse, StateMachine, StoredSnapshot, RaftLog, Membership};
use actix::prelude::*;
use async_raft::raft::{Entry, EntryPayload, MembershipConfig};
use async_raft::storage::{CurrentSnapshotData, HardState, InitialState};
use std::io::Cursor;
use std::sync::Arc;

use super::NodeId;

fn store(db: &sled::Db) -> sled::Tree {
    db.open_tree("raft_store").expect("store open failed")
}
fn logs(db: &sled::Db) -> sled::Tree {
    db.open_tree("raft_logs").expect("logs open failed")
}

#[derive(Debug)]
pub struct InnerStore {

    db : Arc<sled::Db>,
    config_addr: Addr<ConfigActor>,
    id: NodeId,
    /// The Raft log.
    log: BTreeMap<u64, Entry<ClientRequest>>,

    /// The Raft state machine.
    pub state_machine: StateMachine,

    pub hs: Option<HardState>,
    /// The current granted vote.
    //vote: Option<HardState>,
    current_snapshot: Option<StoredSnapshot>,

    membership: Membership,
}

impl InnerStore {
    pub fn new(id: u64,db:Arc<sled::Db>,config_addr: Addr<ConfigActor>) -> Self {
        Self {
            id,
            config_addr,
            db,
            log: Default::default(),
            state_machine: Default::default(),
            hs: None,
            membership: Default::default(),
            current_snapshot: Default::default(),
        }
    }

    fn init(&mut self) -> anyhow::Result<()> {
        //load last log
        let logs_tree = logs(&self.db);
        if let Ok(Some((key,val))) = logs_tree.last() {
            let entry:async_raft::raft::Entry<ClientRequest> = serde_json::from_slice(&val)?;
            let id = bin_to_id(&key);
            self.log.insert(id, entry);
        }

        self.hs = self.get_hard_state_()?;
        if let Some(m) = self.get_membership_()? {
            self.membership = m;
        }
        self.state_machine.last_applied_log = self.get_last_applied_log_()?;

        Ok(())
    }

    fn set_hard_state_(&self, hs: &HardState ) -> anyhow::Result<()> {
        let store_tree = store(&self.db);
        let val = serde_json::to_vec(hs)?;
        store_tree.insert(b"hs", val)?;
        store_tree.flush()?;
        Ok(())
    }

    fn get_hard_state_(&self) -> anyhow::Result<Option<HardState>> {
        let store_tree = store(&self.db);
        let val = match store_tree.get(b"hs")? {
            Some(v) => Some(serde_json::from_slice(&v)?),
            None => None,
        };
        Ok(val)
    }

    fn set_membership_(&self, info: &Membership ) -> anyhow::Result<()> {
        let store_tree = store(&self.db);
        let val = serde_json::to_vec(info)?;
        store_tree.insert(b"membership", val)?;
        store_tree.flush()?;
        Ok(())
    }

    fn get_membership_(&self) -> anyhow::Result<Option<Membership>> {
        let store_tree = store(&self.db);
        let val = match store_tree.get(b"membership")? {
            Some(v) => Some(serde_json::from_slice(&v)?),
            None => None,
        };
        Ok(val)
    }

    fn set_last_applied_log_(&self,log_id: u64) -> anyhow::Result<()> {
        let state_machine = store(&self.db);
        let value = id_to_bin(log_id);
        state_machine.insert(b"last_applied_log", value);
        state_machine.flush();
        Ok(())
    }

    fn get_last_applied_log_(&self) -> anyhow::Result<u64> {
        let store_tree = store(&self.db);
        let val = match store_tree.get(b"last_applied_log")? {
            Some(v) => bin_to_id(&v),
            None => 0,
        };
        Ok(val)
    }

    fn get_membership_config(&mut self) -> anyhow::Result<async_raft::raft::MembershipConfig> {
        Ok(self.membership.membership_config.to_owned())
    }

    fn get_initial_state(&mut self) -> anyhow::Result<async_raft::storage::InitialState> {
        let membership = self.get_membership_config()?;
        match &self.hs {
            Some(hs) => {
                let (last_log_index, last_log_term) = match self.log.values().rev().next() {
                    Some(log) => (log.index, log.term),
                    None => (0, 0),
                };
                let last_applied_log = self.state_machine.last_applied_log;
                Ok(InitialState {
                    last_log_index: last_log_index,
                    last_log_term: last_log_term,
                    last_applied_log,
                    hard_state: hs.clone(),
                    membership,
                })
            }
            None => {
                let new = InitialState::new_initial(self.id);
                self.hs = Some(new.hard_state.clone());
                Ok(new)
            }
        }
    }

    fn save_hard_state(&mut self, hs: async_raft::storage::HardState) -> anyhow::Result<()> {
        self.set_hard_state_(&hs)?;
        self.hs = Some(hs);
        Ok(())
    }

    fn get_log_entries(
        &mut self,
        start_index: u64,
        end_index: u64,
    ) -> anyhow::Result<Vec<async_raft::raft::Entry<ClientRequest>>> {
        if start_index >= end_index {
            tracing::error!("invalid request, start > stop");
            return Ok(vec![]);
        }

        let start = id_to_bin(start_index);
        let end = id_to_bin(end_index);
        let range = start_index..end_index;
        if self.log.contains_key(&start_index) {
            let response = self.log.range(range).map(|(_, val)| val.clone()).collect::<Vec<_>>();
            return Ok(response);
        }
        log::warn!("get_log_entries from db");
        let logs_tree = logs(&self.db);
        let logs = logs_tree
            .range::<&[u8], _>(start.as_slice()..end.as_slice())
            .map(|el_res| {
                let el = el_res.expect("Failed read log entry");
                let id = el.0;
                let val = el.1;
                let entry:async_raft::raft::Entry<ClientRequest> = serde_json::from_slice(&val).unwrap();
                let id = bin_to_id(&id);
                assert_eq!(id, entry.index);
                (id, entry)
            })
            .take_while(|(id, _)| range.contains(id))
            .map(|x| x.1)
            .collect();
        Ok(logs)
    }

    fn delete_logs_from(&mut self, start: u64, stop: Option<u64>) -> anyhow::Result<()> {
        if stop.as_ref().map(|stop| &start > stop).unwrap_or(false) {
            tracing::error!("invalid request, start > stop");
            return Ok(());
        }

        let to_index = if let Some(stop) = stop.as_ref() {
            for key in start..*stop {
                self.log.remove(&key);
            }
            *stop
        }
        else{
            self.log.split_off(&start);
            0xff_ff_ff_ff_ff_ff_ff_ff
        };
        // Else, just split off the remainder.

        let from = id_to_bin(start);
        let to = id_to_bin(to_index);
        let logs_tree = logs(&self.db);
        let entries = logs_tree.range::<&[u8], _>(from.as_slice()..=to.as_slice());
        let mut batch_del = sled::Batch::default();
        for entry_res in entries {
            let (key,_) = entry_res.expect("Read db entry failed");
            batch_del.remove(key);
        }
        logs_tree.apply_batch(batch_del)?;
        logs_tree.flush()?;

        // If a stop point was specified, delete from start until the given stop point.

        Ok(())
    }

    fn append_entry_to_log(
        &mut self,
        entry: async_raft::raft::Entry<ClientRequest>,
    ) -> anyhow::Result<()> {
        let logs_tree = logs(&self.db);
        let id = id_to_bin(entry.index);
        let value = serde_json::to_vec(&entry)?;
        logs_tree.insert(id, value)?;
        logs_tree.flush()?;

        self.log.insert(entry.index, entry);
        Ok(())
    }

    fn replicate_to_log(
        &mut self,
        entries: Vec<async_raft::raft::Entry<ClientRequest>>,
    ) -> anyhow::Result<()> {
        let logs_tree = logs(&self.db);
        let mut batch = sled::Batch::default();
        for entry in &entries {
            let id = id_to_bin(entry.index);
            let value = serde_json::to_vec(&entry)?;
            batch.insert(id, value);
        }
        //cache log
        for entry in entries {
            self.log.insert(entry.index, entry);
        }
        logs_tree.apply_batch(batch)?;
        logs_tree.flush()?;
        Ok(())
    }

/* 
    fn apply_entry_to_state_machine(
        &mut self,
        index: u64,
        req: ClientRequest,
    ) -> anyhow::Result<ClientResponse> {
        self.state_machine.last_applied_log = index;
        let res = match req {
            ClientRequest::Set { key, value } => {
                self.state_machine.data.insert(key, value.clone());
                //ClientResponse { value: Some(value) }
                ClientResponse { value: None }
            }
            ClientRequest::NodeAddr {id,addr} => {
                self.addr_map.insert(id,addr);
                ClientResponse { value: None }
            }
        };
        Ok(res)
    }

    fn replicate_to_state_machine(
        &mut self,
        entries: Vec<(u64, ClientRequest)>,
    ) -> anyhow::Result<()> {
        for (index, req) in entries {
            self.state_machine.last_applied_log = index;
            match req {
                ClientRequest::Set { key, value } => {
                    self.state_machine.data.insert(key, value.clone());
                }
                ClientRequest::NodeAddr {id,addr} => {
                    self.addr_map.insert(id,addr);
                }
            };
        }
        Ok(())
    }
*/

    fn replicate_to_state_machine_range(
        &mut self,
        start:u64,
        end:u64
    ) -> anyhow::Result<()> {
        let entries:Vec<(u64,&Entry<ClientRequest>)> = self.log.range(start..end).map(|(idx,entry)|(*idx,entry)).collect();
        for (index, entry) in entries {
            match &entry.payload {
                EntryPayload::Normal(normal) => {
                    match &normal.data {
                        ClientRequest::Set { key, value } => {
                            self.state_machine.data.insert(key.to_owned(), value.to_owned());
                        }
                        ClientRequest::NodeAddr {id,addr} => {
                            self.membership.node_addr.insert(id.to_owned(),addr.clone());
                            self.set_membership_(&self.membership)?;
                        }
                    }
                },
                EntryPayload::ConfigChange(config_change) => {
                    self.membership.membership_config = config_change.membership.to_owned();
                    self.set_membership_(&self.membership)?;
                },
                _ => {},
            }
            self.state_machine.last_applied_log = index;
            self.set_last_applied_log_(index)?;
        }
        Ok(())
    }

    fn do_log_compaction(
        &mut self,
    ) -> anyhow::Result<async_raft::storage::CurrentSnapshotData<Cursor<Vec<u8>>>> {
        let data = serde_json::to_vec(&self.state_machine)?;
        let term = self
            .log
            .get(&self.state_machine.last_applied_log)
            .map(|entry| entry.term)
            .ok_or_else(|| anyhow::anyhow!("term is empty"))?;
        let membership_config = self.get_membership_config()?;
        let snapshot = StoredSnapshot {
            index: self.state_machine.last_applied_log,
            term,
            membership: membership_config.clone(),
            data,
        };
        let snapshot_bytes = serde_json::to_vec(&snapshot)?;
        let current_snapshot = CurrentSnapshotData {
            term,
            index: self.state_machine.last_applied_log,
            membership: membership_config,
            snapshot: Box::new(Cursor::new(snapshot_bytes)),
        };
        Ok(current_snapshot)
    }

    fn finalize_snapshot_installation(
        &mut self,
        index: u64,
        term: u64,
        delete_through: Option<u64>,
        id: String,
        snapshot: Box<Cursor<Vec<u8>>>,
    ) -> anyhow::Result<()> {
        let new_snapshot: StoredSnapshot = serde_json::from_slice(snapshot.get_ref().as_slice())?;
        // Update log.
        // Go backwards through the log to find the most recent membership config <= the `through` index.
        let membership_config = self
            .log
            .values()
            .rev()
            .skip_while(|entry| entry.index > index)
            .find_map(|entry| match &entry.payload {
                EntryPayload::ConfigChange(cfg) => Some(cfg.membership.clone()),
                _ => None,
            })
            .unwrap_or_else(|| MembershipConfig::new_initial(self.id));

        match &delete_through {
            Some(through) => {
                self.log = self.log.split_off(&(through + 1));
            }
            None => self.log.clear(),
        }
        self.log.insert(
            index,
            Entry::new_snapshot_pointer(index, term, id, membership_config),
        );

        // Update the state machine.
        self.state_machine = serde_json::from_slice(&new_snapshot.data)?;

        // Update current snapshot.
        self.current_snapshot = Some(new_snapshot);
        Ok(())
    }

    fn get_current_snapshot(
        &mut self,
    ) -> anyhow::Result<Option<async_raft::storage::CurrentSnapshotData<Cursor<Vec<u8>>>>> {
        match &self.current_snapshot{
            Some(snapshot) => {
                let reader = serde_json::to_vec(&snapshot)?;
                Ok(Some(CurrentSnapshotData {
                    index: snapshot.index,
                    term: snapshot.term,
                    membership: snapshot.membership.clone(),
                    snapshot: Box::new(Cursor::new(reader)),
                }))
            }
            None => Ok(None),
        }
    }
}

impl Actor for InnerStore {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        self.init().ok();
        log::info!("InnerStore started");
    }
}

#[derive(Message, Debug)]
#[rtype(result = "anyhow::Result<StoreResponse>")]
pub enum StoreRequest {
    GetMembershipConfig,
    GetInitialState,
    SaveHardState(HardState),
    GetLogEntries {
        start: u64,
        stop: u64,
    },
    DeleteLogsFrom {
        start: u64,
        stop: Option<u64>,
    },
    AppendEntryToLog(async_raft::raft::Entry<ClientRequest>),
    ReplicateToLog(Vec<Entry<ClientRequest>>),
    //ApplyEntryToStateMachine { index: u64, request: ClientRequest, },
    //ReplicateToStateMachine(Vec<(u64, ClientRequest)>),
    ApplyEntryToStateMachineRange{start:u64,end:u64},
    DoLogCompaction,
    FinalizeSnapshotInstallation {
        index: u64,
        term: u64,
        delete_through: Option<u64>,
        id: String,
        snapshot: Box<Cursor<Vec<u8>>>,
    },
    GetCurrentSnapshot,
    GetValue(String),
    GetTargetAddr(u64),
}

pub enum StoreResponse {
    MembershipConfig(MembershipConfig),
    InitialState(InitialState),
    LogEntries(Vec<Entry<ClientRequest>>),
    Response(ClientResponse),
    Snapshot(CurrentSnapshotData<Cursor<Vec<u8>>>),
    OptionSnapshot(Option<CurrentSnapshotData<Cursor<Vec<u8>>>>),
    Value(Option<Arc<String>>),
    TargetAddr(Option<Arc<String>>),
    None,
}

impl Handler<StoreRequest> for InnerStore {
    type Result = anyhow::Result<StoreResponse>;
    fn handle(&mut self, msg: StoreRequest, ctx: &mut Self::Context) -> Self::Result {
        tracing::debug!("inner store handler {:?}", &msg);
        match msg {
            StoreRequest::GetMembershipConfig => {
                let v = self.get_membership_config()?;
                Ok(StoreResponse::MembershipConfig(v))
            },
            StoreRequest::GetInitialState => {
                let v = self.get_initial_state()?;
                Ok(StoreResponse::InitialState(v))
            },
            StoreRequest::SaveHardState(v) => {
                self.save_hard_state(v)?;
                Ok(StoreResponse::None)
            },
            StoreRequest::GetLogEntries { start, stop } => {
                let v = self.get_log_entries(start, stop)?;
                Ok(StoreResponse::LogEntries(v))
            },
            StoreRequest::DeleteLogsFrom { start, stop } => {
                self.delete_logs_from(start, stop)?;
                Ok(StoreResponse::None)
            },
            StoreRequest::AppendEntryToLog(v) => {
                self.append_entry_to_log(v)?;
                Ok(StoreResponse::None)
            },
            StoreRequest::ReplicateToLog(v) => {
                self.replicate_to_log(v)?;
                Ok(StoreResponse::None)
            },
            /* 
            StoreRequest::ApplyEntryToStateMachine { index, request } => {
                let v=self.apply_entry_to_state_machine(index, request)?;
                Ok(StoreResponse::Response(v))
            },
            StoreRequest::ReplicateToStateMachine(v) => {
                self.replicate_to_state_machine(v)?;
                Ok(StoreResponse::None)
            },
            */
            StoreRequest::ApplyEntryToStateMachineRange{start,end} => {
                self.replicate_to_state_machine_range(start, end)?;
                Ok(StoreResponse::None)
            }
            StoreRequest::DoLogCompaction => {
                let v = self.do_log_compaction()?;
                Ok(StoreResponse::Snapshot(v))
            },
            StoreRequest::FinalizeSnapshotInstallation { index, term, delete_through, id, snapshot } => {
                self.finalize_snapshot_installation(index, term, delete_through, id, snapshot)?;
                Ok(StoreResponse::None)
            },
            StoreRequest::GetCurrentSnapshot => {
                let v = self.get_current_snapshot()?;
                Ok(StoreResponse::OptionSnapshot(v))
            },
            StoreRequest::GetValue(key)=> {
                let v = self.state_machine.data.get(&key).map(|v|v.to_owned());
                Ok(StoreResponse::Value(v))
            }
            StoreRequest::GetTargetAddr(id)=> {
                let v = self.membership.node_addr.get(&id).map(|v|v.clone());
                Ok(StoreResponse::TargetAddr(v))
            }
        }
    }
}
