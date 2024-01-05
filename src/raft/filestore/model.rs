use std::{borrow::Cow, sync::Arc};

use binrw::prelude::*;

use super::log::{LogRecord, SnapshotHeader, LogSnapshotItem};

///
/// ----
/// index header 24 byte
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
    //本日志第一个序号
    pub first_index: u64,
    //数据区域相对地址, 4096
    pub data_area_index: u16,
    pub index_interval:u16,
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

#[derive(Debug,Clone)]
pub struct LogRecordDto {
    pub index: u64,
    pub term: u64,
    pub tree: String,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub op_type: u32,
}

unsafe impl Send for LogRecordDto{}
unsafe impl Sync for LogRecordDto{}

impl LogRecordDto {
    pub fn to_record_do(&self) -> LogRecord {
        LogRecord{
            index: self.index,
            term: self.term,
            tree: Cow::Borrowed(&self.tree),
            key: Cow::Borrowed(&self.key),
            value: Cow::Borrowed(&self.value),
            op_type: self.op_type,
        }
    }
}

impl <'a> From<LogRecord<'a>> for LogRecordDto {
    fn from(value: LogRecord<'a>) -> Self {
        Self {
            index: value.index,
            term: value.term,
            tree: value.tree.to_string(),
            key: value.key.to_vec(),
            value: value.value.to_vec(),
            op_type: value.op_type,
        }
    }
}


#[derive(Debug,Clone)]
pub struct SnapshotHeaderDto {
    pub last_index: u64,
    pub last_term: u64,
    pub member: Vec<u64>,
    pub member_after_consensus: Vec<u64>,
}

impl <'a> From<SnapshotHeader<'a>> for SnapshotHeaderDto {
    fn from(value: SnapshotHeader<'a>) -> Self {
        Self {
            last_index: value.last_index,
            last_term: value.last_term,
            member: value.member,
            member_after_consensus: value.member_after_consensus,
        }
    }
}

impl SnapshotHeaderDto { 
    pub fn to_record_do(self) -> SnapshotHeader<'static> {
        SnapshotHeader{
            last_index: self.last_index,
            last_term: self.last_term,
            member: self.member,
            member_after_consensus: self.member_after_consensus,
            extend: Cow::Owned(Vec::new()),
        }
    }
}

#[derive(Debug)]
pub struct SnapshotRecordDto{
    pub tree: Arc<String>,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub op_type:u32,
}

impl <'a> From<LogSnapshotItem<'a>> for SnapshotRecordDto {
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
    pub fn to_record_do(&self) -> LogSnapshotItem {
        LogSnapshotItem {
            tree: Cow::Borrowed(self.tree.as_ref()),
            key: Cow::Borrowed(&self.key),
            value: Cow::Borrowed(&self.value),
            op_type: self.op_type,
        }
    }
}