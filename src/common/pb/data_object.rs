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
pub struct McpServerDo<'a> {
    pub id: u64,
    pub namespace: Cow<'a, str>,
    pub name: Cow<'a, str>,
    pub description: Cow<'a, str>,
    pub token: Cow<'a, str>,
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
                Ok(42) => msg.token = r.read_string(bytes).map(Cow::Borrowed)?,
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
        + if self.token == "" { 0 } else { 1 + sizeof_len((&self.token).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.id != 0u64 { w.write_with_tag(8, |w| w.write_uint64(*&self.id))?; }
        if self.namespace != "" { w.write_with_tag(18, |w| w.write_string(&**&self.namespace))?; }
        if self.name != "" { w.write_with_tag(26, |w| w.write_string(&**&self.name))?; }
        if self.description != "" { w.write_with_tag(34, |w| w.write_string(&**&self.description))?; }
        if self.token != "" { w.write_with_tag(42, |w| w.write_string(&**&self.token))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct McpToolSpecDo<'a> {
    pub namespace: Cow<'a, str>,
    pub group: Cow<'a, str>,
    pub tool_name: Cow<'a, str>,
    pub version: u64,
    pub name: Cow<'a, str>,
    pub description: Cow<'a, str>,
}

impl<'a> MessageRead<'a> for McpToolSpecDo<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.namespace = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(18) => msg.group = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(26) => msg.tool_name = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(32) => msg.version = r.read_uint64(bytes)?,
                Ok(42) => msg.name = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(50) => msg.description = r.read_string(bytes).map(Cow::Borrowed)?,
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
        + if self.version == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.version) as u64) }
        + if self.name == "" { 0 } else { 1 + sizeof_len((&self.name).len()) }
        + if self.description == "" { 0 } else { 1 + sizeof_len((&self.description).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.namespace != "" { w.write_with_tag(10, |w| w.write_string(&**&self.namespace))?; }
        if self.group != "" { w.write_with_tag(18, |w| w.write_string(&**&self.group))?; }
        if self.tool_name != "" { w.write_with_tag(26, |w| w.write_string(&**&self.tool_name))?; }
        if self.version != 0u64 { w.write_with_tag(32, |w| w.write_uint64(*&self.version))?; }
        if self.name != "" { w.write_with_tag(42, |w| w.write_string(&**&self.name))?; }
        if self.description != "" { w.write_with_tag(50, |w| w.write_string(&**&self.description))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct McpToolDo<'a> {
    pub id: u64,
    pub tool_name: Cow<'a, str>,
    pub namespace: Cow<'a, str>,
    pub group: Cow<'a, str>,
    pub version: u64,
}

impl<'a> MessageRead<'a> for McpToolDo<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.id = r.read_uint64(bytes)?,
                Ok(18) => msg.tool_name = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(26) => msg.namespace = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(34) => msg.group = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(40) => msg.version = r.read_uint64(bytes)?,
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
        + if self.id == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.id) as u64) }
        + if self.tool_name == "" { 0 } else { 1 + sizeof_len((&self.tool_name).len()) }
        + if self.namespace == "" { 0 } else { 1 + sizeof_len((&self.namespace).len()) }
        + if self.group == "" { 0 } else { 1 + sizeof_len((&self.group).len()) }
        + if self.version == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.version) as u64) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.id != 0u64 { w.write_with_tag(8, |w| w.write_uint64(*&self.id))?; }
        if self.tool_name != "" { w.write_with_tag(18, |w| w.write_string(&**&self.tool_name))?; }
        if self.namespace != "" { w.write_with_tag(26, |w| w.write_string(&**&self.namespace))?; }
        if self.group != "" { w.write_with_tag(34, |w| w.write_string(&**&self.group))?; }
        if self.version != 0u64 { w.write_with_tag(40, |w| w.write_uint64(*&self.version))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct ToolRouteRuleDo<'a> {
    pub protocol: Cow<'a, str>,
    pub url: Cow<'a, str>,
    pub method: Cow<'a, str>,
}

impl<'a> MessageRead<'a> for ToolRouteRuleDo<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.protocol = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(18) => msg.url = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(26) => msg.method = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for ToolRouteRuleDo<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.protocol == "" { 0 } else { 1 + sizeof_len((&self.protocol).len()) }
        + if self.url == "" { 0 } else { 1 + sizeof_len((&self.url).len()) }
        + if self.method == "" { 0 } else { 1 + sizeof_len((&self.method).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.protocol != "" { w.write_with_tag(10, |w| w.write_string(&**&self.protocol))?; }
        if self.url != "" { w.write_with_tag(18, |w| w.write_string(&**&self.url))?; }
        if self.method != "" { w.write_with_tag(26, |w| w.write_string(&**&self.method))?; }
        Ok(())
    }
}

