use crate::common::byte_utils::{bin_to_id_result, id_to_bin};
use crate::common::constant::SEQUENCE_TREE_NAME;
use crate::raft::filestore::model::SnapshotRecordDto;
use crate::raft::filestore::raftapply::{RaftApplyDataRequest, RaftApplyDataResponse};
use crate::raft::filestore::raftsnapshot::{SnapshotWriterActor, SnapshotWriterRequest};
use crate::sequence::model::{SequenceRaftReq, SequenceRaftResult};
use actix::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct SequenceDbManager {
    /// value为下一次可用id
    pub(crate) seq_map: HashMap<Arc<String>, u64>,
    init: bool,
}

impl SequenceDbManager {
    pub fn new() -> Self {
        Self {
            seq_map: HashMap::new(),
            init: false,
        }
    }

    pub fn next_id(&mut self, key: Arc<String>) -> u64 {
        if let Some(id) = self.seq_map.get_mut(&key) {
            let old = *id;
            *id += 1;
            old
        } else {
            self.seq_map.insert(key.clone(), 1 + 1);
            1
        }
    }

    pub fn next_range(&mut self, key: Arc<String>, step: u64) -> anyhow::Result<u64> {
        if let Some(id) = self.seq_map.get_mut(&key) {
            let old = *id;
            *id += step;
            Ok(old)
        } else {
            self.seq_map.insert(key.clone(), step + 1);
            Ok(1)
        }
    }

    fn build_snapshot(&self, writer: Addr<SnapshotWriterActor>) -> anyhow::Result<()> {
        for (key, value) in &self.seq_map {
            let record = SnapshotRecordDto {
                tree: SEQUENCE_TREE_NAME.clone(),
                key: key.as_bytes().to_vec(),
                value: id_to_bin(*value),
                op_type: 0,
            };
            writer.do_send(SnapshotWriterRequest::Record(record));
        }
        Ok(())
    }

    fn load_snapshot_record(&mut self, record: SnapshotRecordDto) -> anyhow::Result<()> {
        let value = bin_to_id_result(&record.value)?;
        self.seq_map
            .insert(Arc::new(String::from_utf8(record.key)?), value);
        Ok(())
    }

    fn load_completed(&mut self, _ctx: &mut Context<Self>) -> anyhow::Result<()> {
        self.init = true;
        Ok(())
    }
}

impl Actor for SequenceDbManager {
    type Context = Context<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("SequenceDbManager started")
    }
}

impl Handler<SequenceRaftReq> for SequenceDbManager {
    type Result = anyhow::Result<SequenceRaftResult>;

    fn handle(&mut self, msg: SequenceRaftReq, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            SequenceRaftReq::NextId(key) => {
                let id = self.next_id(key);
                Ok(SequenceRaftResult::NextId(id))
            }
            SequenceRaftReq::NextRange(key, step) => {
                let start = self.next_range(key, step)?;
                Ok(SequenceRaftResult::NextRange { start, len: step })
            }
            SequenceRaftReq::SetId(key, id) => {
                self.seq_map.insert(key, id);
                Ok(SequenceRaftResult::None)
            }
            SequenceRaftReq::RemoveId(key) => {
                self.seq_map.remove(&key);
                Ok(SequenceRaftResult::None)
            }
        }
    }
}

impl Handler<RaftApplyDataRequest> for SequenceDbManager {
    type Result = anyhow::Result<RaftApplyDataResponse>;

    fn handle(&mut self, msg: RaftApplyDataRequest, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            RaftApplyDataRequest::BuildSnapshot(writer) => {
                self.build_snapshot(writer)?;
            }
            RaftApplyDataRequest::LoadSnapshotRecord(record) => {
                self.load_snapshot_record(record)?;
            }
            RaftApplyDataRequest::LoadCompleted => {
                self.load_completed(ctx)?;
            }
        }
        Ok(RaftApplyDataResponse::None)
    }
}
