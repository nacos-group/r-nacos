#![allow(clippy::boxed_local)]
use std::collections::BTreeMap;

use crate::common::byte_utils::{bin_to_id, id_to_bin};
use crate::common::sled_utils::TABLE_SEQUENCE_TREE_NAME;
use crate::config::core::ConfigActor;
use crate::config::model::ConfigRaftCmd;

use super::{
    ClientRequest, ClientResponse, Membership, SnapshotDataInfo, SnapshotItem, SnapshotMeta,
    StateMachine,
};
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
    db: Arc<sled::Db>,
    config_addr: Addr<ConfigActor>,
    id: NodeId,
    /// The Raft log.
    log: BTreeMap<u64, Entry<ClientRequest>>,

    /// The Raft state machine.
    pub state_machine: StateMachine,

    /// The current granted vote.
    pub hs: Option<HardState>,

    membership: Membership,
}

impl InnerStore {
    pub fn new(id: u64, db: Arc<sled::Db>, config_addr: Addr<ConfigActor>) -> Self {
        Self {
            id,
            config_addr,
            db,
            log: Default::default(),
            state_machine: Default::default(),
            hs: None,
            membership: Default::default(),
        }
    }

    fn init(&mut self) -> anyhow::Result<()> {
        //load last log
        let logs_tree = logs(&self.db);
        if let Ok(Some((key, val))) = logs_tree.last() {
            let entry: async_raft::raft::Entry<ClientRequest> = serde_json::from_slice(&val)?;
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

    fn set_hard_state_(&self, hs: &HardState) -> anyhow::Result<()> {
        let store_tree = store(&self.db);
        let val = serde_json::to_vec(hs)?;
        store_tree.insert(b"hs", val)?;
        //store_tree.flush()?;
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

    fn set_membership_(&self, info: &Membership) -> anyhow::Result<()> {
        let store_tree = store(&self.db);
        let val = serde_json::to_vec(info)?;
        store_tree.insert(b"membership", val)?;
        //store_tree.flush()?;
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

    fn set_snapshot_(&self, data: &[u8]) -> anyhow::Result<()> {
        let store_tree = store(&self.db);
        store_tree.insert(b"snapshot", data)?;
        //store_tree.flush()?;
        Ok(())
    }

    fn get_snapshot_(&self) -> anyhow::Result<Option<(SnapshotDataInfo, Vec<u8>)>> {
        let store_tree = store(&self.db);
        let val = match store_tree.get(b"snapshot")? {
            Some(v) => Some((SnapshotDataInfo::from_bytes(&v)?, v.to_vec())),
            None => None,
        };
        Ok(val)
    }

    fn set_last_applied_log_(&self, log_id: u64) -> anyhow::Result<()> {
        let state_machine = store(&self.db);
        let value = id_to_bin(log_id);
        state_machine.insert(b"last_applied_log", value)?;
        //state_machine.flush()?;
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
        //Ok(self.membership.membership_config.to_owned())
        match self.membership.membership_config.as_ref() {
            Some(v) => Ok(v.to_owned()),
            None => {
                Ok(MembershipConfig::new_initial(self.id))
                //Err(anyhow::anyhow!("membership is none"))
            }
        }
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
                    last_log_index,
                    last_log_term,
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
        if start_index > end_index {
            tracing::error!("invalid request, start > stop");
            return Ok(vec![]);
        }
        if start_index == end_index {
            return Ok(vec![]);
        }

        let start = id_to_bin(start_index);
        let end = id_to_bin(end_index);
        let range = start_index..end_index;
        if self.log.contains_key(&start_index) {
            let response = self
                .log
                .range(range)
                .map(|(_, val)| val.clone())
                .collect::<Vec<_>>();
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
                let entry: async_raft::raft::Entry<ClientRequest> =
                    serde_json::from_slice(&val).unwrap();
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
        } else {
            self.log.split_off(&start);
            0xff_ff_ff_ff_ff_ff_ff_ff
        };
        // Else, just split off the remainder.

        let from = id_to_bin(start);
        let to = id_to_bin(to_index);
        let logs_tree = logs(&self.db);
        let entries = logs_tree.range::<&[u8], _>(from.as_slice()..to.as_slice());
        let mut batch_del = sled::Batch::default();
        for entry_res in entries {
            let (key, _) = entry_res.expect("Read db entry failed");
            batch_del.remove(key);
        }
        logs_tree.apply_batch(batch_del)?;
        //logs_tree.flush()?;

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
        //logs_tree.flush()?;

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
        //logs_tree.flush()?;
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

    fn replicate_to_state_machine_range(&mut self, start: u64, end: u64) -> anyhow::Result<()> {
        let entries: Vec<(u64, &Entry<ClientRequest>)> = self
            .log
            .range(start..end)
            .map(|(idx, entry)| (*idx, entry))
            .collect();
        //let mut cmds = vec![];
        if entries.is_empty() {
            return Ok(());
        }
        let last_index = entries[entries.len() - 1].0;
        for (_index, entry) in entries {
            match &entry.payload {
                EntryPayload::Normal(normal) => {
                    let req = normal.data.to_owned();
                    match req {
                        ClientRequest::ConfigSet {
                            key,
                            value,
                            history_id,
                            history_table_id,
                        } => {
                            let cmd = ConfigRaftCmd::ConfigAdd {
                                key,
                                value,
                                history_id,
                                history_table_id,
                            };
                            self.config_addr.do_send(cmd);
                            //self.wait_send_config_raft_cmd(cmd,ctx).ok();
                        }
                        ClientRequest::ConfigRemove { key } => {
                            let cmd = ConfigRaftCmd::ConfigRemove { key };
                            self.config_addr.do_send(cmd);
                            //self.wait_send_config_raft_cmd(cmd,ctx).ok();
                        }
                        ClientRequest::NodeAddr { id, addr } => {
                            self.membership.node_addr.insert(id, addr);
                            self.set_membership_(&self.membership)?;
                        }
                    }
                }
                EntryPayload::ConfigChange(config_change) => {
                    self.membership.membership_config = Some(config_change.membership.to_owned());
                    self.set_membership_(&self.membership)?;
                }
                _ => {}
            }
        }
        self.state_machine.last_applied_log = last_index;
        self.set_last_applied_log_(last_index)?;
        Ok(())
    }

    fn do_log_compaction(
        &mut self,
    ) -> anyhow::Result<async_raft::storage::CurrentSnapshotData<Cursor<Vec<u8>>>> {
        let index = self.state_machine.last_applied_log;
        let term = self
            .get_log_entries(index, index + 1)?
            .first()
            .map(|e| e.term)
            .unwrap_or_default();
        let meta = SnapshotMeta {
            term,
            index,
            membership: self.membership.to_owned(),
        };
        let data = self.build_snapshot_data(&meta)?;
        let membership_config = self.get_membership_config()?;
        self.set_snapshot_(&data)?;
        let snapshot = CurrentSnapshotData {
            term,
            index,
            membership: membership_config,
            snapshot: Box::new(Cursor::new(data)),
        };
        Ok(snapshot)
    }

    /// load config history data
    fn load_config_history(
        &self,
        history_name: Vec<u8>,
        items: &mut Vec<SnapshotItem>,
    ) -> anyhow::Result<()> {
        let config_history_tree = self.db.open_tree(history_name)?;
        let mut iter = config_history_tree.iter();
        while let Some(Ok((k, v))) = iter.next() {
            let item = SnapshotItem {
                r#type: 2,
                key: k.to_vec(),
                value: v.to_vec(),
            };
            items.push(item);
        }
        Ok(())
    }

    /// load table data
    fn load_table_seq(&self, items: &mut Vec<SnapshotItem>) -> anyhow::Result<()> {
        let tree = self.db.open_tree(TABLE_SEQUENCE_TREE_NAME)?;
        let mut iter = tree.iter();
        while let Some(Ok((k, v))) = iter.next() {
            let item = SnapshotItem {
                r#type: 3,
                key: k.to_vec(),
                value: v.to_vec(),
            };
            items.push(item);
        }
        Ok(())
    }

    fn build_snapshot_data(&mut self, meta: &SnapshotMeta) -> anyhow::Result<Vec<u8>> {
        // load config
        let mut items = vec![];
        let config_tree = self.db.open_tree("config").unwrap();
        let mut iter = config_tree.iter();
        while let Some(Ok((k, v))) = iter.next() {
            let mut key = k.to_vec();
            let item = SnapshotItem {
                r#type: 1,
                key: key.clone(),
                value: v.to_vec(),
            };
            items.push(item);
            let history_name =
                crate::config::config_sled::Config::build_config_history_tree_name(&mut key);
            self.load_config_history(history_name, &mut items)?;
        }
        self.load_table_seq(&mut items)?;
        let snapshot_meta_json = serde_json::to_string(meta)?;

        let snapshot_data = SnapshotDataInfo {
            items,
            snapshot_meta_json,
        };

        snapshot_data.to_bytes()
    }

    fn finalize_snapshot_installation(
        &mut self,
        index: u64,
        term: u64,
        delete_through: Option<u64>,
        id: String,
        snapshot: Box<Cursor<Vec<u8>>>,
    ) -> anyhow::Result<()> {
        // Update log.
        // Go backwards through the log to find the most recent membership config <= the `through` index.
        let snapshot_data =
            SnapshotDataInfo::from_bytes(snapshot.get_ref().as_slice()).unwrap_or_default();
        let meta = snapshot_data.build_snapshot_meta()?;
        let membership_config = meta
            .membership
            .membership_config
            .to_owned()
            .unwrap_or(MembershipConfig::new_initial(self.id));

        match &delete_through {
            Some(through) => {
                self.delete_logs_from(0, Some(*through + 1))?;
            }
            None => {
                self.log.clear();
                self.db.drop_tree("raft_logs")?;
            }
        }
        let entry = Entry::new_snapshot_pointer(index, term, id, membership_config);
        self.append_entry_to_log(entry)?;
        //self.log.insert(index, Entry::new_snapshot_pointer(index, term, id, membership_config));
        // Update the state machine.
        self.membership = meta.membership;
        self.install_snapshot_data(snapshot_data.items).ok();
        // Update current snapshot.
        self.set_snapshot_(snapshot.get_ref().as_slice())?;
        let cmd = ConfigRaftCmd::ApplySnaphot;
        self.config_addr.do_send(cmd);
        //self.wait_send_config_raft_cmd(cmd,ctx).ok();

        Ok(())
    }

    fn install_snapshot_data(&self, items: Vec<SnapshotItem>) -> anyhow::Result<()> {
        let config_tree = self.db.open_tree("config")?;
        let table_seq_tree = self.db.open_tree(TABLE_SEQUENCE_TREE_NAME)?;
        let mut last_history_tree = None;
        for item in items {
            match item.r#type {
                1 => {
                    //config
                    let mut key = item.key.clone();
                    config_tree.insert(item.key, item.value)?;
                    let history_name =
                        crate::config::config_sled::Config::build_config_history_tree_name(
                            &mut key,
                        );
                    last_history_tree = Some(self.db.open_tree(history_name)?);
                }
                2 => {
                    //config history
                    if let Some(last_history_tree) = &last_history_tree {
                        last_history_tree.insert(item.key, item.value)?;
                    }
                }
                3 => {
                    //table seq
                    table_seq_tree.insert(item.key, item.value)?;
                }
                _ => {} //ignore
            }
        }
        Ok(())
    }

    /*
    fn wait_send_config_raft_cmd(&mut self,cmd:ConfigRaftCmd,ctx: &mut Context<Self>) -> anyhow::Result<()> {
        let config_addr = self.config_addr.clone();
        async move {
            config_addr.send(cmd).await.ok();
        }
        .into_actor(self).map(|_,_,_|{})
        .wait(ctx);
        Ok(())
    }
    */

    fn get_current_snapshot(
        &mut self,
    ) -> anyhow::Result<Option<async_raft::storage::CurrentSnapshotData<Cursor<Vec<u8>>>>> {
        let snapshot = self.get_snapshot_()?;
        match snapshot {
            Some((snapshot, data)) => {
                let meta = snapshot.build_snapshot_meta()?;
                Ok(Some(CurrentSnapshotData {
                    index: meta.index,
                    term: meta.term,
                    membership: meta
                        .membership
                        .membership_config
                        .unwrap_or(MembershipConfig::new_initial(self.id)),
                    snapshot: Box::new(Cursor::new(data)),
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
    ApplyEntryToStateMachineRange {
        start: u64,
        end: u64,
    },
    DoLogCompaction,
    FinalizeSnapshotInstallation {
        index: u64,
        term: u64,
        delete_through: Option<u64>,
        id: String,
        snapshot: Box<Cursor<Vec<u8>>>,
    },
    GetCurrentSnapshot,
    GetTargetAddr(u64),
}

pub enum StoreResponse {
    MembershipConfig(MembershipConfig),
    InitialState(InitialState),
    LogEntries(Vec<Entry<ClientRequest>>),
    Response(ClientResponse),
    Snapshot(CurrentSnapshotData<Cursor<Vec<u8>>>),
    OptionSnapshot(Option<CurrentSnapshotData<Cursor<Vec<u8>>>>),
    TargetAddr(Option<Arc<String>>),
    None,
}

impl Handler<StoreRequest> for InnerStore {
    type Result = anyhow::Result<StoreResponse>;
    fn handle(&mut self, msg: StoreRequest, _ctx: &mut Self::Context) -> Self::Result {
        tracing::debug!("inner store handler {:?}", &msg);
        match msg {
            StoreRequest::GetMembershipConfig => {
                let v = self.get_membership_config()?;
                Ok(StoreResponse::MembershipConfig(v))
            }
            StoreRequest::GetInitialState => {
                let v = self.get_initial_state()?;
                Ok(StoreResponse::InitialState(v))
            }
            StoreRequest::SaveHardState(v) => {
                self.save_hard_state(v)?;
                Ok(StoreResponse::None)
            }
            StoreRequest::GetLogEntries { start, stop } => {
                let v = self.get_log_entries(start, stop)?;
                Ok(StoreResponse::LogEntries(v))
            }
            StoreRequest::DeleteLogsFrom { start, stop } => {
                self.delete_logs_from(start, stop)?;
                Ok(StoreResponse::None)
            }
            StoreRequest::AppendEntryToLog(v) => {
                self.append_entry_to_log(v)?;
                Ok(StoreResponse::None)
            }
            StoreRequest::ReplicateToLog(v) => {
                self.replicate_to_log(v)?;
                Ok(StoreResponse::None)
            }
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
            StoreRequest::ApplyEntryToStateMachineRange {
                start: source_start,
                end,
            } => {
                //let start = max(source_start,self.state_machine.last_applied_log+1);
                let start = self.state_machine.last_applied_log + 1;
                if start >= end {
                    log::warn!(
                        "ignore apply entry,source_start:{},start:{},end:{}",
                        &source_start,
                        &start,
                        &end
                    );
                    return Ok(StoreResponse::None);
                }
                self.replicate_to_state_machine_range(start, end)?;
                Ok(StoreResponse::None)
            }
            StoreRequest::DoLogCompaction => {
                let v = self.do_log_compaction()?;
                Ok(StoreResponse::Snapshot(v))
            }
            StoreRequest::FinalizeSnapshotInstallation {
                index,
                term,
                delete_through,
                id,
                snapshot,
            } => {
                self.finalize_snapshot_installation(index, term, delete_through, id, snapshot)?;
                Ok(StoreResponse::None)
            }
            StoreRequest::GetCurrentSnapshot => {
                let v = self.get_current_snapshot()?;
                Ok(StoreResponse::OptionSnapshot(v))
            }
            StoreRequest::GetTargetAddr(id) => {
                let v = self.membership.node_addr.get(&id).cloned();
                Ok(StoreResponse::TargetAddr(v))
            }
        }
    }
}
