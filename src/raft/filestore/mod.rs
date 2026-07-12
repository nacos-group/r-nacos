use async_raft_ext::raft::{Entry, EntryPayload};

use self::model::LogRecordDto;

use super::store::ClientRequest;

pub mod core;
pub mod log;
pub mod model;
pub mod raftapply;
pub mod raftdata;
pub mod raftindex;
pub mod raftlog;
pub mod raftsnapshot;

pub struct StoreUtils;

impl StoreUtils {
    pub fn log_record_to_entry(record: LogRecordDto) -> anyhow::Result<Entry<ClientRequest>> {
        let payload = match serde_json::from_slice::<EntryPayload<ClientRequest>>(&record.value) {
            Ok(v) => v,
            Err(e) => {
                let value_str: String = String::from_utf8_lossy(&record.value)
                    .chars()
                    .take(2048)
                    .collect();
                return Err(anyhow::anyhow!(
                    "There is an error in the log format, {} ; index:{}, source value(take 2048):{}",
                    e,
                    record.index,
                    value_str
                ));
            }
        };
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
