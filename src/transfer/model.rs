use crate::common::pb::transfer::{TableNameMapEntity, TransferHeader, TransferItem};
use crate::common::tempfile::TempFile;
///导入导出中间文件对象
use crate::common::{constant, get_app_version};
use crate::now_millis;
use crate::transfer::writer::TransferWriterActor;
use actix::{Addr, Message};
use binrw_derive::binrw;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[binrw]
#[derive(Debug)]
pub struct TransferPrefix {
    ///魔法值 0x6e61636f
    pub magic: u32,
    ///版本号
    pub fmt_version: u32,
}

impl TransferPrefix {
    pub fn new() -> Self {
        Self {
            magic: 0x6e61636f,
            fmt_version: 0,
        }
    }
}

impl Default for TransferPrefix {
    fn default() -> Self {
        Self::new()
    }
}

///中间文件-头对象
#[derive(Debug, Clone)]
pub struct TransferHeaderDto {
    pub version: u64,
    pub modify_time: u64,
    pub from_sys: Option<String>,
    pub name_to_id: HashMap<Arc<String>, u32>,
    pub id_to_name: HashMap<u32, Arc<String>>,
    pub max_id: u32,
    pub extend_info: HashMap<String, String>,
}

impl TransferHeaderDto {
    pub fn new(version: u64) -> Self {
        Self {
            version,
            modify_time: now_millis(),
            from_sys: Some(format!("r-nacos_{}", get_app_version())),
            name_to_id: Default::default(),
            id_to_name: Default::default(),
            max_id: 0,
            extend_info: Default::default(),
        }
    }

    pub fn to_do(&self) -> TransferHeader<'_> {
        self.into()
    }

    pub fn add_name(&mut self, name: Arc<String>) {
        if self.name_to_id.contains_key(&name) {
            return;
        }
        self.max_id += 1;
        let new_id = self.max_id;
        self.id_to_name.insert(new_id, name.clone());
        self.name_to_id.insert(name, new_id);
    }
}

impl<'a> From<TransferHeader<'a>> for TransferHeaderDto {
    fn from(value: TransferHeader<'a>) -> Self {
        let mut name_to_id = HashMap::new();
        let mut id_to_name = HashMap::new();
        let mut max_id = 0;
        for item in &value.table_name_map_entities {
            let id = item.id;
            let name = Arc::new(item.name.as_ref().to_owned());
            id_to_name.insert(id, name.clone());
            name_to_id.insert(name, id);
            if id > max_id {
                max_id = id;
            }
        }
        TransferHeaderDto {
            version: value.version,
            modify_time: value.modify_time,
            from_sys: Some(value.from_sys.as_ref().to_owned()),
            name_to_id,
            id_to_name,
            max_id,
            extend_info: serde_json::from_slice(value.extend.as_ref()).unwrap_or_default(),
        }
    }
}

impl<'a> From<&'a TransferHeaderDto> for TransferHeader<'a> {
    fn from(value: &'a TransferHeaderDto) -> Self {
        let from_sys = if let Some(v) = value.from_sys.as_ref() {
            v
        } else {
            constant::EMPTY_STR
        };
        let mut table_name_map_entities = Vec::with_capacity(value.id_to_name.len());
        for (id, name) in &value.id_to_name {
            let entity = TableNameMapEntity {
                id: *id,
                name: Cow::Borrowed(name),
            };
            table_name_map_entities.push(entity);
        }
        Self {
            version: value.version,
            modify_time: value.modify_time,
            from_sys: Cow::Borrowed(from_sys),
            table_name_map_entities,
            extend: Cow::Owned(serde_json::to_vec(&value.extend_info).unwrap_or_default()),
        }
    }
}

///中间文件-数据项
#[derive(Debug, Clone, Default)]
pub struct TransferRecordDto {
    pub table_name: Option<Arc<String>>,
    pub table_id: u32,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

impl TransferRecordDto {
    pub fn to_do(&self) -> TransferItem<'_> {
        self.into()
    }
}

impl<'a> From<&'a TransferRecordDto> for TransferItem<'a> {
    fn from(value: &'a TransferRecordDto) -> Self {
        let table_name = if let Some(v) = value.table_name.as_ref() {
            v
        } else {
            constant::EMPTY_STR
        };
        Self {
            table_name: Cow::Borrowed(table_name),
            table_id: value.table_id,
            key: Cow::Borrowed(&value.key),
            value: Cow::Borrowed(&value.value),
        }
    }
}

impl<'a> From<TransferItem<'a>> for TransferRecordDto {
    fn from(value: TransferItem<'a>) -> Self {
        let table_name = if value.table_name.as_ref().is_empty() {
            None
        } else {
            Some(Arc::new(value.table_name.as_ref().to_owned()))
        };
        Self {
            table_name,
            table_id: value.table_id,
            key: value.key.to_vec(),
            value: value.value.to_vec(),
        }
    }
}

pub struct TransferRecordRef<'a> {
    pub table_name: Arc<String>,
    pub key: Cow<'a, [u8]>,
    pub value: Cow<'a, [u8]>,
}

impl<'a> TransferRecordRef<'a> {
    pub fn new(table_name: Arc<String>, item: TransferItem<'a>) -> Self {
        Self {
            table_name,
            key: item.key,
            value: item.value,
        }
    }
}

#[derive(Message, Debug)]
#[rtype(result = "anyhow::Result<TransferWriterResponse>")]
pub enum TransferWriterRequest {
    AddTableNameMap(Arc<String>),
    InitHeader,
    AddRecord(TransferRecordDto),
}

#[derive(Message, Debug)]
#[rtype(result = "anyhow::Result<TransferWriterResponse>")]
pub enum TransferWriterAsyncRequest {
    Flush,
}

pub enum TransferWriterResponse {
    Path(PathBuf),
    None,
}

#[derive(Debug, Clone)]
pub struct TransferBackupParam {
    pub config: bool,
    pub user: bool,
    pub cache: bool,
    pub mcp: bool,
}

impl TransferBackupParam {
    pub fn all() -> Self {
        Self {
            config: true,
            user: true,
            cache: true,
            mcp: true,
        }
    }
}

#[derive(Message, Debug)]
#[rtype(result = "anyhow::Result<TransferManagerResponse>")]
pub enum TransferManagerAsyncRequest {
    Backup(TransferBackupParam),
}

pub enum TransferManagerResponse {
    BackupFile(TempFile),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferImportParam {
    pub config: bool,
    pub user: bool,
    pub cache: bool,
    pub mcp: bool,
}

impl TransferImportParam {
    pub fn all() -> Self {
        Self {
            config: true,
            user: true,
            cache: true,
            mcp: true,
        }
    }
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<TransferImportResponse>")]
pub enum TransferImportRequest {
    Import(Vec<u8>, TransferImportParam),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransferImportResponse {
    None,
    Running,
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<TransferDataResponse>")]
pub enum TransferDataRequest {
    Backup(Addr<TransferWriterActor>, TransferBackupParam),
}

pub enum TransferDataResponse {
    None,
}
