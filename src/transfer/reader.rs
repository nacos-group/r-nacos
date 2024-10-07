use crate::common::constant::{
    CACHE_TREE_NAME, CONFIG_TREE_NAME, EMPTY_ARC_STRING, NAMESPACE_TREE_NAME, USER_TREE_NAME,
};
use crate::common::pb::transfer::{TransferHeader, TransferItem};
use crate::common::protobuf_utils::MessageBufReader;
use crate::common::sequence_utils::CacheSequence;
use crate::config::core::{ConfigActor, ConfigCmd, ConfigResult, ConfigValue};
use crate::config::model::ConfigValueDO;
use crate::namespace::model::{Namespace, NamespaceDO, NamespaceParam, NamespaceRaftReq};
use crate::raft::db::table::TableManagerReq;
use crate::raft::filestore::raftdata::RaftDataWrap;
use crate::raft::store::ClientRequest;
use crate::raft::NacosRaft;
use crate::transfer::model::{
    TransferHeaderDto, TransferImportParam, TransferImportRequest, TransferImportResponse,
    TransferPrefix, TransferRecordRef,
};
use actix::prelude::*;
use async_raft_ext::raft::ClientWriteRequest;
use bean_factory::{bean, BeanFactory, FactoryData, Inject};
use binrw::BinReaderExt;
use quick_protobuf::BytesReader;
use std::io::Cursor;
use std::sync::Arc;

pub struct TransferReader {
    message_reader: MessageBufReader,
    //prefix: TransferPrefix,
    header: TransferHeaderDto,
}

impl TransferReader {
    pub fn new(data: Vec<u8>) -> anyhow::Result<Self> {
        let mut stream = Cursor::new(&data);
        let prefix: TransferPrefix = stream.read_be()?;
        if prefix.magic != 0x6e61636f {
            return Err(anyhow::anyhow!("transfer file format is invalid"));
        }
        let mut message_reader = MessageBufReader::new_with_data(data, 8);
        let header = if let Some(v) = message_reader.next_message_vec() {
            let mut reader = BytesReader::from_bytes(v);
            let header_do: TransferHeader = reader.read_message(v)?;
            header_do.into()
        } else {
            return Err(anyhow::anyhow!("read header error from transfer file"));
        };
        Ok(Self {
            message_reader,
            //prefix,
            header,
        })
    }

    pub fn read_record(&mut self) -> anyhow::Result<Option<TransferRecordRef>> {
        if let Some(v) = self.message_reader.next_message_vec() {
            let mut reader = BytesReader::from_bytes(v);
            let record_do: TransferItem = reader.read_message(v)?;
            let table_name = if record_do.table_id == 0 {
                if CONFIG_TREE_NAME.as_str() == record_do.table_name.as_ref() {
                    CONFIG_TREE_NAME.clone()
                } else if USER_TREE_NAME.as_str() == record_do.table_name.as_ref() {
                    USER_TREE_NAME.clone()
                } else if CACHE_TREE_NAME.as_str() == record_do.table_name.as_ref() {
                    CACHE_TREE_NAME.clone()
                } else if NAMESPACE_TREE_NAME.as_str() == record_do.table_name.as_ref() {
                    NAMESPACE_TREE_NAME.clone()
                } else {
                    //ignore
                    EMPTY_ARC_STRING.clone()
                }
            } else {
                self.header
                    .id_to_name
                    .get(&record_do.table_id)
                    .cloned()
                    .unwrap_or(EMPTY_ARC_STRING.clone())
            };
            let record = TransferRecordRef::new(table_name, record_do);
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }
}

#[derive(Clone)]
pub struct ConfigCacheSequence {
    seq: CacheSequence,
    config: Addr<ConfigActor>,
}

impl ConfigCacheSequence {
    pub fn new(config: Addr<ConfigActor>) -> Self {
        Self {
            seq: CacheSequence::new(0, 0),
            config,
        }
    }

    pub async fn next_state(&mut self) -> anyhow::Result<(u64, Option<u64>)> {
        if let Some(next_id) = self.seq.next_id() {
            Ok((next_id, None))
        } else if let ConfigResult::SequenceSection { start, end } = self
                .config
                .send(ConfigCmd::GetSequenceSection(100))
                .await??
            {
                self.seq = CacheSequence::new(start + 1, end - start);
                Ok((start, Some(end)))
            } else {
                Err(anyhow::anyhow!("config result is error"))
            }

    }
}

#[bean(inject)]
pub struct TransferImportManager {
    data_wrap: Option<Arc<RaftDataWrap>>,
    raft: Option<Arc<NacosRaft>>,
    importing: bool,
}

impl Default for TransferImportManager {
    fn default() -> Self {
        Self::new()
    }
}


impl TransferImportManager {
    pub fn new() -> Self {
        Self {
            data_wrap: None,
            importing: false,
            raft: None,
        }
    }

    async fn read_then_import(
        data: Vec<u8>,
        param: TransferImportParam,
        raft: Option<Arc<NacosRaft>>,
        data_wrap: Option<Arc<RaftDataWrap>>,
    ) -> anyhow::Result<()> {
        if let (Some(raft), Some(data_wrap)) = (&raft, data_wrap) {
            let mut count = 0;
            let mut ignore = 0;
            let mut reader = TransferReader::new(data)?;
            let mut config_seq = ConfigCacheSequence::new(data_wrap.config.clone());
            while let Ok(Some(record)) = reader.read_record() {
                count += 1;
                if param.config && record.table_name.as_str() == CONFIG_TREE_NAME.as_str() {
                    Self::apply_config(raft, &mut config_seq, record).await?;
                } else if param.config && record.table_name.as_str() == NAMESPACE_TREE_NAME.as_str()
                {
                    Self::apply_namespace(raft, record).await?;
                } else if ( param.user && record.table_name.as_str() == USER_TREE_NAME.as_str() )  ||
                    (param.cache && record.table_name.as_str() == CACHE_TREE_NAME.as_str()) {
                    Self::apply_table(raft, record).await?;
                } else {
                    ignore += 1;
                }
            }
            log::info!("transfer import finished,count:{},ignore:{}", count, ignore);
        }
        Ok(())
    }

    async fn apply_table(
        raft: &Arc<NacosRaft>,
        record: TransferRecordRef<'_>,
    ) -> anyhow::Result<()> {
        let table_req = TableManagerReq::Set {
            table_name: record.table_name.clone(),
            key: record.key.to_vec(),
            value: record.value.to_vec(),
            last_seq_id: None,
        };
        let req = ClientRequest::TableManagerReq(table_req);
        Self::send_raft_request(raft, req).await?;
        Ok(())
    }

    async fn apply_namespace(
        raft: &Arc<NacosRaft>,
        record: TransferRecordRef<'_>,
    ) -> anyhow::Result<()> {
        let value_do: NamespaceDO = NamespaceDO::from_bytes(&record.value)?;
        let value: Namespace = value_do.into();
        let param = NamespaceParam {
            namespace_id: value.namespace_id,
            namespace_name: Some(value.namespace_name),
            r#type: Some(value.r#type),
        };
        let req = ClientRequest::NamespaceReq(NamespaceRaftReq::Update(param));
        Self::send_raft_request(raft, req).await?;
        Ok(())
    }

    async fn apply_config(
        raft: &Arc<NacosRaft>,
        config_seq: &mut ConfigCacheSequence,
        record: TransferRecordRef<'_>,
    ) -> anyhow::Result<()> {
        let value_do = ConfigValueDO::from_bytes(&record.value)?;
        let mut config_value: ConfigValue = value_do.into();
        let mut update_last_id = None;
        for item in &mut config_value.histories {
            let (id, last_id) = config_seq.next_state().await?;
            if let Some(v) = last_id {
                update_last_id = Some(v);
            }
            item.id = id;
        }
        let save_do: ConfigValueDO = config_value.into();
        let req = ClientRequest::ConfigFullValue {
            key: record.key.into_owned(),
            value: save_do.to_bytes()?,
            last_seq_id: update_last_id,
        };
        Self::send_raft_request(raft, req).await?;
        Ok(())
    }

    async fn send_raft_request(raft: &Arc<NacosRaft>, req: ClientRequest) -> anyhow::Result<()> {
        raft.client_write(ClientWriteRequest::new(req)).await?;
        Ok(())
    }

    fn import(&mut self, data: Vec<u8>, param: TransferImportParam, ctx: &mut Context<Self>) {
        log::info!(
            "starting transfer import data,len:{},import param:{:?}",
            data.len(),
            &param
        );
        self.importing = true;
        let data_wrap = self.data_wrap.clone();
        let raft = self.raft.clone();
        async move { Self::read_then_import(data, param, raft, data_wrap).await }
            .into_actor(self)
            .map(|v: anyhow::Result<()>, act, _ctx| {
                match v {
                    Ok(_) => log::info!("transfer import success."),
                    Err(e) => log::error!("transfer import error: {}", e),
                }
                act.importing = false;
            })
            .spawn(ctx);
    }
}

impl Actor for TransferImportManager {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("TransferReaderManager started");
    }
}

impl Inject for TransferImportManager {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        factory_data: FactoryData,
        _factory: BeanFactory,
        _ctx: &mut Self::Context,
    ) {
        self.data_wrap = factory_data.get_bean();
        self.raft = factory_data.get_bean();
    }
}

impl Handler<TransferImportRequest> for TransferImportManager {
    type Result = anyhow::Result<TransferImportResponse>;

    fn handle(&mut self, msg: TransferImportRequest, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            TransferImportRequest::Import(data, param) => {
                if self.importing {
                    Ok(TransferImportResponse::Running)
                } else {
                    self.import(data, param, ctx);
                    Ok(TransferImportResponse::None)
                }
            }
        }
    }
}
