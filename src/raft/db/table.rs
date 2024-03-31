use std::collections::BTreeMap;
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};

use async_raft_ext::raft::ClientWriteRequest;
use bean_factory::{bean, Inject};
use serde::{Deserialize, Serialize};

use actix::prelude::*;

use crate::common::constant::CACHE_TREE_NAME;
use crate::common::sequence_utils::SimpleSequence;
use crate::raft::filestore::model::SnapshotRecordDto;
use crate::raft::filestore::raftsnapshot::{SnapshotWriterActor, SnapshotWriterRequest};
use crate::{
    common::string_utils::StringUtils,
    raft::{
        cache::{CacheManager, CacheManagerReq},
        cluster::model::RouterRequest,
        store::ClientRequest,
        NacosRaft,
    },
};

type TableKV = (Vec<u8>, Vec<u8>);

#[derive(Clone, prost::Message, Serialize, Deserialize)]
pub struct TableDefinition {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(uint32, tag = "2")]
    pub sequence_step: u32, // 0: None
}

impl TableDefinition {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::new();
        prost::Message::encode(self, &mut v).unwrap();
        v
    }

    pub fn from_bytes(v: &[u8]) -> anyhow::Result<Self> {
        Ok(prost::Message::decode(v)?)
    }
}

//pub(crate) const TABLE_DEFINITION_TREE_NAME: &str = "tables";

pub struct TableInfo {
    pub table_data: BTreeMap<Vec<u8>, Vec<u8>>,
    pub name: Arc<String>,
    //pub table_db_name: Arc<String>,
    pub seq: Option<SimpleSequence>,
}

impl TableInfo {
    pub fn new(name: Arc<String>, sequence_step: u32) -> Self {
        //let table_name = Arc::new(format!("t_{}", &name));
        let seq = if sequence_step == 0 {
            None
        } else {
            Some(SimpleSequence::new(0, sequence_step as u64))
        };
        Self {
            table_data: Default::default(),
            name,
            //table_db_name: table_name,
            seq,
        }
    }
}

#[bean(inject)]
#[derive(Default)]
pub struct TableManager {
    //pub db: Arc<sled::Db>,
    pub table_map: HashMap<Arc<String>, TableInfo>,
    raft: Option<Weak<NacosRaft>>,
    cache_manager: Option<Addr<CacheManager>>,
}

impl TableManager {
    pub fn new() -> Self {
        Self::default()
    }

    fn init_table(&mut self, _name: Arc<String>, _sequence_step: u32) {}

    pub fn drop_table(&mut self, name: &Arc<String>) {
        if let Some(mut table) = self.table_map.remove(name) {
            if let Some(seq) = table.seq.as_mut() {
                seq.set_last_id(0);
            }
        }
    }

    pub fn next_id(&mut self, name: Arc<String>, seq_step: u32) -> anyhow::Result<u64> {
        if let Some(table_info) = self.table_map.get_mut(&name) {
            if let Some(seq) = table_info.seq.as_mut() {
                Ok(seq.next_id())
            } else {
                Err(anyhow::anyhow!("the table {} seq is none", &name))
            }
        } else {
            self.init_table(name.clone(), seq_step);
            let mut table_info = TableInfo::new(name.clone(), 1);
            let r = table_info.seq.as_mut().unwrap().next_id();
            self.table_map.insert(name, table_info);
            Ok(r)
        }
    }

    pub fn set_last_seq_id(&mut self, name: Arc<String>, last_seq_id: u64) {
        if let Some(table_info) = self.table_map.get_mut(&name) {
            if let Some(seq) = table_info.seq.as_mut() {
                seq.set_last_id(last_seq_id);
            }
        }
    }

    pub fn insert(
        &mut self,
        name: Arc<String>,
        key: Vec<u8>,
        value: Vec<u8>,
        last_seq_id: Option<u64>,
    ) -> Option<Vec<u8>> {
        if let Some(table_info) = self.table_map.get_mut(&name) {
            if let (Some(seq), Some(last_seq_id)) = (table_info.seq.as_mut(), last_seq_id) {
                seq.set_last_id(last_seq_id);
            }
            table_info.table_data.insert(key, value)
        } else {
            self.init_table(name.clone(), 0);
            let mut table_info = TableInfo::new(name.clone(), 1);
            table_info.table_data.insert(key, value);
            if let (Some(seq), Some(last_seq_id)) = (table_info.seq.as_mut(), last_seq_id) {
                seq.set_last_id(last_seq_id);
            }
            self.table_map.insert(name, table_info);
            None
        }
    }

    pub fn remove(&mut self, name: Arc<String>, key: Vec<u8>) -> Option<Vec<u8>> {
        if let Some(table_info) = self.table_map.get_mut(&name) {
            table_info.table_data.remove(&key)
        } else {
            None
        }
    }

    pub fn get(&mut self, name: Arc<String>, key: Vec<u8>) -> Option<Vec<u8>> {
        if let Some(table_info) = self.table_map.get(&name) {
            table_info.table_data.get(&key).cloned()
        } else {
            None
        }
    }

    pub(crate) fn query_page_list(
        &self,
        name: Arc<String>,
        like_key: Option<String>,
        offset: Option<i64>,
        limit: Option<i64>,
        is_rev: bool,
    ) -> (usize, Vec<TableKV>) {
        if let Some(table_info) = self.table_map.get(&name) {
            let total = self.query_list_count(&name, &like_key);
            let mut ret = vec![];
            if is_rev {
                let offset = offset.unwrap_or_default();
                let iter = table_info.table_data.iter().rev().skip(offset as usize);
                if let Some(limit) = limit {
                    let t = iter.take(limit as usize);
                    for (k, v) in t {
                        Self::push_match_condition_item(k, v, &like_key, &mut ret);
                    }
                } else {
                    for (k, v) in iter {
                        Self::push_match_condition_item(k, v, &like_key, &mut ret);
                    }
                }
            } else {
                //正反 iter 类型不同，后继可以考虑使用宏消除下面的重复编码
                let offset = offset.unwrap_or_default();
                let iter = table_info.table_data.iter().skip(offset as usize);
                if let Some(limit) = limit {
                    let t = iter.take(limit as usize);
                    for (k, v) in t {
                        Self::push_match_condition_item(k, v, &like_key, &mut ret);
                    }
                } else {
                    for (k, v) in iter {
                        Self::push_match_condition_item(k, v, &like_key, &mut ret);
                    }
                }
            }
            (total, ret)
        } else {
            (0, vec![])
        }
    }

    pub(crate) fn query_list_count(&self, name: &Arc<String>, like_key: &Option<String>) -> usize {
        if let Some(table_info) = self.table_map.get(name) {
            if let Some(like_key) = like_key {
                let mut count = 0;
                for (k, _) in table_info.table_data.iter() {
                    let key_str = String::from_utf8_lossy(k);
                    if StringUtils::like(&key_str, like_key.as_str()).is_some() {
                        count += 1;
                    }
                }
                count
            } else {
                table_info.table_data.len()
            }
        } else {
            0
        }
    }

    async fn send_raft_request(
        raft: &Option<Weak<NacosRaft>>,
        req: ClientRequest,
    ) -> anyhow::Result<()> {
        if let Some(weak_raft) = raft {
            if let Some(raft) = weak_raft.upgrade() {
                raft.client_write(ClientWriteRequest::new(req)).await?;
            }
        }
        Ok(())
    }

    fn push_match_condition_item(
        k: &[u8],
        v: &[u8],
        like_key: &Option<String>,
        ret: &mut Vec<(Vec<u8>, Vec<u8>)>,
    ) {
        let key_str = String::from_utf8_lossy(k);
        if let Some(like_key) = like_key.as_ref() {
            if StringUtils::like(&key_str, like_key.as_str()).is_none() {
                return;
            }
        }
        ret.push((k.to_vec(), v.to_vec()));
    }

    fn get_table_names(&self) -> Vec<Arc<String>> {
        self.table_map.values().map(|e| e.name.clone()).collect()
    }

    ///
    /// 将数据写入raft snapshot文件中
    ///
    fn build_snapshot(&self, writer: Addr<SnapshotWriterActor>) -> anyhow::Result<()> {
        for table_info in self.table_map.values() {
            for (key, value) in &table_info.table_data {
                let record = SnapshotRecordDto {
                    tree: table_info.name.clone(),
                    key: key.to_owned(),
                    value: value.to_owned(),
                    op_type: 0,
                };
                writer.do_send(SnapshotWriterRequest::Record(record));
            }
        }
        Ok(())
    }
}

impl Actor for TableManager {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("TableManage actor started")
    }
}

impl Inject for TableManager {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        factory_data: bean_factory::FactoryData,
        _factory: bean_factory::BeanFactory,
        _ctx: &mut Self::Context,
    ) {
        let raft: Option<Arc<NacosRaft>> = factory_data.get_bean();
        self.raft = raft.map(|e| Arc::downgrade(&e));
        self.cache_manager = factory_data.get_actor();
    }
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<TableManagerResult>")]
pub struct TableManagerAsyncReq(pub TableManagerReq);

#[derive(Message, Clone, Debug, Serialize, Deserialize)]
#[rtype(result = "anyhow::Result<TableManagerResult>")]
pub enum TableManagerReq {
    Set {
        table_name: Arc<String>,
        key: Vec<u8>,
        value: Vec<u8>,
        last_seq_id: Option<u64>,
    },
    SetUseAutoId {
        table_name: Arc<String>,
        value: Vec<u8>,
    },
    Remove {
        table_name: Arc<String>,
        key: Vec<u8>,
    },
    Drop(Arc<String>),
    NextId {
        table_name: Arc<String>,
        seq_step: Option<u32>,
    },
    SetSeqId {
        table_name: Arc<String>,
        last_seq_id: u64,
    },
    ReloadTable,
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<TableManagerResult>")]
pub enum TableManagerInnerReq {
    BuildSnapshot(Addr<SnapshotWriterActor>),
}

impl From<TableManagerReq> for RouterRequest {
    fn from(req: TableManagerReq) -> Self {
        Self::TableManagerReq { req }
    }
}

#[derive(Message, Clone, Debug, Serialize, Deserialize)]
#[rtype(result = "anyhow::Result<TableManagerResult>")]
pub enum TableManagerQueryReq {
    QueryTableNames,
    Get {
        table_name: Arc<String>,
        key: String,
    },
    GetByArcKey {
        table_name: Arc<String>,
        key: Arc<String>,
    },
    GetByBytes {
        table_name: Arc<String>,
        key: Vec<u8>,
    },
    QueryPageList {
        table_name: Arc<String>,
        like_key: Option<String>,
        offset: Option<i64>,
        limit: Option<i64>,
        is_rev: bool,
    },
}

impl From<TableManagerQueryReq> for RouterRequest {
    fn from(req: TableManagerQueryReq) -> Self {
        Self::TableManagerQueryReq { req }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TableManagerResult {
    None,
    Value(Vec<u8>),
    NextId(u64),
    TableNames(Vec<Arc<String>>),
    PageListResult(usize, Vec<(Vec<u8>, Vec<u8>)>),
}

impl Handler<TableManagerAsyncReq> for TableManager {
    type Result = ResponseActFuture<Self, anyhow::Result<TableManagerResult>>;

    fn handle(&mut self, msg: TableManagerAsyncReq, _ctx: &mut Self::Context) -> Self::Result {
        let req = msg.0;
        let raft = self.raft.clone();

        let fut = async move {
            let _ = Self::send_raft_request(&raft, ClientRequest::TableManagerReq(req)).await;
            Ok(TableManagerResult::None)
        }
        .into_actor(self)
        .map(|r, _act, _ctx| r);
        Box::pin(fut)
    }
}

impl Handler<TableManagerReq> for TableManager {
    type Result = anyhow::Result<TableManagerResult>;

    fn handle(&mut self, msg: TableManagerReq, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            TableManagerReq::Set {
                table_name,
                key,
                value,
                last_seq_id,
            } => {
                if table_name.as_str() == CACHE_TREE_NAME.as_str() {
                    if let Some(cache_manager) = &self.cache_manager {
                        let req = CacheManagerReq::NotifyChange {
                            key: key.clone(),
                            value: value.clone(),
                        };
                        cache_manager.do_send(req);
                    }
                }
                self.insert(table_name, key, value, last_seq_id);
                Ok(TableManagerResult::None)
            }
            TableManagerReq::Remove { table_name, key } => {
                if table_name.as_str() == CACHE_TREE_NAME.as_str() {
                    if let Some(cache_manager) = &self.cache_manager {
                        let req = CacheManagerReq::NotifyRemove { key: key.clone() };
                        cache_manager.do_send(req);
                    }
                }
                match self.remove(table_name, key) {
                    Some(v) => Ok(TableManagerResult::Value(v.to_vec())),
                    None => Ok(TableManagerResult::None),
                }
            }
            TableManagerReq::Drop(name) => {
                self.drop_table(&name);
                Ok(TableManagerResult::None)
            }
            TableManagerReq::NextId {
                table_name,
                seq_step,
            } => match self.next_id(table_name, seq_step.unwrap_or(100)) {
                Ok(v) => Ok(TableManagerResult::NextId(v)),
                Err(_) => Ok(TableManagerResult::None),
            },
            TableManagerReq::SetSeqId {
                table_name,
                last_seq_id,
            } => {
                self.set_last_seq_id(table_name, last_seq_id);
                Ok(TableManagerResult::None)
            }
            TableManagerReq::SetUseAutoId {
                table_name: _,
                value: _,
            } => Err(anyhow::anyhow!(
                "must pre transform to TableManagerReq::Set"
            )),
            TableManagerReq::ReloadTable => {
                //self.load_tables();
                Ok(TableManagerResult::None)
            }
        }
    }
}

impl Handler<TableManagerInnerReq> for TableManager {
    type Result = anyhow::Result<TableManagerResult>;

    fn handle(&mut self, msg: TableManagerInnerReq, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            TableManagerInnerReq::BuildSnapshot(writer) => {
                self.build_snapshot(writer).ok();
                Ok(TableManagerResult::None)
            }
        }
    }
}

impl Handler<TableManagerQueryReq> for TableManager {
    type Result = anyhow::Result<TableManagerResult>;

    fn handle(&mut self, msg: TableManagerQueryReq, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            TableManagerQueryReq::QueryTableNames => {
                let table_names = self.get_table_names();
                Ok(TableManagerResult::TableNames(table_names))
            }
            TableManagerQueryReq::Get { table_name, key } => {
                match self.get(table_name, key.as_bytes().to_vec()) {
                    Some(v) => Ok(TableManagerResult::Value(v.to_vec())),
                    None => Ok(TableManagerResult::None),
                }
            }
            TableManagerQueryReq::GetByArcKey { table_name, key } => {
                match self.get(table_name, key.as_bytes().to_vec()) {
                    Some(v) => Ok(TableManagerResult::Value(v.to_vec())),
                    None => Ok(TableManagerResult::None),
                }
            }
            TableManagerQueryReq::GetByBytes { table_name, key } => match self.get(table_name, key)
            {
                Some(v) => Ok(TableManagerResult::Value(v.to_vec())),
                None => Ok(TableManagerResult::None),
            },
            TableManagerQueryReq::QueryPageList {
                table_name,
                like_key,
                offset,
                limit,
                is_rev,
            } => {
                let (size, list) =
                    self.query_page_list(table_name, like_key, offset, limit, is_rev);
                Ok(TableManagerResult::PageListResult(size, list))
            }
        }
    }
}
