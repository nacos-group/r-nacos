// Automatically generated rust module for 'service_meta.proto' file

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]
#![allow(unknown_lints)]
#![allow(clippy::all)]
#![cfg_attr(rustfmt, rustfmt_skip)]


use std::borrow::Cow;
use std::collections::HashMap;
type KVMap<K, V> = HashMap<K, V>;
use quick_protobuf::{MessageInfo, MessageRead, MessageWrite, BytesReader, Writer, WriterBackend, Result};
use quick_protobuf::sizeofs::*;
use super::*;

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct InstanceMetaDo<'a> {
    pub namespace_id: Cow<'a, str>,
    pub group_name: Cow<'a, str>,
    pub service_name: Cow<'a, str>,
    pub ip: Cow<'a, str>,
    pub port: u32,
    pub metadata: KVMap<Cow<'a, str>, Cow<'a, str>>,
}

impl<'a> MessageRead<'a> for InstanceMetaDo<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.namespace_id = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(18) => msg.group_name = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(26) => msg.service_name = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(34) => msg.ip = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(40) => msg.port = r.read_uint32(bytes)?,
                Ok(50) => {
                    let (key, value) = r.read_map(bytes, |r, bytes| Ok(r.read_string(bytes).map(Cow::Borrowed)?), |r, bytes| Ok(r.read_string(bytes).map(Cow::Borrowed)?))?;
                    msg.metadata.insert(key, value);
                }
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for InstanceMetaDo<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.namespace_id == "" { 0 } else { 1 + sizeof_len((&self.namespace_id).len()) }
        + if self.group_name == "" { 0 } else { 1 + sizeof_len((&self.group_name).len()) }
        + if self.service_name == "" { 0 } else { 1 + sizeof_len((&self.service_name).len()) }
        + if self.ip == "" { 0 } else { 1 + sizeof_len((&self.ip).len()) }
        + if self.port == 0u32 { 0 } else { 1 + sizeof_varint(*(&self.port) as u64) }
        + self.metadata.iter().map(|(k, v)| 1 + sizeof_len(2 + sizeof_len((k).len()) + sizeof_len((v).len()))).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.namespace_id != "" { w.write_with_tag(10, |w| w.write_string(&**&self.namespace_id))?; }
        if self.group_name != "" { w.write_with_tag(18, |w| w.write_string(&**&self.group_name))?; }
        if self.service_name != "" { w.write_with_tag(26, |w| w.write_string(&**&self.service_name))?; }
        if self.ip != "" { w.write_with_tag(34, |w| w.write_string(&**&self.ip))?; }
        if self.port != 0u32 { w.write_with_tag(40, |w| w.write_uint32(*&self.port))?; }
        for (k, v) in self.metadata.iter() { w.write_with_tag(50, |w| w.write_map(2 + sizeof_len((k).len()) + sizeof_len((v).len()), 10, |w| w.write_string(&**k), 18, |w| w.write_string(&**v)))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct InstanceFileDo<'a> {
    pub namespace_id: Cow<'a, str>,
    pub group_name: Cow<'a, str>,
    pub service_name: Cow<'a, str>,
    pub file_name: Cow<'a, str>,
}

impl<'a> MessageRead<'a> for InstanceFileDo<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.namespace_id = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(18) => msg.group_name = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(26) => msg.service_name = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(34) => msg.file_name = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for InstanceFileDo<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.namespace_id == "" { 0 } else { 1 + sizeof_len((&self.namespace_id).len()) }
        + if self.group_name == "" { 0 } else { 1 + sizeof_len((&self.group_name).len()) }
        + if self.service_name == "" { 0 } else { 1 + sizeof_len((&self.service_name).len()) }
        + if self.file_name == "" { 0 } else { 1 + sizeof_len((&self.file_name).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.namespace_id != "" { w.write_with_tag(10, |w| w.write_string(&**&self.namespace_id))?; }
        if self.group_name != "" { w.write_with_tag(18, |w| w.write_string(&**&self.group_name))?; }
        if self.service_name != "" { w.write_with_tag(26, |w| w.write_string(&**&self.service_name))?; }
        if self.file_name != "" { w.write_with_tag(34, |w| w.write_string(&**&self.file_name))?; }
        Ok(())
    }
}

