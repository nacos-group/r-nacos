#![allow(clippy::suspicious_open_options)]
use crate::common::constant::{
    CACHE_TREE_NAME, CONFIG_TREE_NAME, EMPTY_STR, MCP_SERVER_TABLE_NAME, MCP_TOOL_SPEC_TABLE_NAME,
    NAMESPACE_TREE_NAME, SEQUENCE_TREE_NAME, USER_TREE_NAME,
};
use crate::common::tempfile::TempFile;
use crate::raft::filestore::raftdata::RaftDataHandler;
use crate::transfer::model::{
    TransferBackupParam, TransferDataRequest, TransferHeaderDto, TransferManagerAsyncRequest,
    TransferManagerResponse, TransferPrefix, TransferRecordDto, TransferWriterAsyncRequest,
    TransferWriterRequest, TransferWriterResponse,
};
use actix::prelude::*;
use bean_factory::{bean, BeanFactory, FactoryData, Inject};
use binrw::BinWriterExt;
use quick_protobuf::Writer;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::{fs::OpenOptions, io::AsyncWriteExt};
use uuid::Uuid;

#[derive(Debug)]
pub struct TransferWriter {
    file: tokio::fs::File,
}

impl TransferWriter {
    pub async fn init(path: &str, header: TransferHeaderDto) -> anyhow::Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .await?;
        let mut buf = Vec::new();
        let mut writer = Writer::new(&mut buf);
        // write prefix
        let data_buf = vec![0u8; 8];
        let mut stream = Cursor::new(data_buf);
        let prefix = TransferPrefix::new();
        stream.write_be(&prefix)?;
        stream.set_position(0);
        file.write_all(stream.get_mut()).await?;
        // write header
        writer.write_message(&header.to_do())?;
        file.write_all(&buf).await?;
        Ok(Self { file })
    }

    pub async fn write(&mut self, buf: &[u8]) -> anyhow::Result<()> {
        self.file.write_all(buf).await?;
        Ok(())
    }

    pub async fn write_record(&mut self, record: &TransferRecordDto) -> anyhow::Result<()> {
        let mut buf = Vec::new();
        let mut writer = Writer::new(&mut buf);
        writer.write_message(&record.to_do())?;
        self.file.write_all(&buf).await?;
        Ok(())
    }

    pub async fn flush(&mut self) -> anyhow::Result<()> {
        self.file.flush().await?;
        Ok(())
    }
}

pub struct TransferWriterActor {
    path: PathBuf,
    header: TransferHeaderDto,
    inner_writer: Option<TransferWriter>,
}

impl TransferWriterActor {
    pub fn new(path: PathBuf, version: u64) -> Self {
        Self {
            path,
            header: TransferHeaderDto::new(version),
            inner_writer: None,
        }
    }

    fn add_table_name_map(&mut self, name: Arc<String>) {
        if self.inner_writer.is_some() {
            log::warn!("add_table_name_map after init header");
            return;
        }
        self.header.add_name(name);
    }

    fn init(&mut self, ctx: &mut Context<Self>) {
        let path = self.path.clone();
        let header = self.header.clone();
        async move { TransferWriter::init(path.to_str().unwrap_or(EMPTY_STR), header).await }
            .into_actor(self)
            .map(|v: anyhow::Result<TransferWriter>, act, ctx| {
                if let Ok(v) = v {
                    act.inner_writer = Some(v);
                } else {
                    ctx.stop()
                }
            })
            .wait(ctx);
    }

    fn add_record(&mut self, record: TransferRecordDto, ctx: &mut Context<Self>) {
        if self.inner_writer.is_none() {
            log::warn!(
                "add_record before init header,ignore record table name:{:?}",
                record.table_name
            );
            return;
        }
        let mut writer = self.inner_writer.take().unwrap();
        async move {
            writer.write_record(&record).await?;
            Ok(writer)
        }
        .into_actor(self)
        .map(|v: anyhow::Result<TransferWriter>, act, ctx| {
            if let Ok(v) = v {
                act.inner_writer = Some(v);
            } else {
                ctx.stop()
            }
        })
        .wait(ctx);
    }
}

impl Actor for TransferWriterActor {
    type Context = Context<Self>;
    fn stopped(&mut self, _ctx: &mut Self::Context) {
        log::info!("TransferWriterActor stopped,file path:{:?}", &self.path);
    }
}
impl Handler<TransferWriterRequest> for TransferWriterActor {
    type Result = anyhow::Result<TransferWriterResponse>;

    fn handle(&mut self, msg: TransferWriterRequest, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            TransferWriterRequest::AddTableNameMap(table_name) => {
                self.add_table_name_map(table_name);
            }
            TransferWriterRequest::InitHeader => {
                self.init(ctx);
            }
            TransferWriterRequest::AddRecord(record) => {
                self.add_record(record, ctx);
            }
        };
        Ok(TransferWriterResponse::None)
    }
}

impl Handler<TransferWriterAsyncRequest> for TransferWriterActor {
    type Result = ResponseActFuture<Self, anyhow::Result<TransferWriterResponse>>;

    fn handle(
        &mut self,
        _msg: TransferWriterAsyncRequest,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let mut writer = self.inner_writer.take().unwrap();
        let fut = async move {
            writer.flush().await?;
            Ok(())
        }
        .into_actor(self)
        .map(|r: anyhow::Result<()>, act, _ctx| {
            r?;
            Ok(TransferWriterResponse::Path(act.path.clone()))
        });
        Box::pin(fut)
    }
}

#[bean(inject)]
pub struct TransferWriterManager {
    data_wrap: Option<Arc<RaftDataHandler>>,
    tmp_dir: PathBuf,
    version: u64,
}

impl TransferWriterManager {
    pub fn new(tmp_dir: PathBuf, version: u64) -> Self {
        Self {
            tmp_dir,
            version,
            data_wrap: None,
        }
    }

    fn build_writer_actor(&self) -> Addr<TransferWriterActor> {
        let mut path = self.tmp_dir.clone();
        path.push(format!("{}.data", Uuid::new_v4().simple()));
        let writer_actor = TransferWriterActor::new(path, self.version).start();
        writer_actor.do_send(TransferWriterRequest::AddTableNameMap(
            CONFIG_TREE_NAME.clone(),
        ));
        writer_actor.do_send(TransferWriterRequest::AddTableNameMap(
            SEQUENCE_TREE_NAME.clone(),
        ));
        writer_actor.do_send(TransferWriterRequest::AddTableNameMap(
            NAMESPACE_TREE_NAME.clone(),
        ));
        writer_actor.do_send(TransferWriterRequest::AddTableNameMap(
            USER_TREE_NAME.clone(),
        ));
        writer_actor.do_send(TransferWriterRequest::AddTableNameMap(
            CACHE_TREE_NAME.clone(),
        ));
        writer_actor.do_send(TransferWriterRequest::AddTableNameMap(
            MCP_TOOL_SPEC_TABLE_NAME.clone(),
        ));
        writer_actor.do_send(TransferWriterRequest::AddTableNameMap(
            MCP_SERVER_TABLE_NAME.clone(),
        ));
        writer_actor.do_send(TransferWriterRequest::InitHeader);
        writer_actor
    }

    async fn do_backup(
        data_wrap: Option<Arc<RaftDataHandler>>,
        backup_param: TransferBackupParam,
        writer_actor: Addr<TransferWriterActor>,
    ) -> anyhow::Result<PathBuf> {
        if let Some(data_wrap) = data_wrap {
            data_wrap
                .config
                .send(TransferDataRequest::Backup(
                    writer_actor.clone(),
                    backup_param.clone(),
                ))
                .await??;
            data_wrap
                .mcp_manager
                .send(TransferDataRequest::Backup(
                    writer_actor.clone(),
                    backup_param.clone(),
                ))
                .await??;
            data_wrap
                .namespace
                .send(TransferDataRequest::Backup(
                    writer_actor.clone(),
                    backup_param.clone(),
                ))
                .await??;
            data_wrap
                .table
                .send(TransferDataRequest::Backup(
                    writer_actor.clone(),
                    backup_param.clone(),
                ))
                .await??;
        } else {
            return Err(anyhow::anyhow!("data_wrap is empty"));
        }
        let res = writer_actor
            .send(TransferWriterAsyncRequest::Flush)
            .await??;
        match res {
            TransferWriterResponse::Path(p) => Ok(p),
            TransferWriterResponse::None => Err(anyhow::anyhow!("backup error")),
        }
    }
}

impl Actor for TransferWriterManager {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("TransferWriterManager started");
    }
}

impl Inject for TransferWriterManager {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        factory_data: FactoryData,
        _factory: BeanFactory,
        _ctx: &mut Self::Context,
    ) {
        self.data_wrap = factory_data.get_bean();
    }
}

impl Handler<TransferManagerAsyncRequest> for TransferWriterManager {
    type Result = ResponseActFuture<Self, anyhow::Result<TransferManagerResponse>>;

    fn handle(
        &mut self,
        msg: TransferManagerAsyncRequest,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let writer_actor = self.build_writer_actor();
        let data_warp = self.data_wrap.clone();
        let fut = async move {
            match msg {
                TransferManagerAsyncRequest::Backup(param) => {
                    Self::do_backup(data_warp, param, writer_actor).await
                }
            }
        }
        .into_actor(self)
        .map(|r, _act, _ctx| {
            let path = r?;
            Ok(TransferManagerResponse::BackupFile(TempFile::new(path)))
        });
        Box::pin(fut)
    }
}
