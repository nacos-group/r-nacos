// Automatically generated rust module for 'data_object.proto' file

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
pub struct ToolSpecVersionDo<'a> {
    pub version: u64,
    pub parameters_json: Cow<'a, str>,
    pub op_user: Cow<'a, str>,
    pub update_time: i64,
}

impl<'a> MessageRead<'a> for ToolSpecVersionDo<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.version = r.read_uint64(bytes)?,
                Ok(18) => msg.parameters_json = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(26) => msg.op_user = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(32) => msg.update_time = r.read_int64(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for ToolSpecVersionDo<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.version == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.version) as u64) }
        + if self.parameters_json == "" { 0 } else { 1 + sizeof_len((&self.parameters_json).len()) }
        + if self.op_user == "" { 0 } else { 1 + sizeof_len((&self.op_user).len()) }
        + if self.update_time == 0i64 { 0 } else { 1 + sizeof_varint(*(&self.update_time) as u64) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.version != 0u64 { w.write_with_tag(8, |w| w.write_uint64(*&self.version))?; }
        if self.parameters_json != "" { w.write_with_tag(18, |w| w.write_string(&**&self.parameters_json))?; }
        if self.op_user != "" { w.write_with_tag(26, |w| w.write_string(&**&self.op_user))?; }
        if self.update_time != 0i64 { w.write_with_tag(32, |w| w.write_int64(*&self.update_time))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct McpToolSpecDo<'a> {
    pub namespace: Cow<'a, str>,
    pub group: Cow<'a, str>,
    pub tool_name: Cow<'a, str>,
    pub current_version: u64,
    pub create_time: i64,
    pub create_user: Cow<'a, str>,
    pub versions: Vec<data_object::ToolSpecVersionDo<'a>>,
}

impl<'a> MessageRead<'a> for McpToolSpecDo<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.namespace = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(18) => msg.group = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(26) => msg.tool_name = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(32) => msg.current_version = r.read_uint64(bytes)?,
                Ok(40) => msg.create_time = r.read_int64(bytes)?,
                Ok(50) => msg.create_user = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(58) => msg.versions.push(r.read_message::<data_object::ToolSpecVersionDo>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for McpToolSpecDo<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.namespace == "" { 0 } else { 1 + sizeof_len((&self.namespace).len()) }
        + if self.group == "" { 0 } else { 1 + sizeof_len((&self.group).len()) }
        + if self.tool_name == "" { 0 } else { 1 + sizeof_len((&self.tool_name).len()) }
        + if self.current_version == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.current_version) as u64) }
        + if self.create_time == 0i64 { 0 } else { 1 + sizeof_varint(*(&self.create_time) as u64) }
        + if self.create_user == "" { 0 } else { 1 + sizeof_len((&self.create_user).len()) }
        + self.versions.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.namespace != "" { w.write_with_tag(10, |w| w.write_string(&**&self.namespace))?; }
        if self.group != "" { w.write_with_tag(18, |w| w.write_string(&**&self.group))?; }
        if self.tool_name != "" { w.write_with_tag(26, |w| w.write_string(&**&self.tool_name))?; }
        if self.current_version != 0u64 { w.write_with_tag(32, |w| w.write_uint64(*&self.current_version))?; }
        if self.create_time != 0i64 { w.write_with_tag(40, |w| w.write_int64(*&self.create_time))?; }
        if self.create_user != "" { w.write_with_tag(50, |w| w.write_string(&**&self.create_user))?; }
        for s in &self.versions { w.write_with_tag(58, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct McpToolDo<'a> {
    pub tool_name: Cow<'a, str>,
    pub namespace: Cow<'a, str>,
    pub group: Cow<'a, str>,
    pub version: u64,
    pub route_rule_json: Cow<'a, str>,
}

impl<'a> MessageRead<'a> for McpToolDo<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(18) => msg.tool_name = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(26) => msg.namespace = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(34) => msg.group = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(40) => msg.version = r.read_uint64(bytes)?,
                Ok(50) => msg.route_rule_json = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for McpToolDo<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.tool_name == "" { 0 } else { 1 + sizeof_len((&self.tool_name).len()) }
        + if self.namespace == "" { 0 } else { 1 + sizeof_len((&self.namespace).len()) }
        + if self.group == "" { 0 } else { 1 + sizeof_len((&self.group).len()) }
        + if self.version == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.version) as u64) }
        + if self.route_rule_json == "" { 0 } else { 1 + sizeof_len((&self.route_rule_json).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.tool_name != "" { w.write_with_tag(18, |w| w.write_string(&**&self.tool_name))?; }
        if self.namespace != "" { w.write_with_tag(26, |w| w.write_string(&**&self.namespace))?; }
        if self.group != "" { w.write_with_tag(34, |w| w.write_string(&**&self.group))?; }
        if self.version != 0u64 { w.write_with_tag(40, |w| w.write_uint64(*&self.version))?; }
        if self.route_rule_json != "" { w.write_with_tag(50, |w| w.write_string(&**&self.route_rule_json))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct McpServerValueDo<'a> {
    pub id: u64,
    pub description: Cow<'a, str>,
    pub tools: Vec<data_object::McpToolDo<'a>>,
    pub op_user: Cow<'a, str>,
    pub update_time: i64,
}

impl<'a> MessageRead<'a> for McpServerValueDo<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.id = r.read_uint64(bytes)?,
                Ok(18) => msg.description = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(26) => msg.tools.push(r.read_message::<data_object::McpToolDo>(bytes)?),
                Ok(34) => msg.op_user = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(40) => msg.update_time = r.read_int64(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for McpServerValueDo<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.id == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.id) as u64) }
        + if self.description == "" { 0 } else { 1 + sizeof_len((&self.description).len()) }
        + self.tools.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + if self.op_user == "" { 0 } else { 1 + sizeof_len((&self.op_user).len()) }
        + if self.update_time == 0i64 { 0 } else { 1 + sizeof_varint(*(&self.update_time) as u64) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.id != 0u64 { w.write_with_tag(8, |w| w.write_uint64(*&self.id))?; }
        if self.description != "" { w.write_with_tag(18, |w| w.write_string(&**&self.description))?; }
        for s in &self.tools { w.write_with_tag(26, |w| w.write_message(s))?; }
        if self.op_user != "" { w.write_with_tag(34, |w| w.write_string(&**&self.op_user))?; }
        if self.update_time != 0i64 { w.write_with_tag(40, |w| w.write_int64(*&self.update_time))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct McpServerDo<'a> {
    pub id: u64,
    pub namespace: Cow<'a, str>,
    pub name: Cow<'a, str>,
    pub description: Cow<'a, str>,
    pub auth_keys: Vec<Cow<'a, str>>,
    pub create_time: i64,
    pub create_user: Cow<'a, str>,
    pub current_value: Option<data_object::McpServerValueDo<'a>>,
    pub release_value: Option<data_object::McpServerValueDo<'a>>,
    pub histories: Vec<data_object::McpServerValueDo<'a>>,
    pub unique_key: Cow<'a, str>,
}

impl<'a> MessageRead<'a> for McpServerDo<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.id = r.read_uint64(bytes)?,
                Ok(18) => msg.namespace = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(26) => msg.name = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(34) => msg.description = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(42) => msg.auth_keys.push(r.read_string(bytes).map(Cow::Borrowed)?),
                Ok(48) => msg.create_time = r.read_int64(bytes)?,
                Ok(58) => msg.create_user = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(66) => msg.current_value = Some(r.read_message::<data_object::McpServerValueDo>(bytes)?),
                Ok(74) => msg.release_value = Some(r.read_message::<data_object::McpServerValueDo>(bytes)?),
                Ok(82) => msg.histories.push(r.read_message::<data_object::McpServerValueDo>(bytes)?),
                Ok(90) => msg.unique_key = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for McpServerDo<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.id == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.id) as u64) }
        + if self.namespace == "" { 0 } else { 1 + sizeof_len((&self.namespace).len()) }
        + if self.name == "" { 0 } else { 1 + sizeof_len((&self.name).len()) }
        + if self.description == "" { 0 } else { 1 + sizeof_len((&self.description).len()) }
        + self.auth_keys.iter().map(|s| 1 + sizeof_len((s).len())).sum::<usize>()
        + if self.create_time == 0i64 { 0 } else { 1 + sizeof_varint(*(&self.create_time) as u64) }
        + if self.create_user == "" { 0 } else { 1 + sizeof_len((&self.create_user).len()) }
        + self.current_value.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
        + self.release_value.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
        + self.histories.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + if self.unique_key == "" { 0 } else { 1 + sizeof_len((&self.unique_key).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.id != 0u64 { w.write_with_tag(8, |w| w.write_uint64(*&self.id))?; }
        if self.namespace != "" { w.write_with_tag(18, |w| w.write_string(&**&self.namespace))?; }
        if self.name != "" { w.write_with_tag(26, |w| w.write_string(&**&self.name))?; }
        if self.description != "" { w.write_with_tag(34, |w| w.write_string(&**&self.description))?; }
        for s in &self.auth_keys { w.write_with_tag(42, |w| w.write_string(&**s))?; }
        if self.create_time != 0i64 { w.write_with_tag(48, |w| w.write_int64(*&self.create_time))?; }
        if self.create_user != "" { w.write_with_tag(58, |w| w.write_string(&**&self.create_user))?; }
        if let Some(ref s) = self.current_value { w.write_with_tag(66, |w| w.write_message(s))?; }
        if let Some(ref s) = self.release_value { w.write_with_tag(74, |w| w.write_message(s))?; }
        for s in &self.histories { w.write_with_tag(82, |w| w.write_message(s))?; }
        if self.unique_key != "" { w.write_with_tag(90, |w| w.write_string(&**&self.unique_key))?; }
        Ok(())
    }
}

