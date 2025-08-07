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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ConvertType {
    NONE = 0,
    FORM_TO_JSON = 1,
    CUSTOM = 2,
}

impl Default for ConvertType {
    fn default() -> Self {
        ConvertType::NONE
    }
}

impl From<i32> for ConvertType {
    fn from(i: i32) -> Self {
        match i {
            0 => ConvertType::NONE,
            1 => ConvertType::FORM_TO_JSON,
            2 => ConvertType::CUSTOM,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for ConvertType {
    fn from(s: &'a str) -> Self {
        match s {
            "NONE" => ConvertType::NONE,
            "FORM_TO_JSON" => ConvertType::FORM_TO_JSON,
            "CUSTOM" => ConvertType::CUSTOM,
            _ => Self::default(),
        }
    }
}

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
pub struct ToolSpecVersionDo<'a> {
    pub version: u64,
    pub parameters_json: Cow<'a, str>,
    pub op_user: Cow<'a, str>,
    pub update_time: i64,
    pub ref_count: i64,
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
                Ok(40) => msg.ref_count = r.read_int64(bytes)?,
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
        + if self.ref_count == 0i64 { 0 } else { 1 + sizeof_varint(*(&self.ref_count) as u64) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.version != 0u64 { w.write_with_tag(8, |w| w.write_uint64(*&self.version))?; }
        if self.parameters_json != "" { w.write_with_tag(18, |w| w.write_string(&**&self.parameters_json))?; }
        if self.op_user != "" { w.write_with_tag(26, |w| w.write_string(&**&self.op_user))?; }
        if self.update_time != 0i64 { w.write_with_tag(32, |w| w.write_int64(*&self.update_time))?; }
        if self.ref_count != 0i64 { w.write_with_tag(40, |w| w.write_int64(*&self.ref_count))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct ToolKey<'a> {
    pub namespace: Cow<'a, str>,
    pub group: Cow<'a, str>,
    pub tool_name: Cow<'a, str>,
}

impl<'a> MessageRead<'a> for ToolKey<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.namespace = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(18) => msg.group = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(26) => msg.tool_name = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for ToolKey<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.namespace == "" { 0 } else { 1 + sizeof_len((&self.namespace).len()) }
        + if self.group == "" { 0 } else { 1 + sizeof_len((&self.group).len()) }
        + if self.tool_name == "" { 0 } else { 1 + sizeof_len((&self.tool_name).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.namespace != "" { w.write_with_tag(10, |w| w.write_string(&**&self.namespace))?; }
        if self.group != "" { w.write_with_tag(18, |w| w.write_string(&**&self.group))?; }
        if self.tool_name != "" { w.write_with_tag(26, |w| w.write_string(&**&self.tool_name))?; }
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
pub struct ToolRouteRuleDo<'a> {
    pub protocol: Cow<'a, str>,
    pub url: Cow<'a, str>,
    pub method: Cow<'a, str>,
    pub addition_headers_json: Cow<'a, str>,
    pub convert_type: Cow<'a, str>,
    pub service_namespace: Cow<'a, str>,
    pub service_group: Cow<'a, str>,
    pub service_name: Cow<'a, str>,
}

impl<'a> MessageRead<'a> for ToolRouteRuleDo<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.protocol = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(18) => msg.url = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(26) => msg.method = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(34) => msg.addition_headers_json = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(42) => msg.convert_type = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(50) => msg.service_namespace = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(58) => msg.service_group = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(66) => msg.service_name = r.read_string(bytes).map(Cow::Borrowed)?,
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
        + if self.addition_headers_json == "" { 0 } else { 1 + sizeof_len((&self.addition_headers_json).len()) }
        + if self.convert_type == "" { 0 } else { 1 + sizeof_len((&self.convert_type).len()) }
        + if self.service_namespace == "" { 0 } else { 1 + sizeof_len((&self.service_namespace).len()) }
        + if self.service_group == "" { 0 } else { 1 + sizeof_len((&self.service_group).len()) }
        + if self.service_name == "" { 0 } else { 1 + sizeof_len((&self.service_name).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.protocol != "" { w.write_with_tag(10, |w| w.write_string(&**&self.protocol))?; }
        if self.url != "" { w.write_with_tag(18, |w| w.write_string(&**&self.url))?; }
        if self.method != "" { w.write_with_tag(26, |w| w.write_string(&**&self.method))?; }
        if self.addition_headers_json != "" { w.write_with_tag(34, |w| w.write_string(&**&self.addition_headers_json))?; }
        if self.convert_type != "" { w.write_with_tag(42, |w| w.write_string(&**&self.convert_type))?; }
        if self.service_namespace != "" { w.write_with_tag(50, |w| w.write_string(&**&self.service_namespace))?; }
        if self.service_group != "" { w.write_with_tag(58, |w| w.write_string(&**&self.service_group))?; }
        if self.service_name != "" { w.write_with_tag(66, |w| w.write_string(&**&self.service_name))?; }
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
    pub spec_json: Cow<'a, str>,
    pub route_rule_json: Cow<'a, str>,
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
                Ok(50) => msg.spec_json = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(58) => msg.route_rule_json = r.read_string(bytes).map(Cow::Borrowed)?,
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
        + if self.spec_json == "" { 0 } else { 1 + sizeof_len((&self.spec_json).len()) }
        + if self.route_rule_json == "" { 0 } else { 1 + sizeof_len((&self.route_rule_json).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.id != 0u64 { w.write_with_tag(8, |w| w.write_uint64(*&self.id))?; }
        if self.tool_name != "" { w.write_with_tag(18, |w| w.write_string(&**&self.tool_name))?; }
        if self.namespace != "" { w.write_with_tag(26, |w| w.write_string(&**&self.namespace))?; }
        if self.group != "" { w.write_with_tag(34, |w| w.write_string(&**&self.group))?; }
        if self.version != 0u64 { w.write_with_tag(40, |w| w.write_uint64(*&self.version))?; }
        if self.spec_json != "" { w.write_with_tag(50, |w| w.write_string(&**&self.spec_json))?; }
        if self.route_rule_json != "" { w.write_with_tag(58, |w| w.write_string(&**&self.route_rule_json))?; }
        Ok(())
    }
}

