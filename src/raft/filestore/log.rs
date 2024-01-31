// Automatically generated rust module for 'log-pb.proto' file

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]
#![allow(unknown_lints)]
#![allow(clippy::all)]
#![cfg_attr(rustfmt, rustfmt_skip)]


use std::borrow::Cow;
use quick_protobuf::{MessageInfo, MessageRead, MessageWrite, BytesReader, Writer, WriterBackend, Result};
use quick_protobuf::sizeofs::*;
use super::*;

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct LogRecord<'a> {
    pub index: u64,
    pub term: u64,
    pub value: Cow<'a, [u8]>,
}

impl<'a> MessageRead<'a> for LogRecord<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.index = r.read_uint64(bytes)?,
                Ok(16) => msg.term = r.read_uint64(bytes)?,
                Ok(42) => msg.value = r.read_bytes(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for LogRecord<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.index == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.index) as u64) }
        + if self.term == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.term) as u64) }
        + if self.value == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.value).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.index != 0u64 { w.write_with_tag(8, |w| w.write_uint64(*&self.index))?; }
        if self.term != 0u64 { w.write_with_tag(16, |w| w.write_uint64(*&self.term))?; }
        if self.value != Cow::Borrowed(b"") { w.write_with_tag(42, |w| w.write_bytes(&**&self.value))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct SnapshotHeader<'a> {
    pub last_index: u64,
    pub last_term: u64,
    pub member: Vec<u64>,
    pub member_after_consensus: Vec<u64>,
    pub node_addrs: Vec<log::NodeAddrItem<'a>>,
    pub extend: Cow<'a, [u8]>,
}

impl<'a> MessageRead<'a> for SnapshotHeader<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.last_index = r.read_uint64(bytes)?,
                Ok(16) => msg.last_term = r.read_uint64(bytes)?,
                Ok(26) => msg.member = r.read_packed(bytes, |r, bytes| Ok(r.read_uint64(bytes)?))?,
                Ok(34) => msg.member_after_consensus = r.read_packed(bytes, |r, bytes| Ok(r.read_uint64(bytes)?))?,
                Ok(42) => msg.node_addrs.push(r.read_message::<log::NodeAddrItem>(bytes)?),
                Ok(50) => msg.extend = r.read_bytes(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for SnapshotHeader<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.last_index == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.last_index) as u64) }
        + if self.last_term == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.last_term) as u64) }
        + if self.member.is_empty() { 0 } else { 1 + sizeof_len(self.member.iter().map(|s| sizeof_varint(*(s) as u64)).sum::<usize>()) }
        + if self.member_after_consensus.is_empty() { 0 } else { 1 + sizeof_len(self.member_after_consensus.iter().map(|s| sizeof_varint(*(s) as u64)).sum::<usize>()) }
        + self.node_addrs.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + if self.extend == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.extend).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.last_index != 0u64 { w.write_with_tag(8, |w| w.write_uint64(*&self.last_index))?; }
        if self.last_term != 0u64 { w.write_with_tag(16, |w| w.write_uint64(*&self.last_term))?; }
        w.write_packed_with_tag(26, &self.member, |w, m| w.write_uint64(*m), &|m| sizeof_varint(*(m) as u64))?;
        w.write_packed_with_tag(34, &self.member_after_consensus, |w, m| w.write_uint64(*m), &|m| sizeof_varint(*(m) as u64))?;
        for s in &self.node_addrs { w.write_with_tag(42, |w| w.write_message(s))?; }
        if self.extend != Cow::Borrowed(b"") { w.write_with_tag(50, |w| w.write_bytes(&**&self.extend))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct LogSnapshotItem<'a> {
    pub tree: Cow<'a, str>,
    pub key: Cow<'a, [u8]>,
    pub value: Cow<'a, [u8]>,
    pub op_type: u32,
}

impl<'a> MessageRead<'a> for LogSnapshotItem<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.tree = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(34) => msg.key = r.read_bytes(bytes).map(Cow::Borrowed)?,
                Ok(42) => msg.value = r.read_bytes(bytes).map(Cow::Borrowed)?,
                Ok(48) => msg.op_type = r.read_uint32(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for LogSnapshotItem<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.tree == "" { 0 } else { 1 + sizeof_len((&self.tree).len()) }
        + if self.key == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.key).len()) }
        + if self.value == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.value).len()) }
        + if self.op_type == 0u32 { 0 } else { 1 + sizeof_varint(*(&self.op_type) as u64) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.tree != "" { w.write_with_tag(10, |w| w.write_string(&**&self.tree))?; }
        if self.key != Cow::Borrowed(b"") { w.write_with_tag(34, |w| w.write_bytes(&**&self.key))?; }
        if self.value != Cow::Borrowed(b"") { w.write_with_tag(42, |w| w.write_bytes(&**&self.value))?; }
        if self.op_type != 0u32 { w.write_with_tag(48, |w| w.write_uint32(*&self.op_type))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct LogRange {
    pub id: u64,
    pub pre_term: u64,
    pub start_index: u64,
    pub record_count: u64,
    pub split_off_index: u64,
    pub is_close: bool,
    pub mark_remove: bool,
}

impl<'a> MessageRead<'a> for LogRange {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.id = r.read_uint64(bytes)?,
                Ok(16) => msg.pre_term = r.read_uint64(bytes)?,
                Ok(24) => msg.start_index = r.read_uint64(bytes)?,
                Ok(32) => msg.record_count = r.read_uint64(bytes)?,
                Ok(40) => msg.split_off_index = r.read_uint64(bytes)?,
                Ok(48) => msg.is_close = r.read_bool(bytes)?,
                Ok(56) => msg.mark_remove = r.read_bool(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for LogRange {
    fn get_size(&self) -> usize {
        0
        + if self.id == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.id) as u64) }
        + if self.pre_term == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.pre_term) as u64) }
        + if self.start_index == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.start_index) as u64) }
        + if self.record_count == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.record_count) as u64) }
        + if self.split_off_index == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.split_off_index) as u64) }
        + if self.is_close == false { 0 } else { 1 + sizeof_varint(*(&self.is_close) as u64) }
        + if self.mark_remove == false { 0 } else { 1 + sizeof_varint(*(&self.mark_remove) as u64) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.id != 0u64 { w.write_with_tag(8, |w| w.write_uint64(*&self.id))?; }
        if self.pre_term != 0u64 { w.write_with_tag(16, |w| w.write_uint64(*&self.pre_term))?; }
        if self.start_index != 0u64 { w.write_with_tag(24, |w| w.write_uint64(*&self.start_index))?; }
        if self.record_count != 0u64 { w.write_with_tag(32, |w| w.write_uint64(*&self.record_count))?; }
        if self.split_off_index != 0u64 { w.write_with_tag(40, |w| w.write_uint64(*&self.split_off_index))?; }
        if self.is_close != false { w.write_with_tag(48, |w| w.write_bool(*&self.is_close))?; }
        if self.mark_remove != false { w.write_with_tag(56, |w| w.write_bool(*&self.mark_remove))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct SnapshotRange {
    pub id: u64,
    pub end_index: u64,
}

impl<'a> MessageRead<'a> for SnapshotRange {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.id = r.read_uint64(bytes)?,
                Ok(16) => msg.end_index = r.read_uint64(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for SnapshotRange {
    fn get_size(&self) -> usize {
        0
        + if self.id == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.id) as u64) }
        + if self.end_index == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.end_index) as u64) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.id != 0u64 { w.write_with_tag(8, |w| w.write_uint64(*&self.id))?; }
        if self.end_index != 0u64 { w.write_with_tag(16, |w| w.write_uint64(*&self.end_index))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct NodeAddrItem<'a> {
    pub id: u64,
    pub addr: Cow<'a, str>,
}

impl<'a> MessageRead<'a> for NodeAddrItem<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.id = r.read_uint64(bytes)?,
                Ok(18) => msg.addr = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for NodeAddrItem<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.id == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.id) as u64) }
        + if self.addr == "" { 0 } else { 1 + sizeof_len((&self.addr).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.id != 0u64 { w.write_with_tag(8, |w| w.write_uint64(*&self.id))?; }
        if self.addr != "" { w.write_with_tag(18, |w| w.write_string(&**&self.addr))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct RaftIndex<'a> {
    pub logs: Vec<log::LogRange>,
    pub current_log: u64,
    pub snapshots: Vec<log::SnapshotRange>,
    pub last_snapshot: u64,
    pub last_snapshot_index: u64,
    pub last_snapshot_term: u64,
    pub current_term: u64,
    pub voted_for: u64,
    pub member: Vec<u64>,
    pub member_after_consensus: Vec<u64>,
    pub node_addrs: Vec<log::NodeAddrItem<'a>>,
}

impl<'a> MessageRead<'a> for RaftIndex<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.logs.push(r.read_message::<log::LogRange>(bytes)?),
                Ok(16) => msg.current_log = r.read_uint64(bytes)?,
                Ok(26) => msg.snapshots.push(r.read_message::<log::SnapshotRange>(bytes)?),
                Ok(32) => msg.last_snapshot = r.read_uint64(bytes)?,
                Ok(40) => msg.last_snapshot_index = r.read_uint64(bytes)?,
                Ok(48) => msg.last_snapshot_term = r.read_uint64(bytes)?,
                Ok(56) => msg.current_term = r.read_uint64(bytes)?,
                Ok(64) => msg.voted_for = r.read_uint64(bytes)?,
                Ok(74) => msg.member = r.read_packed(bytes, |r, bytes| Ok(r.read_uint64(bytes)?))?,
                Ok(82) => msg.member_after_consensus = r.read_packed(bytes, |r, bytes| Ok(r.read_uint64(bytes)?))?,
                Ok(90) => msg.node_addrs.push(r.read_message::<log::NodeAddrItem>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for RaftIndex<'a> {
    fn get_size(&self) -> usize {
        0
        + self.logs.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + if self.current_log == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.current_log) as u64) }
        + self.snapshots.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + if self.last_snapshot == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.last_snapshot) as u64) }
        + if self.last_snapshot_index == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.last_snapshot_index) as u64) }
        + if self.last_snapshot_term == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.last_snapshot_term) as u64) }
        + if self.current_term == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.current_term) as u64) }
        + if self.voted_for == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.voted_for) as u64) }
        + if self.member.is_empty() { 0 } else { 1 + sizeof_len(self.member.iter().map(|s| sizeof_varint(*(s) as u64)).sum::<usize>()) }
        + if self.member_after_consensus.is_empty() { 0 } else { 1 + sizeof_len(self.member_after_consensus.iter().map(|s| sizeof_varint(*(s) as u64)).sum::<usize>()) }
        + self.node_addrs.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.logs { w.write_with_tag(10, |w| w.write_message(s))?; }
        if self.current_log != 0u64 { w.write_with_tag(16, |w| w.write_uint64(*&self.current_log))?; }
        for s in &self.snapshots { w.write_with_tag(26, |w| w.write_message(s))?; }
        if self.last_snapshot != 0u64 { w.write_with_tag(32, |w| w.write_uint64(*&self.last_snapshot))?; }
        if self.last_snapshot_index != 0u64 { w.write_with_tag(40, |w| w.write_uint64(*&self.last_snapshot_index))?; }
        if self.last_snapshot_term != 0u64 { w.write_with_tag(48, |w| w.write_uint64(*&self.last_snapshot_term))?; }
        if self.current_term != 0u64 { w.write_with_tag(56, |w| w.write_uint64(*&self.current_term))?; }
        if self.voted_for != 0u64 { w.write_with_tag(64, |w| w.write_uint64(*&self.voted_for))?; }
        w.write_packed_with_tag(74, &self.member, |w, m| w.write_uint64(*m), &|m| sizeof_varint(*(m) as u64))?;
        w.write_packed_with_tag(82, &self.member_after_consensus, |w, m| w.write_uint64(*m), &|m| sizeof_varint(*(m) as u64))?;
        for s in &self.node_addrs { w.write_with_tag(90, |w| w.write_message(s))?; }
        Ok(())
    }
}

