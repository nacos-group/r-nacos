use async_raft_ext::raft::ClientWriteRequest;
use bean_factory::bean;
use bean_factory::Inject;
use chrono::Local;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::sync::Weak;
use std::time::Duration;

use crate::raft::store::ClientRequest;
use crate::raft::NacosRaft;
use crate::utils::get_md5;
use serde::{Deserialize, Serialize};

use crate::common::byte_utils::id_to_bin;
use crate::common::constant::{CONFIG_TREE_NAME, SEQUENCE_TREE_NAME, SEQ_KEY_CONFIG};
use crate::common::sequence_utils::SimpleSequence;
use actix::prelude::*;

use super::config_subscribe::Subscriber;
use super::dal::ConfigHistoryParam;
use crate::config::config_index::{ConfigQueryParam, TenantIndex};
use crate::config::config_type::ConfigType;
use crate::config::model::{
    ConfigRaftCmd, ConfigRaftResult, ConfigValueDO, HistoryItem, SetConfigParam,
};
use crate::config::utils::param_utils;
use crate::namespace::NamespaceActor;
use crate::now_millis_i64;
use crate::raft::filestore::model::SnapshotRecordDto;
use crate::raft::filestore::raftsnapshot::{SnapshotWriterActor, SnapshotWriterRequest};
use crate::transfer::model::{
    TransferDataRequest, TransferDataResponse, TransferRecordDto, TransferWriterRequest,
};
use crate::transfer::writer::TransferWriterActor;

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct ConfigKey {
    pub(crate) data_id: Arc<String>,
    pub(crate) group: Arc<String>,
    pub(crate) tenant: Arc<String>,
}

impl ConfigKey {
    pub fn new(data_id: &str, group: &str, tenant: &str) -> ConfigKey {
        ConfigKey {
            data_id: Arc::new(data_id.to_owned()),
            group: Arc::new(group.to_owned()),
            tenant: Arc::new(tenant.to_owned()),
        }
    }

    pub fn new_by_arc(data_id: Arc<String>, group: Arc<String>, tenant: Arc<String>) -> ConfigKey {
        ConfigKey {
            data_id,
            group,
            tenant,
        }
    }

    pub fn build_key(&self) -> String {
        if self.tenant.is_empty() {
            return format!("{}\x02{}", self.data_id, self.group);
        }
        format!("{}\x02{}\x02{}", self.data_id, self.group, self.tenant)
    }

    ///
    /// 是否合法的key
    /// 暂时只用于接口层面判断,以支持对部分场景不校验
    ///
    pub fn is_valid(&self) -> anyhow::Result<()> {
        if !param_utils::is_valid(self.data_id.as_str()) {
            return Err(anyhow::anyhow!(
                "the config data_id is invalid : {}",
                self.data_id.as_str()
            ));
        }
        if !param_utils::is_valid(self.group.as_str()) {
            return Err(anyhow::anyhow!(
                "the config group is invalid : {}",
                self.group.as_str()
            ));
        }
        Ok(())
    }
}

impl From<&str> for ConfigKey {
    fn from(value: &str) -> Self {
        let mut list = value.split('\x02');
        let data_id = list.next();
        let group = list.next();
        let tenant = list.next();
        ConfigKey::new(
            data_id.unwrap_or(""),
            group.unwrap_or(""),
            tenant.unwrap_or(""),
        )
    }
}

// impl PartialEq for ConfigKey {
//     fn eq(&self, o: &Self) -> bool {
//         self.data_id == o.data_id && self.group == o.group && self.tenant == o.tenant
//     }
// }

#[derive(Clone)]
pub struct ConfigValue {
    pub(crate) content: Arc<String>,
    pub(crate) md5: Arc<String>,
    pub(crate) tmp: bool,
    pub(crate) histories: Vec<HistoryItem>,
    pub(crate) config_type: Option<Arc<String>>,
    pub(crate) desc: Option<Arc<String>>,
    pub(crate) last_modified: i64,
}

impl ConfigValue {
    pub fn new(content: Arc<String>) -> Self {
        let md5 = get_md5(&content);
        Self {
            content,
            md5: Arc::new(md5),
            tmp: false,
            histories: vec![],
            config_type: None,
            desc: None,
            last_modified: now_millis_i64(),
        }
    }

    pub fn init(
        content: Arc<String>,
        history_id: u64,
        op_time: i64,
        md5: Option<Arc<String>>,
        op_user: Option<Arc<String>>,
    ) -> Self {
        let md5 = if let Some(v) = md5 {
            v
        } else {
            Arc::new(get_md5(&content))
        };
        Self {
            content: content.clone(),
            md5,
            tmp: false,
            histories: vec![HistoryItem {
                id: history_id,
                content,
                modified_time: op_time,
                op_user,
            }],
            config_type: None,
            desc: None,
            last_modified: op_time,
        }
    }

    pub fn update_value(
        &mut self,
        content: Arc<String>,
        history_id: u64,
        op_time: i64,
        md5: Option<Arc<String>>,
        op_user: Option<Arc<String>>,
    ) {
        let md5 = if let Some(v) = md5 {
            v
        } else {
            Arc::new(get_md5(&content))
        };
        self.md5 = md5;
        self.content = content.clone();
        self.tmp = false;
        let item = HistoryItem {
            id: history_id,
            content,
            modified_time: op_time,
            op_user,
        };
        if self.histories.len() >= 100 {
            self.histories.remove(0);
        }
        self.last_modified = op_time;
        self.histories.push(item);
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigInfoDto {
    pub tenant: Arc<String>,
    pub group: Arc<String>,
    pub data_id: Arc<String>,
    pub content: Option<Arc<String>>,
    pub md5: Option<Arc<String>>,
    pub desc: Option<Arc<String>>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConfigHistoryInfoDto {
    pub id: Option<i64>,
    pub tenant: Option<String>,
    pub group: Option<String>,
    pub data_id: Option<String>,
    pub content: Option<String>,
    pub modified_time: Option<i64>, //给历史记录使用
    pub op_user: Option<String>,
}

#[derive(Debug)]
pub struct ListenerItem {
    pub key: ConfigKey,
    pub md5: Arc<String>,
}

impl ListenerItem {
    pub fn new(key: ConfigKey, md5: Arc<String>) -> Self {
        Self { key, md5 }
    }

    pub fn decode_listener_items(configs: &str) -> Vec<Self> {
        let mut list = vec![];
        let mut start = 0;
        let bytes = configs.as_bytes();
        let mut tmp_list = vec![];
        for i in 0..bytes.len() {
            let char = bytes[i];
            if char == 2 {
                if tmp_list.len() > 2 {
                    continue;
                }
                tmp_list.push(String::from_utf8(bytes[start..i].to_vec()).unwrap());
                start = i + 1;
            } else if char == 1 {
                let mut end_value = String::new();
                if start < i {
                    end_value = String::from_utf8(bytes[start..i].to_vec()).unwrap();
                }
                start = i + 1;
                if tmp_list.len() == 2 {
                    let key = ConfigKey::new(&tmp_list[0], &tmp_list[1], "");
                    list.push(ListenerItem::new(key, Arc::new(end_value)));
                } else {
                    if end_value == "public" {
                        "".clone_into(&mut end_value);
                    }
                    let key = ConfigKey::new(&tmp_list[0], &tmp_list[1], &end_value);
                    list.push(ListenerItem::new(key, Arc::new(tmp_list[2].to_owned())));
                }
                tmp_list.clear();
            }
        }
        list
    }

    pub fn decode_listener_change_keys(configs: &str) -> Vec<ConfigKey> {
        let mut list = vec![];
        let mut start = 0;
        let bytes = configs.as_bytes();
        let mut tmp_list = vec![];
        for i in 0..bytes.len() {
            let char = bytes[i];
            if char == 2 {
                if tmp_list.len() > 2 {
                    continue;
                }
                tmp_list.push(String::from_utf8(bytes[start..i].to_vec()).unwrap());
                start = i + 1;
            } else if char == 1 {
                let mut end_value = String::new();
                if start < i {
                    end_value = String::from_utf8(bytes[start..i].to_vec()).unwrap();
                }
                start = i + 1;
                if tmp_list.len() == 1 {
                    let key = ConfigKey::new(&tmp_list[0], &end_value, "");
                    list.push(key);
                } else {
                    let key = ConfigKey::new(&tmp_list[0], &tmp_list[1], &end_value);
                    list.push(key);
                }
                tmp_list.clear();
            }
        }
        list
    }
}

struct OnceListener {
    version: u64,
    //time: i64,
    //list: Vec<ListenerItem>,
}

pub enum ListenerResult {
    NULL,
    DATA(Vec<ConfigKey>),
}

type ListenerSenderType = tokio::sync::oneshot::Sender<ListenerResult>;
//type ListenerReceiverType = tokio::sync::oneshot::Receiver<ListenerResult>;

pub(crate) struct ConfigListener {
    version: u64,
    listener: HashMap<ConfigKey, Vec<u64>>,
    time_listener: BTreeMap<i64, Vec<OnceListener>>,
    sender_map: HashMap<u64, ListenerSenderType>,
}

impl ConfigListener {
    fn new() -> Self {
        Self {
            version: 0,
            listener: Default::default(),
            time_listener: Default::default(),
            sender_map: Default::default(),
        }
    }

    fn add(&mut self, items: Vec<ListenerItem>, sender: ListenerSenderType, time: i64) {
        self.version += 1;
        for item in &items {
            let key = item.key.clone();
            match self.listener.get_mut(&key) {
                Some(list) => {
                    list.push(self.version);
                }
                None => {
                    self.listener.insert(key, vec![self.version]);
                }
            };
        }
        self.sender_map.insert(self.version, sender);
        let once_listener = OnceListener {
            version: self.version,
            //time,
            //list: items,
        };
        match self.time_listener.get_mut(&time) {
            Some(list) => {
                list.push(once_listener);
            }
            None => {
                self.time_listener.insert(time, vec![once_listener]);
            }
        }
    }

    fn notify(&mut self, key: ConfigKey) {
        if let Some(list) = self.listener.remove(&key) {
            for v in list {
                if let Some(sender) = self.sender_map.remove(&v) {
                    sender.send(ListenerResult::DATA(vec![key.clone()])).ok();
                }
            }
        }
    }

    fn timeout(&mut self) {
        let current_time = Local::now().timestamp_millis();
        let mut keys: Vec<i64> = Vec::new();
        for (key, list) in self.time_listener.iter().take(10000) {
            if *key < current_time {
                keys.push(*key);
                for item in list {
                    let v = item.version;
                    if let Some(sender) = self.sender_map.remove(&v) {
                        sender.send(ListenerResult::NULL).ok();
                    }
                }
            } else {
                break;
            }
        }
        for key in keys {
            self.time_listener.remove(&key);
        }
    }

    pub(crate) fn get_listener_client_size(&self) -> usize {
        self.sender_map.len()
    }

    pub(crate) fn get_listener_key_size(&self) -> usize {
        self.listener.len()
    }
}

#[bean(inject)]
pub struct ConfigActor {
    pub(crate) cache: HashMap<ConfigKey, ConfigValue>,
    pub(crate) listener: ConfigListener,
    pub(crate) subscriber: Subscriber,
    pub(crate) tenant_index: TenantIndex,
    raft: Option<Weak<NacosRaft>>,
    namespace_actor: Option<Addr<NamespaceActor>>,
    sequence: SimpleSequence,
}

impl Inject for ConfigActor {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        factory_data: bean_factory::FactoryData,
        _factory: bean_factory::BeanFactory,
        _ctx: &mut Self::Context,
    ) {
        let raft: Option<Arc<NacosRaft>> = factory_data.get_bean();
        self.raft = raft.map(|e| Arc::downgrade(&e));
        self.namespace_actor = factory_data.get_actor();
        self.tenant_index.namespace_actor = self.namespace_actor.clone();
        if let Some(conn_manage) = factory_data.get_actor() {
            self.subscriber.set_conn_manage(conn_manage);
        }
        log::info!("ConfigActor inject complete");
    }
}

impl Default for ConfigActor {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigActor {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            subscriber: Subscriber::new(),
            listener: ConfigListener::new(),
            tenant_index: TenantIndex::new(),
            raft: None,
            namespace_actor: None,
            sequence: SimpleSequence::new(0, 100),
        }
    }

    fn set_tmp_config(&mut self, key: ConfigKey, val: Arc<String>) {
        if let Some(v) = self.cache.get_mut(&key) {
            v.tmp = true;
            v.md5 = Arc::new(get_md5(&val));
            v.content = val;
        } else {
            let mut config_val = ConfigValue::new(val);
            config_val.tmp = true;
            self.cache.insert(key, config_val);
        }
    }

    fn inner_set_config(&mut self, key: ConfigKey, value: ConfigValue) {
        self.tenant_index.insert_config(key.clone());
        self.cache.insert(key, value);
    }

    fn set_config(&mut self, param: SetConfigParam) -> anyhow::Result<ConfigResult> {
        if let Some(history_table_id) = param.history_table_id {
            self.sequence.set_valid_last_id(history_table_id);
        }
        if let Some(v) = self.cache.get_mut(&param.key) {
            let md5 = get_md5(param.value.as_str());
            if let Some(s) = param.config_type {
                v.config_type = Some(s);
            }
            if let Some(s) = param.desc {
                v.desc = Some(s);
            }
            if !v.tmp && v.md5.as_str() == md5 {
                return Ok(ConfigResult::NULL);
            }
            if v.histories.is_empty() {
                self.tenant_index.insert_config(param.key.clone());
            }
            v.update_value(
                param.value,
                param.history_id,
                param.op_time,
                Some(Arc::new(md5)),
                param.op_user,
            );
        } else {
            let mut v = ConfigValue::init(
                param.value,
                param.history_id,
                param.op_time,
                None,
                param.op_user,
            );
            v.config_type = param.config_type;
            v.desc = param.desc;
            self.cache.insert(param.key.clone(), v);
            self.tenant_index.insert_config(param.key.clone());
        }
        self.listener.notify(param.key.clone());
        self.subscriber.notify(param.key);
        Ok(ConfigResult::NULL)
    }

    fn del_config(&mut self, key: ConfigKey) -> anyhow::Result<()> {
        self.cache.remove(&key);
        //self.config_db.del_config(&key).ok();
        self.tenant_index.remove_config(&key);
        self.listener.notify(key.clone());
        self.subscriber.notify(key.clone());
        self.subscriber.remove_config_key(key);
        Ok(())
    }

    /*
    fn load_config(&mut self) {
        for item in self.config_db.query_config_list().unwrap() {
            let key = ConfigKey::new(
                item.data_id.as_ref(),
                item.group.as_ref(),
                item.tenant.as_ref(),
            );
            let val = ConfigValue::new(Arc::new(item.content.unwrap_or_default()));
            self.tenant_index.insert_config(key.clone());
            self.cache.insert(key, val);
        }
        self.config_db.init_seq();
    }
     */

    async fn send_raft_request(
        raft: &Option<Weak<NacosRaft>>,
        req: ClientRequest,
    ) -> anyhow::Result<()> {
        if let Some(weak_raft) = raft {
            if let Some(raft) = weak_raft.upgrade() {
                //TODO换成feature,非wait的方式
                raft.client_write(ClientWriteRequest::new(req)).await?;
            }
        }
        Ok(())
    }

    pub fn get_config_info_page(&self, param: &ConfigQueryParam) -> (usize, Vec<ConfigInfoDto>) {
        let (size, list) = self.tenant_index.query_config_page(param);

        if size == 0 {
            return (size, Vec::new());
        }

        let mut info_list = Vec::with_capacity(size);
        for item in &list {
            if let Some(value) = self.cache.get(item) {
                let mut info = ConfigInfoDto {
                    tenant: item.tenant.clone(),
                    group: item.group.clone(),
                    data_id: item.data_id.clone(),
                    desc: value.desc.clone(),
                    //md5:Some(value.md5.clone()),
                    //content:Some(value.content.clone()),
                    ..Default::default()
                };
                if param.query_context {
                    info.content = Some(value.content.clone());
                    info.md5 = Some(value.md5.clone());
                }
                info_list.push(info);
            }
        }
        (size, info_list)
    }

    pub fn get_config_info_by_keys(&self, keys: &[ConfigKey]) -> (usize, Vec<ConfigInfoDto>) {
        let mut info_list = Vec::with_capacity(keys.len());

        for key in keys.iter() {
            let key: ConfigKey = ConfigKey {
                tenant: key.tenant.clone(),
                data_id: key.data_id.clone(),
                group: key.group.clone(),
            };

            if let Some(value) = self.cache.get(&key) {
                let info = ConfigInfoDto {
                    tenant: key.tenant.clone(),
                    group: key.group.clone(),
                    data_id: key.data_id.clone(),
                    desc: value.desc.clone(),
                    content: Some(value.content.clone()),
                    md5: Some(value.md5.clone()),
                };
                info_list.push(info);
            }
        }

        let size = info_list.len();
        (size, info_list)
    }
    /*
    pub(crate) fn get_history_info_page_old(
        &self,
        param: &ConfigHistoryParam,
    ) -> (usize, Vec<ConfigHistoryInfoDto>) {
        let (size, list) = self.config_db.query_config_history_page(param).unwrap();
        let info_list = list
            .into_iter()
            .map(|cfg| ConfigHistoryInfoDto {
                tenant: Some(cfg.tenant),
                group: Some(cfg.group),
                data_id: Some(cfg.data_id),
                modified_time: cfg.last_time,
                content: cfg.content,
                id: cfg.id,
            })
            .collect();
        (size, info_list)
    }
     */

    ///
    /// 从内存列表中直接查询历史记录
    pub(crate) fn get_history_info_page(
        &self,
        param: &ConfigHistoryParam,
    ) -> (usize, Vec<ConfigHistoryInfoDto>) {
        if let (Some(t), Some(g), Some(id)) = (&param.tenant, &param.group, &param.data_id) {
            let key = ConfigKey::new(id, g, t);
            if let Some(v) = self.cache.get(&key) {
                let mut ret = vec![];
                let iter = v.histories.iter().rev();
                if let Some(offset) = param.offset {
                    let n_i = iter.skip(offset as usize);
                    if let Some(limit) = param.limit {
                        let t = n_i.take(limit as usize);
                        for item in t {
                            ret.push(item.to_dto(&key));
                        }
                    } else {
                        for item in n_i {
                            ret.push(item.to_dto(&key));
                        }
                    }
                }
                return (v.histories.len(), ret);
            };
        };
        (0, vec![])
    }

    ///
    /// 将配置中心数据写入 raft snapshot文件中
    ///
    fn build_snapshot(&self, writer: Addr<SnapshotWriterActor>) -> anyhow::Result<()> {
        for (key, value) in &self.cache {
            let value_db: ConfigValueDO = value.clone().into();
            let record = SnapshotRecordDto {
                tree: CONFIG_TREE_NAME.clone(),
                key: key.build_key().as_bytes().to_vec(),
                value: value_db.to_bytes()?,
                op_type: 0,
            };
            writer.do_send(SnapshotWriterRequest::Record(record));
        }
        let seq_record = SnapshotRecordDto {
            tree: SEQUENCE_TREE_NAME.clone(),
            key: SEQ_KEY_CONFIG.as_bytes().to_vec(),
            value: id_to_bin(self.sequence.get_end_id()),
            op_type: 0,
        };
        writer.do_send(SnapshotWriterRequest::Record(seq_record));
        Ok(())
    }

    ///
    /// 迁移数据备件
    fn transfer_backup(&self, writer: Addr<TransferWriterActor>) -> anyhow::Result<()> {
        for (key, value) in &self.cache {
            let value_db: ConfigValueDO = value.clone().into();
            let record = TransferRecordDto {
                table_name: Some(CONFIG_TREE_NAME.clone()),
                key: key.build_key().as_bytes().to_vec(),
                value: value_db.to_bytes()?,
                table_id: 0,
            };
            writer.do_send(TransferWriterRequest::AddRecord(record));
        }
        /*
        let seq_record = TransferRecordDto {
            table_name: Some(SEQUENCE_TREE_NAME.clone()),
            key: SEQ_KEY_CONFIG.as_bytes().to_vec(),
            value: id_to_bin(self.sequence.get_end_id()),
            table_id: 0,
        };
        writer.do_send(TransferWriterRequest::AddRecord(seq_record));
         */
        Ok(())
    }

    pub fn hb(&self, ctx: &mut actix::Context<Self>) {
        ctx.run_later(Duration::from_millis(500), |act, ctx| {
            act.listener.timeout();
            act.hb(ctx);
        });
    }
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<ConfigResult>")]
pub enum ConfigCmd {
    //ADD(ConfigKey, Arc<String>),
    //DELETE(ConfigKey),
    SetTmpValue(ConfigKey, Arc<String>),
    SetFullValue(ConfigKey, ConfigValue),
    InnerSetLastId(u64),
    GET(ConfigKey),
    QueryPageInfo(Box<ConfigQueryParam>),
    QueryInfoByKeys(Box<Vec<ConfigKey>>),
    QueryHistoryPageInfo(Box<ConfigHistoryParam>),
    LISTENER(Vec<ListenerItem>, ListenerSenderType, i64),
    Subscribe(Vec<ListenerItem>, Arc<String>),
    RemoveSubscribe(Vec<ListenerItem>, Arc<String>),
    RemoveSubscribeClient(Arc<String>),
    BuildSnapshot(Addr<SnapshotWriterActor>),
    GetSequenceSection(u64),
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<ConfigResult>")]
pub enum ConfigAsyncCmd {
    Add {
        key: ConfigKey,
        value: Arc<String>,
        op_user: Option<Arc<String>>,
        config_type: Option<Arc<String>>,
        desc: Option<Arc<String>>,
    },
    Delete(ConfigKey),
}

pub enum ConfigResult {
    Data {
        value: Arc<String>,
        md5: Arc<String>,
        config_type: Option<Arc<String>>,
        desc: Option<Arc<String>>,
        last_modified: i64,
    },
    NULL,
    ChangeKey(Vec<ConfigKey>),
    ConfigInfoPage(usize, Vec<ConfigInfoDto>),
    ConfigHistoryInfoPage(usize, Vec<ConfigHistoryInfoDto>),
    SequenceSection {
        //id包含start值
        start: u64,
        //id包含end值
        end: u64,
    },
}

impl Actor for ConfigActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!("ConfigActor started");
        self.hb(ctx);
    }
}

impl Supervised for ConfigActor {
    fn restarting(&mut self, _ctx: &mut <Self as Actor>::Context) {
        log::warn!("ConfigActor restart ...");
    }
}

impl Handler<ConfigCmd> for ConfigActor {
    type Result = anyhow::Result<ConfigResult>;

    fn handle(&mut self, msg: ConfigCmd, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            ConfigCmd::SetTmpValue(key, value) => {
                self.set_tmp_config(key, value);
            }
            ConfigCmd::SetFullValue(key, value) => {
                self.inner_set_config(key, value);
            }
            ConfigCmd::InnerSetLastId(last_id) => {
                self.sequence.set_last_id(last_id);
            }
            ConfigCmd::GET(key) => {
                if let Some(v) = self.cache.get(&key) {
                    return Ok(ConfigResult::Data {
                        value: v.content.clone(),
                        md5: v.md5.clone(),
                        config_type: v.config_type.clone(),
                        desc: v.desc.clone(),
                        last_modified: v.last_modified,
                    });
                }
            }
            ConfigCmd::LISTENER(items, sender, time) => {
                let mut changes = vec![];
                for item in &items {
                    if let Some(v) = self.cache.get(&item.key) {
                        if v.md5 != item.md5 {
                            changes.push(item.key.clone());
                        }
                    } else if !item.md5.is_empty() {
                        changes.push(item.key.clone());
                    }
                }
                if !changes.is_empty() || time <= 0 {
                    sender.send(ListenerResult::DATA(changes)).ok();
                    return Ok(ConfigResult::NULL);
                } else {
                    self.listener.add(items, sender, time);
                    return Ok(ConfigResult::NULL);
                }
            }
            ConfigCmd::Subscribe(items, client_id) => {
                let mut changes = vec![];
                for item in &items {
                    if let Some(v) = self.cache.get(&item.key) {
                        if v.md5 != item.md5 {
                            changes.push(item.key.clone());
                        }
                    } else if !item.md5.is_empty() {
                        changes.push(item.key.clone());
                    }
                }
                self.subscriber.add_subscribe(client_id, items);
                if !changes.is_empty() {
                    return Ok(ConfigResult::ChangeKey(changes));
                }
            }
            ConfigCmd::RemoveSubscribe(items, client_id) => {
                self.subscriber.remove_subscribe(client_id, items);
            }
            ConfigCmd::RemoveSubscribeClient(client_id) => {
                self.subscriber.remove_client_subscribe(client_id);
            }
            ConfigCmd::QueryPageInfo(config_query_param) => {
                let (size, list) = self.get_config_info_page(config_query_param.as_ref());
                return Ok(ConfigResult::ConfigInfoPage(size, list));
            }
            ConfigCmd::QueryInfoByKeys(config_keys) => {
                let (size, list) = self.get_config_info_by_keys(config_keys.as_ref());
                return Ok(ConfigResult::ConfigInfoPage(size, list));
            }
            ConfigCmd::QueryHistoryPageInfo(query_param) => {
                let (size, list) = self.get_history_info_page(query_param.as_ref());
                return Ok(ConfigResult::ConfigHistoryInfoPage(size, list));
            }
            ConfigCmd::BuildSnapshot(writer) => {
                self.build_snapshot(writer).ok();
            }
            ConfigCmd::GetSequenceSection(size) => {
                let (start, end) = self.sequence.next_section(size)?;
                return Ok(ConfigResult::SequenceSection { start, end });
            }
        }
        Ok(ConfigResult::NULL)
    }
}

impl Handler<ConfigAsyncCmd> for ConfigActor {
    type Result = ResponseActFuture<Self, anyhow::Result<ConfigResult>>;

    fn handle(&mut self, msg: ConfigAsyncCmd, _ctx: &mut Context<Self>) -> Self::Result {
        let raft = self.raft.clone();
        let history_info = if let ConfigAsyncCmd::Add { .. } = &msg {
            self.sequence.next_state().ok()
        } else {
            None
        };
        let fut = async move {
            match msg {
                ConfigAsyncCmd::Add {
                    key,
                    value,
                    op_user,
                    config_type,
                    desc,
                } => {
                    if let Some((history_id, history_table_id)) = history_info {
                        let req = ClientRequest::ConfigSet {
                            key: key.build_key(),
                            value,
                            config_type,
                            desc,
                            history_id,
                            history_table_id,
                            op_time: now_millis_i64(),
                            op_user,
                        };
                        Self::send_raft_request(&raft, req).await.ok();
                    }
                }
                ConfigAsyncCmd::Delete(key) => {
                    let req = ClientRequest::ConfigRemove {
                        key: key.build_key(),
                    };
                    Self::send_raft_request(&raft, req).await.ok();
                }
            }
            Ok(ConfigResult::NULL)
        }
        .into_actor(self)
        .map(|r, _act, _ctx| r);
        Box::pin(fut)
    }
}

impl Handler<ConfigRaftCmd> for ConfigActor {
    type Result = anyhow::Result<ConfigRaftResult>;

    fn handle(&mut self, msg: ConfigRaftCmd, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            ConfigRaftCmd::ConfigAdd {
                key,
                value,
                config_type,
                desc,
                history_id,
                history_table_id,
                op_time,
                op_user,
            } => {
                let key: ConfigKey = (&key as &str).into();
                let param = SetConfigParam {
                    key,
                    value,
                    config_type: config_type
                        .map(|v| ConfigType::new_by_value(v.as_ref()).get_value()),
                    desc,
                    history_id,
                    history_table_id,
                    op_time,
                    op_user,
                };
                self.set_config(param).ok();
            }
            ConfigRaftCmd::SetFullValue {
                key,
                value,
                last_id,
            } => {
                self.inner_set_config(key, value);
                if let Some(last_id) = last_id {
                    self.sequence.set_valid_last_id(last_id);
                }
            }
            ConfigRaftCmd::ConfigRemove { key } => {
                let config_key: ConfigKey = (&key as &str).into();
                self.del_config(config_key).ok();
            }
        }
        Ok(ConfigRaftResult::None)
    }
}

impl Handler<TransferDataRequest> for ConfigActor {
    type Result = anyhow::Result<TransferDataResponse>;

    fn handle(&mut self, msg: TransferDataRequest, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            TransferDataRequest::Backup(writer_actor, param) => {
                if param.config {
                    self.transfer_backup(writer_actor)?;
                }
                Ok(TransferDataResponse::None)
            }
        }
    }
}
