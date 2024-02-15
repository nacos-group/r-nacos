use async_raft_ext::raft::{Entry, EntryPayload};

use self::model::LogRecordDto;

use super::store::ClientRequest;

pub mod log;
pub mod model;
pub mod raftindex;
pub mod raftlog;
pub mod raftsnapshot;
pub mod raftapply;
pub mod core;
mod raftdata;

pub struct StoreUtils;

impl StoreUtils {
    pub fn log_record_to_entry(record: LogRecordDto) -> anyhow::Result<Entry<ClientRequest>> {
        let payload: EntryPayload<ClientRequest> = serde_json::from_slice(&record.value)?;
        let entry = Entry {
            term: record.term,
            index: record.index,
            payload,
        };
        Ok(entry)
    }

    pub fn entry_to_record(entry: &Entry<ClientRequest>) -> anyhow::Result<LogRecordDto> {
        let value = serde_json::to_vec(&entry.payload)?;
        let record = LogRecordDto {
            index: entry.index,
            term: entry.term,
            value,
        };
        Ok(record)
    }
}
