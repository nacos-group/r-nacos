pub mod model;

use crate::common::byte_utils::id_to_bin;
use crate::common::constant::{
    CONFIG_TREE_NAME, EMPTY_ARC_STRING, NAMESPACE_TREE_NAME, SEQUENCE_TREE_NAME, SEQ_KEY_CONFIG,
};
use crate::common::string_utils::StringUtils;
use crate::config::core::ConfigActor;
use crate::config::model::ConfigValueDO;
use crate::console::model::NamespaceInfo;
use crate::console::NamespaceUtilsOld;
use crate::namespace::model::{
    Namespace, NamespaceDO, NamespaceParam, NamespaceQueryReq, NamespaceQueryResult,
    NamespaceRaftReq, NamespaceRaftResult,
};
use crate::naming::core::NamingActor;
use crate::raft::filestore::model::SnapshotRecordDto;
use crate::raft::filestore::raftapply::{RaftApplyDataRequest, RaftApplyDataResponse};
use crate::raft::filestore::raftsnapshot::{SnapshotWriterActor, SnapshotWriterRequest};
use crate::raft::store::ClientRequest;
use crate::raft::NacosRaft;
use crate::transfer::model::{
    TransferDataRequest, TransferDataResponse, TransferRecordDto, TransferWriterRequest,
};
use crate::transfer::writer::TransferWriterActor;
use actix::prelude::*;
use async_raft_ext::raft::ClientWriteRequest;
use bean_factory::{bean, BeanFactory, FactoryData, Inject};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

pub const DEFAULT_NAMESPACE: &str = "public";

pub const ALREADY_SYNC_FROM_CONFIG_KEY: &str = "__already_sync";

pub fn build_already_mark_param() -> NamespaceParam {
    NamespaceParam {
        namespace_id: Arc::new(ALREADY_SYNC_FROM_CONFIG_KEY.to_string()),
        namespace_name: None,
        r#type: None,
    }
}

#[bean(inject)]
#[derive(Clone)]
pub struct NamespaceActor {
    data: HashMap<Arc<String>, Arc<Namespace>>,
    id_order_list: Vec<Arc<String>>,
    config_addr: Option<Addr<ConfigActor>>,
    naming_addr: Option<Addr<NamingActor>>,
    raft: Option<Arc<NacosRaft>>,
    raft_node_id: u64,
    already_sync_from_config: bool,
}

impl Actor for NamespaceActor {
    type Context = Context<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("NamespaceActor started");
    }
}

impl Inject for NamespaceActor {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        factory_data: FactoryData,
        _factory: BeanFactory,
        ctx: &mut Self::Context,
    ) {
        self.config_addr = factory_data.get_actor();
        self.naming_addr = factory_data.get_actor();
        self.raft = factory_data.get_bean();
        self.init(ctx);
    }
}

impl NamespaceActor {
    pub(crate) fn new(raft_node_id: u64) -> Self {
        Self {
            data: Default::default(),
            id_order_list: Default::default(),
            config_addr: None,
            naming_addr: None,
            raft: None,
            already_sync_from_config: false,
            raft_node_id,
        }
    }

    fn init(&mut self, _ctx: &mut Context<Self>) {
        self.set_namespace(
            NamespaceParam {
                namespace_id: EMPTY_ARC_STRING.clone(),
                namespace_name: Some(DEFAULT_NAMESPACE.to_owned()),
                r#type: Some("0".to_owned()),
            },
            false,
            false,
        )
    }

    fn set_namespace(&mut self, param: NamespaceParam, only_add: bool, only_update: bool) {
        if !self.already_sync_from_config
            && param.namespace_id.as_str() == ALREADY_SYNC_FROM_CONFIG_KEY
        {
            //标记已同步旧版本数据
            self.already_sync_from_config = true;
        }
        let value = if let Some(v) = self.data.get(&param.namespace_id) {
            if only_add {
                return;
            }
            let mut value = Namespace::default();
            value.namespace_id = param.namespace_id;
            value.namespace_name = if let Some(name) = param.namespace_name {
                name
            } else {
                v.namespace_name.to_owned()
            };
            value.r#type = if let Some(r#type) = param.r#type {
                r#type
            } else {
                v.r#type.to_owned()
            };
            value
        } else {
            if only_update {
                return;
            }
            self.id_order_list.push(param.namespace_id.clone());
            Namespace {
                namespace_id: param.namespace_id,
                namespace_name: param.namespace_name.unwrap_or_default(),
                r#type: param.r#type.unwrap_or("2".to_owned()),
            }
        };
        self.data
            .insert(value.namespace_id.clone(), Arc::new(value));
    }

    fn remove_id(&mut self, id: &Arc<String>) {
        for (i, item) in self.id_order_list.iter().enumerate() {
            if id == item {
                self.id_order_list.remove(i);
                break;
            }
        }
    }

    fn delete(&mut self, id: &Arc<String>) -> bool {
        if id.is_empty() {
            return false;
        }
        if self.data.remove(id).is_some() {
            self.remove_id(id);
            true
        } else {
            false
        }
    }

    fn query_list(&mut self) -> Vec<Arc<Namespace>> {
        let mut list = Vec::with_capacity(self.id_order_list.len());
        for id in self.id_order_list.iter() {
            if let Some(v) = self.data.get(id) {
                list.push(v.clone());
            }
        }
        list
    }

    fn build_snapshot(&self, writer: Addr<SnapshotWriterActor>) -> anyhow::Result<()> {
        for (key, value) in &self.data {
            if key.is_empty() {
                continue;
            }
            let value_db: NamespaceDO = value.as_ref().to_owned().into();
            let record = SnapshotRecordDto {
                tree: NAMESPACE_TREE_NAME.clone(),
                key: key.as_bytes().to_vec(),
                value: value_db.to_bytes()?,
                op_type: 0,
            };
            writer.do_send(SnapshotWriterRequest::Record(record));
        }
        if self.already_sync_from_config {
            let param = build_already_mark_param();
            let value = Namespace {
                namespace_id: param.namespace_id,
                namespace_name: param.namespace_name.unwrap_or_default(),
                r#type: param.r#type.unwrap_or_default(),
            };
            let key = value.namespace_id.clone();
            let value_db: NamespaceDO = value.into();
            let record = SnapshotRecordDto {
                tree: NAMESPACE_TREE_NAME.clone(),
                key: key.as_bytes().to_vec(),
                value: value_db.to_bytes()?,
                op_type: 0,
            };
            writer.do_send(SnapshotWriterRequest::Record(record));
        }
        Ok(())
    }

    ///
    /// 迁移数据备件
    fn transfer_backup(&self, writer: Addr<TransferWriterActor>) -> anyhow::Result<()> {
        for (key, value) in &self.data {
            if key.is_empty() {
                continue;
            }
            let value_db: NamespaceDO = value.as_ref().to_owned().into();
            let record = TransferRecordDto {
                table_name: Some(NAMESPACE_TREE_NAME.clone()),
                key: key.as_bytes().to_vec(),
                value: value_db.to_bytes()?,
                table_id: 0,
            };
            writer.do_send(TransferWriterRequest::AddRecord(record));
        }
        Ok(())
    }

    fn load_snapshot_record(&mut self, record: SnapshotRecordDto) -> anyhow::Result<()> {
        let value_do: NamespaceDO = NamespaceDO::from_bytes(&record.value)?;
        let value: Namespace = value_do.into();
        self.set_namespace(
            NamespaceParam {
                namespace_id: value.namespace_id,
                namespace_name: Some(value.namespace_name),
                r#type: Some(value.r#type),
            },
            false,
            false,
        );
        Ok(())
    }

    fn init_from_old_value(&mut self, namespace_source: Arc<String>) {
        if namespace_source.is_empty() {
            return;
        }
        let list = NamespaceUtilsOld::get_namespaces_from_source(namespace_source);
        for item in list {
            if StringUtils::is_option_empty_arc(&item.namespace_id) {
                continue;
            }
            let item = item.as_ref().to_owned();
            self.set_namespace(
                NamespaceParam {
                    namespace_id: item.namespace_id.unwrap_or_default(),
                    namespace_name: item.namespace_name,
                    r#type: item.r#type,
                },
                true,
                false,
            );
        }
    }

    fn load_completed(&mut self, ctx: &mut Context<Self>) -> anyhow::Result<()> {
        if self.already_sync_from_config {
            return Ok(());
        }
        let config_addr = self.config_addr.clone();
        async move {
            if let Some(config_addr) = config_addr {
                NamespaceUtilsOld::get_namespace_source(&config_addr).await
            } else {
                Arc::new("".to_owned())
            }
        }
        .into_actor(self)
        .map(|r: Arc<String>, act, ctx| {
            act.init_from_old_value(r);
            act.delay_init_from_old_value(ctx);
        })
        .wait(ctx);
        Ok(())
    }

    fn delay_init_from_old_value(&mut self, ctx: &mut Context<Self>) {
        if self.already_sync_from_config {
            return;
        }
        log::info!("delay_init_from_old_value");
        ctx.run_later(Duration::from_secs(5), |act, ctx| {
            if let (Some(raft), Some(config_addr)) = (act.raft.clone(), act.config_addr.clone()) {
                let node_id = act.raft_node_id.to_owned();
                async move { Self::try_init(raft, config_addr, node_id).await }
                    .into_actor(act)
                    .map(|r, act, ctx| {
                        if let Ok(true) = r {
                        } else {
                            act.delay_init_from_old_value(ctx);
                        }
                    })
                    .spawn(ctx);
            }
        });
    }

    async fn try_init(
        raft: Arc<NacosRaft>,
        config_addr: Addr<ConfigActor>,
        local_id: u64,
    ) -> anyhow::Result<bool> {
        if let Some(node_id) = raft.current_leader().await {
            if node_id == local_id {
                let v = NamespaceUtilsOld::get_namespace_source(&config_addr).await;
                raft.client_write(ClientWriteRequest::new(ClientRequest::NamespaceReq(
                    NamespaceRaftReq::InitFromOldValue(v),
                )))
                .await?;
                return Ok(true);
            }
        }
        Ok(false)
    }
}

impl Handler<NamespaceRaftReq> for NamespaceActor {
    type Result = anyhow::Result<NamespaceRaftResult>;

    fn handle(&mut self, msg: NamespaceRaftReq, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            NamespaceRaftReq::AddOnly(v) => {
                self.set_namespace(v, true, true);
                Ok(NamespaceRaftResult::None)
            }
            NamespaceRaftReq::Update(v) => {
                self.set_namespace(v, false, true);
                Ok(NamespaceRaftResult::None)
            }
            NamespaceRaftReq::Set(v) => {
                self.set_namespace(v, false, false);
                Ok(NamespaceRaftResult::None)
            }
            NamespaceRaftReq::Delete { id } => {
                self.delete(&id);
                Ok(NamespaceRaftResult::None)
            }
            NamespaceRaftReq::InitFromOldValue(value) => {
                self.init_from_old_value(value);
                self.already_sync_from_config = true;
                log::info!("namespace InitFromOldValue done");
                Ok(NamespaceRaftResult::None)
            }
        }
    }
}

impl Handler<NamespaceQueryReq> for NamespaceActor {
    type Result = anyhow::Result<NamespaceQueryResult>;
    fn handle(&mut self, msg: NamespaceQueryReq, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            NamespaceQueryReq::List => {
                let list = self.query_list();
                Ok(NamespaceQueryResult::List(list))
            }
        }
    }
}

impl Handler<RaftApplyDataRequest> for NamespaceActor {
    type Result = anyhow::Result<RaftApplyDataResponse>;

    fn handle(&mut self, msg: RaftApplyDataRequest, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            RaftApplyDataRequest::BuildSnapshot(writer) => {
                self.build_snapshot(writer)?;
            }
            RaftApplyDataRequest::LoadSnapshotRecord(record) => {
                self.load_snapshot_record(record)?;
            }
            RaftApplyDataRequest::LoadCompleted => {
                self.load_completed(ctx)?;
            }
        };
        Ok(RaftApplyDataResponse::None)
    }
}

impl Handler<TransferDataRequest> for NamespaceActor {
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
