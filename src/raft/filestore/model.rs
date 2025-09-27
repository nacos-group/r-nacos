use std::{borrow::Cow, collections::HashMap, sync::Arc};

use async_raft_ext::raft::InstallSnapshotRequest;
use async_trait::async_trait;
use binrw::prelude::*;
use prost::Message;

use crate::raft::store::ClientRequest;

use super::log::{
    LogRange, LogRecord, LogSnapshotItem, NodeAddrItem, RaftIndex, SnapshotHeader, SnapshotRange,
};

pub const LOG_INDEX_HEADER_LEN: u64 = 32;

///
/// ----
/// index header 32 byte
/// ----
/// index data end:4095; 每个索引记录相对位置，索引大小固定
/// ----
/// data
/// ----
#[binrw]
#[derive(Debug)]
pub struct LogIndexHeaderDo {
    //魔法值 0x42313644 "raft"
    pub magic: u32,
    //版本号
    pub version: u16,
    //本日志之前的term
    pub last_term: u64,
    //本日志第一个序号
    pub first_index: u64,
    //数据区域相对地址, 4096
    pub data_area_index: u16,
    pub index_interval: u16,
    pub all_index_count: u16,
    pub status: u8,
    pub ext1: u8,
    pub ext2: u8,
    pub ext3: u8,
}

impl LogIndexHeaderDo {
    pub fn new() -> Self {
        Self {
            magic: 0x42313644,
            version: 0,
            last_term: 0,
            first_index: 0,
            data_area_index: 4096,
            index_interval: 128,
            all_index_count: 0,
            status: 0,
            ext1: 0,
            ext2: 0,
            ext3: 0,
        }
    }
}

impl Default for LogIndexHeaderDo {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default)]
pub struct LogRecordDto {
    pub index: u64,
    pub term: u64,
    //pub tree: String,
    //pub key: Vec<u8>,
    pub value: Vec<u8>,
    //pub op_type: u32,
}

//unsafe impl Send for LogRecordDto {}
//unsafe impl Sync for LogRecordDto {}

impl LogRecordDto {
    pub fn to_record_do(&self) -> LogRecord<'_> {
        LogRecord {
            index: self.index,
            term: self.term,
            //tree: Cow::Borrowed(&self.tree),
            //key: Cow::Borrowed(&self.key),
            value: Cow::Borrowed(&self.value),
            //op_type: self.op_type,
        }
    }
}

impl<'a> From<LogRecord<'a>> for LogRecordDto {
    fn from(value: LogRecord<'a>) -> Self {
        Self {
            index: value.index,
            term: value.term,
            //tree: value.tree.to_string(),
            //key: value.key.to_vec(),
            value: value.value.to_vec(),
            //op_type: value.op_type,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SnapshotHeaderDto {
    pub last_index: u64,
    pub last_term: u64,
    pub member: Vec<u64>,
    pub member_after_consensus: Vec<u64>,
    pub node_addrs: HashMap<u64, Arc<String>>,
}

impl<'a> From<SnapshotHeader<'a>> for SnapshotHeaderDto {
    fn from(value: SnapshotHeader<'a>) -> Self {
        let mut node_addrs = HashMap::new();
        for item in value.node_addrs {
            node_addrs.insert(item.id, Arc::new(item.addr.to_string()));
        }
        Self {
            last_index: value.last_index,
            last_term: value.last_term,
            member: value.member,
            member_after_consensus: value.member_after_consensus,
            node_addrs,
        }
    }
}

impl SnapshotHeaderDto {
    pub fn to_record_do(&self) -> SnapshotHeader<'_> {
        let mut node_addrs = Vec::with_capacity(self.node_addrs.len());
        for item in self.node_addrs.iter() {
            node_addrs.push(NodeAddrItem {
                id: item.0.to_owned(),
                addr: Cow::Borrowed(item.1.as_str()),
            });
        }
        SnapshotHeader {
            last_index: self.last_index,
            last_term: self.last_term,
            member: self.member.clone(),
            member_after_consensus: self.member_after_consensus.clone(),
            node_addrs,
            extend: Cow::Owned(Vec::new()),
        }
    }
}

#[derive(Debug)]
pub struct SnapshotRecordDto {
    pub tree: Arc<String>,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub op_type: u32,
}

impl<'a> From<LogSnapshotItem<'a>> for SnapshotRecordDto {
    fn from(value: LogSnapshotItem<'a>) -> Self {
        Self {
            tree: Arc::new(value.tree.into_owned()),
            key: value.key.into_owned(),
            value: value.value.into_owned(),
            op_type: value.op_type,
        }
    }
}

impl SnapshotRecordDto {
    pub fn to_record_do(&self) -> LogSnapshotItem<'_> {
        LogSnapshotItem {
            tree: Cow::Borrowed(self.tree.as_ref()),
            key: Cow::Borrowed(&self.key),
            value: Cow::Borrowed(&self.value),
            op_type: self.op_type,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RaftIndexDto {
    pub logs: Vec<LogRange>,
    pub current_log: u64,
    pub snapshots: Vec<SnapshotRange>,
    pub last_snapshot: u64,
    pub last_snapshot_index: u64,
    pub last_snapshot_term: u64,
    pub current_term: u64,
    pub voted_for: u64,
    pub member: Vec<u64>,
    pub member_after_consensus: Vec<u64>,
    pub node_addrs: HashMap<u64, Arc<String>>,
}

impl<'a> From<RaftIndex<'a>> for RaftIndexDto {
    fn from(value: RaftIndex<'a>) -> Self {
        let mut node_addrs = HashMap::new();
        for item in value.node_addrs {
            node_addrs.insert(item.id, Arc::new(item.addr.to_string()));
        }
        Self {
            logs: value.logs.clone(),
            current_log: value.current_log,
            snapshots: value.snapshots.clone(),
            last_snapshot: value.last_snapshot,
            last_snapshot_index: value.last_snapshot_index,
            last_snapshot_term: value.last_snapshot_term,
            current_term: value.current_term,
            voted_for: value.voted_for,
            member: value.member.clone(),
            member_after_consensus: value.member_after_consensus.clone(),
            node_addrs,
        }
    }
}

impl RaftIndexDto {
    pub fn to_record_do(&self) -> RaftIndex<'_> {
        let mut node_addrs = Vec::with_capacity(self.node_addrs.len());
        for item in self.node_addrs.iter() {
            node_addrs.push(NodeAddrItem {
                id: item.0.to_owned(),
                addr: Cow::Borrowed(item.1.as_str()),
            });
        }
        RaftIndex {
            logs: self.logs.clone(),
            current_log: self.current_log,
            snapshots: self.snapshots.clone(),
            last_snapshot: self.last_snapshot,
            last_snapshot_index: self.last_snapshot_index,
            last_snapshot_term: self.last_snapshot_term,
            current_term: self.current_term,
            voted_for: self.voted_for,
            member: self.member.clone(),
            member_after_consensus: self.member_after_consensus.clone(),
            node_addrs,
        }
    }
}

#[async_trait]
pub trait LogRecordLoader {
    async fn load(&self, dto: LogRecordDto) -> anyhow::Result<()>;
}

#[derive(Debug, Default, Clone)]
pub struct LogIndexInfo {
    pub index: u64,
    pub term: u64,
}

#[derive(Debug, Clone, Default)]
pub struct MemberShip {
    pub member: Vec<u64>,
    pub member_after_consensus: Vec<u64>,
    pub node_addrs: HashMap<u64, Arc<String>>,
}

#[derive(Debug, Clone)]
pub struct ApplyRequestDto {
    pub index: u64,
    pub request: ClientRequest,
}

impl ApplyRequestDto {
    pub fn new(index: u64, request: ClientRequest) -> Self {
        Self { index, request }
    }
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct InstallSnapshotRequestDto {
    /// The leader's current term.
    #[prost(uint64, tag = "1")]
    pub term: u64,
    /// The leader's ID. Useful in redirecting clients.
    #[prost(uint64, tag = "2")]
    pub leader_id: u64,
    /// The snapshot replaces all log entries up through and including this index.
    #[prost(uint64, tag = "3")]
    pub last_included_index: u64,
    /// The term of the `last_included_index`.
    #[prost(uint64, tag = "4")]
    pub last_included_term: u64,
    /// The byte offset where this chunk of data is positioned in the snapshot file.
    #[prost(uint64, tag = "5")]
    pub offset: u64,
    /// The raw bytes of the snapshot chunk, starting at `offset`.
    #[prost(bytes, tag = "6")]
    pub data: Vec<u8>,
    /// Will be `true` if this is the last chunk in the snapshot.
    #[prost(bool, tag = "7")]
    pub done: bool,
}

impl From<InstallSnapshotRequest> for InstallSnapshotRequestDto {
    fn from(value: InstallSnapshotRequest) -> Self {
        Self {
            term: value.term,
            leader_id: value.leader_id,
            last_included_index: value.last_included_index,
            last_included_term: value.last_included_term,
            offset: value.offset,
            data: value.data,
            done: value.done,
        }
    }
}

impl From<InstallSnapshotRequestDto> for InstallSnapshotRequest {
    fn from(value: InstallSnapshotRequestDto) -> Self {
        Self {
            term: value.term,
            leader_id: value.leader_id,
            last_included_index: value.last_included_index,
            last_included_term: value.last_included_term,
            offset: value.offset,
            data: value.data,
            done: value.done,
        }
    }
}

impl InstallSnapshotRequestDto {
    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let mut data_bytes: Vec<u8> = Vec::new();
        self.encode(&mut data_bytes)?;
        Ok(data_bytes)
    }

    pub fn from_bytes(buf: &[u8]) -> anyhow::Result<Self> {
        let s = InstallSnapshotRequestDto::decode(buf)?;
        Ok(s)
    }
}
