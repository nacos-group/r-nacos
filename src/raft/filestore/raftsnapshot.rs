#![allow(clippy::suspicious_open_options)]
use std::{path::Path, sync::Arc};

use actix::prelude::*;
use bean_factory::{bean, Inject};
use quick_protobuf::{BytesReader, Writer};
use tokio::{
    fs::OpenOptions,
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
};

use crate::common::protobuf_utils::MessageBufReader;

use super::{
    log::{LogSnapshotItem, SnapshotHeader, SnapshotRange},
    model::{SnapshotHeaderDto, SnapshotRecordDto},
    raftindex::{RaftIndexManager, RaftIndexRequest, RaftIndexResponse},
};

#[derive(Debug)]
pub struct SnapshotWriter {
    file: tokio::fs::File,
}

impl SnapshotWriter {
    pub async fn init(path: &str, header: SnapshotHeaderDto) -> anyhow::Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            //.append(true)
            //.create_new(true)
            .create(true)
            .open(path)
            .await?;
        let mut buf = Vec::new();
        let mut writer = Writer::new(&mut buf);
        let record = header.to_record_do();
        writer.write_message(&record)?;
        file.write_all(&buf).await?;
        Ok(Self { file })
    }

    pub async fn write(&mut self, buf: &[u8]) -> anyhow::Result<()> {
        self.file.write_all(buf).await?;
        Ok(())
    }

    pub async fn write_record(&mut self, record: &SnapshotRecordDto) -> anyhow::Result<()> {
        let mut buf = Vec::new();
        let mut writer = Writer::new(&mut buf);
        writer.write_message(&record.to_record_do())?;
        self.file.write_all(&buf).await?;
        Ok(())
    }

    pub async fn flush(&mut self) -> anyhow::Result<()> {
        self.file.flush().await?;
        Ok(())
    }
}

///
/// 每次打包镜像创建一个WriterActor
pub struct SnapshotWriterActor {
    path: Arc<String>,
    header: Option<SnapshotHeaderDto>,
    inner_writer: Option<SnapshotWriter>,
}

impl SnapshotWriterActor {
    pub fn new(path: Arc<String>, header: SnapshotHeaderDto) -> Self {
        Self {
            path,
            header: Some(header),
            inner_writer: None,
        }
    }

    fn init(&mut self, ctx: &mut Context<Self>) {
        let path = self.path.clone();
        let header = self.header.take().unwrap();
        async move {
            let writer = SnapshotWriter::init(&path, header).await?;
            Ok(writer)
        }
        .into_actor(self)
        .map(|v: anyhow::Result<SnapshotWriter>, act, ctx| {
            if let Ok(v) = v {
                act.inner_writer = Some(v);
            } else {
                ctx.stop();
            };
        })
        .wait(ctx);
    }

    fn write(&mut self, ctx: &mut Context<Self>, record: SnapshotRecordDto) {
        let mut writer = self.inner_writer.take().unwrap();
        async move {
            writer.write_record(&record).await?;
            Ok(writer)
        }
        .into_actor(self)
        .map(|v: anyhow::Result<SnapshotWriter>, act, ctx| {
            if let Ok(v) = v {
                act.inner_writer = Some(v);
            } else {
                ctx.stop()
            }
        })
        .wait(ctx);
    }

    fn flush(&mut self, ctx: &mut Context<Self>) {
        let mut writer = self.inner_writer.take().unwrap();
        async move {
            writer.flush().await?;
            Ok(writer)
        }
        .into_actor(self)
        .map(|v: anyhow::Result<SnapshotWriter>, act, ctx| {
            if let Ok(v) = v {
                act.inner_writer = Some(v);
            } else {
                ctx.stop()
            }
        })
        .wait(ctx);
    }
}

impl Actor for SnapshotWriterActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.init(ctx);
    }
}

#[derive(Message, Debug)]
#[rtype(result = "anyhow::Result<SnapshotWriterResponse>")]
pub enum SnapshotWriterRequest {
    Record(SnapshotRecordDto),
    Flush,
}

pub enum SnapshotWriterResponse {
    Path(Arc<String>),
    None,
}

impl Handler<SnapshotWriterRequest> for SnapshotWriterActor {
    type Result = anyhow::Result<SnapshotWriterResponse>;

    fn handle(&mut self, msg: SnapshotWriterRequest, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            SnapshotWriterRequest::Record(record) => {
                self.write(ctx, record);
                Ok(SnapshotWriterResponse::None)
            }
            SnapshotWriterRequest::Flush => {
                self.flush(ctx);
                Ok(SnapshotWriterResponse::Path(self.path.clone()))
            }
        }
    }
}

pub struct SnapshotReader {
    file: Box<tokio::fs::File>,
    header: SnapshotHeaderDto,
    message_reader: MessageBufReader,
    is_end: bool,
}

impl SnapshotReader {
    pub async fn init_by_file(mut file: Box<tokio::fs::File>) -> anyhow::Result<Self> {
        let mut message_reader = MessageBufReader::new();
        let mut buf = vec![0u8; 1024];
        file.seek(std::io::SeekFrom::Start(0)).await?;
        let read_len = file.read(&mut buf).await?;
        message_reader.append_next_buf(&buf[..read_len]);
        if let Some(v) = message_reader.next_message_vec() {
            let mut reader = BytesReader::from_bytes(v);
            let header: SnapshotHeader = reader.read_message(v)?;
            Ok(Self {
                file,
                header: header.into(),
                message_reader,
                is_end: false,
            })
        } else {
            Err(anyhow::anyhow!("read snapshot head error"))
        }
    }

    pub async fn init(path: &str) -> anyhow::Result<Self> {
        let mut file = Box::new(OpenOptions::new().read(true).open(path).await?);
        let mut message_reader = MessageBufReader::new();
        let mut buf = vec![0u8; 1024];
        let read_len = file.read(&mut buf).await?;
        message_reader.append_next_buf(&buf[..read_len]);
        if let Some(v) = message_reader.next_message_vec() {
            let mut reader = BytesReader::from_bytes(v);
            let header: SnapshotHeader = reader.read_message(v)?;
            Ok(Self {
                file,
                header: header.into(),
                message_reader,
                is_end: false,
            })
        } else {
            Err(anyhow::anyhow!("read snapshot head error"))
        }
    }

    pub fn get_header(&self) -> &SnapshotHeaderDto {
        &self.header
    }

    pub async fn read_record(&mut self) -> anyhow::Result<Option<SnapshotRecordDto>> {
        if self.is_end {
            return Ok(None);
        }
        loop {
            if let Some(v) = self.message_reader.next_message_vec() {
                let mut reader = BytesReader::from_bytes(v);
                let item: LogSnapshotItem = reader.read_message(v)?;
                let dto = item.into();
                return Ok(Some(dto));
            }
            let mut buf = vec![0u8; 1024];
            let read_len = self.file.read(&mut buf).await?;
            if read_len == 0 {
                self.is_end = true;
                return Ok(None);
            }
            self.message_reader.append_next_buf(&buf[..read_len]);
        }
    }
}

#[bean(inject)]
#[derive(Default)]
pub struct RaftSnapshotManager {
    base_path: Arc<String>,
    snapshots: Vec<SnapshotRange>,
    last_header: Option<SnapshotHeaderDto>,
    building: Option<SnapshotRange>,
    index_manager: Option<Addr<RaftIndexManager>>,
    is_init: bool,
}

impl RaftSnapshotManager {
    pub fn new(base_path: Arc<String>, index_manager: Option<Addr<RaftIndexManager>>) -> Self {
        Self {
            base_path,
            snapshots: Vec::default(),
            last_header: None,
            building: None,
            index_manager,
            is_init: false,
        }
    }

    fn init(&mut self, ctx: &mut Context<Self>) {
        self.load_index_info(ctx);
    }

    fn load_index_info(&mut self, ctx: &mut Context<Self>) {
        //加载索引文件、构建raft日志
        let index_manager = self.index_manager.clone();
        async move {
            if let Some(index_manager) = &index_manager {
                index_manager
                    .send(super::raftindex::RaftIndexRequest::LoadIndexInfo)
                    .await?
            } else {
                Err(anyhow::anyhow!(
                    "load_index_info is error, index_manager is node"
                ))
            }
        }
        .into_actor(self)
        .map(|v, act, ctx| {
            if act.is_init {
                return;
            }
            if let Ok(RaftIndexResponse::RaftIndexInfo {
                raft_index,
                last_applied_log: _last_applied_log,
            }) = v
            {
                act.is_init = true;
                act.snapshots = raft_index.snapshots;
                if let Some(last) = act.snapshots.last() {
                    act.load_snapshot_header(ctx, last.id);
                }
            }
        })
        .wait(ctx);
    }

    fn load_snapshot_header(&mut self, ctx: &mut Context<Self>, snapshot_id: u64) {
        let path = Self::get_snapshot_path(&self.base_path, snapshot_id);
        async move {
            let reader = SnapshotReader::init(&path).await?;
            Ok(reader.header)
        }
        .into_actor(self)
        .map(|v: anyhow::Result<SnapshotHeaderDto>, act, _ctx| {
            if let Ok(v) = v {
                act.last_header = Some(v);
            }
        })
        .wait(ctx);
    }

    fn get_next_id(&mut self) -> anyhow::Result<u64> {
        if self.building.is_some() {
            return Err(anyhow::anyhow!("An snapshots is being packaged"));
        }
        let next_id = if let Some(last) = self.snapshots.last() {
            last.id + 1
        } else {
            1
        };
        Ok(next_id)
    }

    fn get_snapshot_path(base_path: &str, id: u64) -> String {
        Path::new(base_path)
            .join(format!("snapshot_{}", id))
            .to_string_lossy()
            .into_owned()
    }

    fn new_writer(
        &mut self,
        _ctx: &mut Context<Self>,
        header: SnapshotHeaderDto,
        path: Arc<String>,
    ) -> Addr<SnapshotWriterActor> {
        SnapshotWriterActor::new(path, header).start()
    }

    fn complete_snapshot(
        &mut self,
        ctx: &mut Context<Self>,
        snapshot_range: SnapshotRange,
    ) -> anyhow::Result<()> {
        self.building.take();
        //1. 删除历史镜像
        let old_snapshot_len = self.snapshots.len();
        let split_index = if old_snapshot_len > 1 {
            old_snapshot_len - 1
        } else {
            self.snapshots.push(snapshot_range);
            //更新snapshot到index中
            return self.save_snapshot_to_index(ctx);
        };
        for item in &self.snapshots[0..split_index] {
            let path = Self::get_snapshot_path(&self.base_path, item.id);
            std::fs::remove_file(path).ok();
        }

        //2. 更新index中的数据
        let mut new_snapshots = Vec::with_capacity(2);
        if let Some(last_range) = self.snapshots.last() {
            new_snapshots.push(last_range.clone());
        }
        new_snapshots.push(snapshot_range);
        self.snapshots = new_snapshots;
        self.save_snapshot_to_index(ctx)
    }

    fn save_snapshot_to_index(&mut self, ctx: &mut Context<Self>) -> anyhow::Result<()> {
        let index_request = RaftIndexRequest::SaveSnapshots(self.snapshots.clone());
        self.index_manager.as_ref().unwrap().do_send(index_request);
        if let Some(last) = self.snapshots.last() {
            self.load_snapshot_header(ctx, last.id);
        }
        Ok(())
    }

    fn install_snapshot(
        &mut self,
        ctx: &mut Context<Self>,
        snapshot_id: u64,
        end_index: u64,
    ) -> anyhow::Result<()> {
        let snapshot_range = SnapshotRange {
            id: snapshot_id,
            end_index,
        };
        self.complete_snapshot(ctx, snapshot_range)?;
        //更新 member信息到index中
        if let (Some(header), Some(index_manager)) = (&self.last_header, &self.index_manager) {
            let member_after_consensus = if header.member_after_consensus.is_empty() {
                None
            } else {
                Some(header.member_after_consensus.clone())
            };
            let req = RaftIndexRequest::SaveMember {
                member: header.member.clone(),
                member_after_consensus,
                node_addr: Some(header.node_addrs.clone()),
            };
            index_manager.do_send(req);
        }
        Ok(())
    }
}

impl Actor for RaftSnapshotManager {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        if self.index_manager.is_some() {
            self.init(ctx);
        }
        log::info!("RaftSnapshotActor started");
    }
}

impl Inject for RaftSnapshotManager {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        factory_data: bean_factory::FactoryData,
        _factory: bean_factory::BeanFactory,
        ctx: &mut Self::Context,
    ) {
        if self.index_manager.is_none() {
            self.index_manager = factory_data.get_actor();
            self.init(ctx);
        }
    }
}

#[derive(Message, Debug)]
#[rtype(result = "anyhow::Result<RaftSnapshotResponse>")]
pub enum RaftSnapshotRequest {
    GetLastSnapshot,
    NewSnapshot(SnapshotHeaderDto),
    NewSnapshotForLoad,
    CompleteSnapshot(SnapshotRange),
    InstallSnapshot { end_index: u64, snapshot_id: u64 },
}

pub enum RaftSnapshotResponse {
    LastSnapshot(Option<String>, Option<SnapshotHeaderDto>),
    NewSnapshot(Addr<SnapshotWriterActor>, u64, Arc<String>),
    NewSnapshotForLoad(String, u64),
    None,
}

impl Handler<RaftSnapshotRequest> for RaftSnapshotManager {
    type Result = anyhow::Result<RaftSnapshotResponse>;

    fn handle(&mut self, msg: RaftSnapshotRequest, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            RaftSnapshotRequest::GetLastSnapshot => {
                let path = self
                    .snapshots
                    .last()
                    .map(|e| Self::get_snapshot_path(&self.base_path, e.id));
                Ok(RaftSnapshotResponse::LastSnapshot(
                    path,
                    self.last_header.clone(),
                ))
            }
            RaftSnapshotRequest::NewSnapshot(header) => {
                let next_id = self.get_next_id()?;
                let path = Arc::new(Self::get_snapshot_path(&self.base_path, next_id));
                let writer = self.new_writer(ctx, header, path.clone());
                Ok(RaftSnapshotResponse::NewSnapshot(writer, next_id, path))
            }
            RaftSnapshotRequest::NewSnapshotForLoad => {
                let next_id = self.get_next_id()?;
                let path = Self::get_snapshot_path(&self.base_path, next_id);
                Ok(RaftSnapshotResponse::NewSnapshotForLoad(path, next_id))
            }
            RaftSnapshotRequest::CompleteSnapshot(snapshot_range) => {
                self.complete_snapshot(ctx, snapshot_range).ok();
                Ok(RaftSnapshotResponse::None)
            }
            RaftSnapshotRequest::InstallSnapshot {
                end_index,
                snapshot_id,
            } => {
                self.install_snapshot(ctx, snapshot_id, end_index).ok();
                Ok(RaftSnapshotResponse::None)
            }
        }
    }
}
