// Automatically generated rust module for 'transfer.proto' file

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
pub struct SnapshotHeader<'a> {
    pub version: u64,
    pub modify_time: u64,
    pub from_sys: Cow<'a, str>,
    pub table_name_map_entities: Vec<transfer::TableNameMapEntity<'a>>,
    pub extend: Cow<'a, [u8]>,
}

impl<'a> MessageRead<'a> for SnapshotHeader<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.version = r.read_uint64(bytes)?,
                Ok(16) => msg.modify_time = r.read_uint64(bytes)?,
                Ok(26) => msg.from_sys = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(34) => msg.table_name_map_entities.push(r.read_message::<transfer::TableNameMapEntity>(bytes)?),
                Ok(42) => msg.extend = r.read_bytes(bytes).map(Cow::Borrowed)?,
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
        + if self.version == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.version) as u64) }
        + if self.modify_time == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.modify_time) as u64) }
        + if self.from_sys == "" { 0 } else { 1 + sizeof_len((&self.from_sys).len()) }
        + self.table_name_map_entities.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + if self.extend == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.extend).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.version != 0u64 { w.write_with_tag(8, |w| w.write_uint64(*&self.version))?; }
        if self.modify_time != 0u64 { w.write_with_tag(16, |w| w.write_uint64(*&self.modify_time))?; }
        if self.from_sys != "" { w.write_with_tag(26, |w| w.write_string(&**&self.from_sys))?; }
        for s in &self.table_name_map_entities { w.write_with_tag(34, |w| w.write_message(s))?; }
        if self.extend != Cow::Borrowed(b"") { w.write_with_tag(42, |w| w.write_bytes(&**&self.extend))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct TableNameMapEntity<'a> {
    pub id: u32,
    pub name: Cow<'a, str>,
}

impl<'a> MessageRead<'a> for TableNameMapEntity<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.id = r.read_uint32(bytes)?,
                Ok(18) => msg.name = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for TableNameMapEntity<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.id == 0u32 { 0 } else { 1 + sizeof_varint(*(&self.id) as u64) }
        + if self.name == "" { 0 } else { 1 + sizeof_len((&self.name).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.id != 0u32 { w.write_with_tag(8, |w| w.write_uint32(*&self.id))?; }
        if self.name != "" { w.write_with_tag(18, |w| w.write_string(&**&self.name))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct SnapshotItem<'a> {
    pub table_name: Cow<'a, str>,
    pub table_id: u32,
    pub key: Cow<'a, [u8]>,
    pub value: Cow<'a, [u8]>,
}

impl<'a> MessageRead<'a> for SnapshotItem<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.table_name = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(16) => msg.table_id = r.read_uint32(bytes)?,
                Ok(26) => msg.key = r.read_bytes(bytes).map(Cow::Borrowed)?,
                Ok(34) => msg.value = r.read_bytes(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for SnapshotItem<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.table_name == "" { 0 } else { 1 + sizeof_len((&self.table_name).len()) }
        + if self.table_id == 0u32 { 0 } else { 1 + sizeof_varint(*(&self.table_id) as u64) }
        + if self.key == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.key).len()) }
        + if self.value == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.value).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.table_name != "" { w.write_with_tag(10, |w| w.write_string(&**&self.table_name))?; }
        if self.table_id != 0u32 { w.write_with_tag(16, |w| w.write_uint32(*&self.table_id))?; }
        if self.key != Cow::Borrowed(b"") { w.write_with_tag(26, |w| w.write_bytes(&**&self.key))?; }
        if self.value != Cow::Borrowed(b"") { w.write_with_tag(34, |w| w.write_bytes(&**&self.value))?; }
        Ok(())
    }
}

