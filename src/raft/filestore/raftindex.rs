
use actix::prelude::*;
use quick_protobuf::{Writer, BytesReader};
use tokio::{fs::OpenOptions, io::{AsyncWriteExt, AsyncReadExt, AsyncSeekExt}};

use crate::common::{byte_utils::{id_to_bin, bin_to_id}, protobuf_utils::FileMessageReader};

use super::log::{RaftIndex, LogRange, SnapshotRange};


pub struct RaftIndexInnerManager {
    file: tokio::fs::File,
    pub(crate) raft_index: RaftIndex,
    pub(crate) last_applied_log: u64,
}

impl RaftIndexInnerManager {
    pub async fn init(path:&str) -> anyhow::Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path).await?;
        let meta = file.metadata().await?;
        let (last_applied_log,raft_index) = if meta.len()==0 {
            //init write
            let mut index = RaftIndex::default();
            index.logs.push(LogRange{
                id: 0,
                start_index: 0,
                record_count: 0,
                is_close: false,
                mark_remove: false,
            });
            let mut buf = Vec::new();
            let mut writer = Writer::new(&mut buf);
            writer.write_message(&index)?;
            let header_buf = id_to_bin(0);
            file.write(&header_buf).await?;
            file.write(&buf).await?;
            file.flush().await?;
            (0,index)
        }
        else {
            //read
            let mut header_buf = vec![0u8;8];
            file.read(&mut header_buf).await?;
            let last_applied_log = bin_to_id(&mut header_buf);
            let mut file_reader = FileMessageReader::new(file.try_clone().await?,8);
            let buf = file_reader.read_next().await?;
            let mut reader = BytesReader::from_bytes(&buf);
            let index : RaftIndex = reader.read_message(&buf)?;
            (last_applied_log,index)
        };
        Ok(Self {
            file,
            raft_index,
            last_applied_log
        })
    }

    pub async fn write_last_applied_log(&mut self,last_applied_log:u64) -> anyhow::Result<()> {
        self.last_applied_log = last_applied_log;
        self.file.seek(std::io::SeekFrom::Start(0)).await?;
        self.file.write(&id_to_bin(last_applied_log)).await?;
        Ok(())
    }

    pub async fn write_index(&mut self,index:RaftIndex) -> anyhow::Result<()> {
        self.raft_index = index;
        self.file.seek(std::io::SeekFrom::Start(8)).await?;
        let mut buf = Vec::new();
        let mut writer = Writer::new(&mut buf);
        writer.write_message(&self.raft_index)?;
        self.file.write(&buf).await?;
        self.file.flush().await?;
        Ok(())
    }
}


pub struct RaftIndexManager {
    path: String,
    inner: Option<Box<RaftIndexInnerManager>>,
}

impl RaftIndexManager{
    pub fn new(path: String) -> Self {
        Self {
            path,
            inner: None
        }
    }

    pub fn init(&mut self,ctx:&mut Context<Self>) {
        let path = self.path.to_owned();
        async move {
            RaftIndexInnerManager::init(&path).await
        }
        .into_actor(self)
        .map(|r,act,ctx|{
            match r {
                Ok(v) => {
                    act.inner = Some(Box::new(v));
                },
                Err(e) => {
                    log::error!("RaftIndexManager init error,{}",e);
                    ctx.stop();
                },
            }
        })
        .wait(ctx);
    }

    fn inner_is_empty_error() -> anyhow::Error {
        anyhow::anyhow!("inner is empty")
    }

    pub fn load_index_info(&self) -> anyhow::Result<RaftIndexResponse> {
        if let Some(inner) = self.inner.as_ref() {
            Ok(RaftIndexResponse::RaftIndexInfo { 
                raft_index: inner.raft_index.clone(), 
                last_applied_log: inner.last_applied_log, 
            })
        }
        else {
            Err(Self::inner_is_empty_error())
        }
    }

    pub fn write_last_applied_log(&mut self,ctx:&mut Context<Self>,last_applied_log:u64) -> anyhow::Result<RaftIndexResponse> {
        if self.inner.is_none() {
            return Err(Self::inner_is_empty_error())
        }
        let mut inner = self.inner.take();
        async move {
            if let Some(v) = &mut inner{
                match v.write_last_applied_log(last_applied_log).await {
                    Ok(_) => {},
                    Err(err) => {
                        log::error!("write_last_applied_log error,{}",err)
                    },
                }
            }
            inner
        }
        .into_actor(self)
        .map(|v,act,_ctx|{
            act.inner=v;
        })
        .wait(ctx);
        Ok(RaftIndexResponse::None)
    }

    pub fn write_index(&mut self,ctx:&mut Context<Self>,index:RaftIndex) -> anyhow::Result<RaftIndexResponse> {
        if self.inner.is_none() {
            return Err(Self::inner_is_empty_error())
        }
        let mut inner = self.inner.take();
        async move {
            if let Some(v) = &mut inner{
                match v.write_index(index).await {
                    Ok(_) => {},
                    Err(err) => {
                        log::error!("write_index error,{}",err)
                    },
                }
            }
            inner
        }
        .into_actor(self)
        .map(|v,act,_ctx|{
            act.inner=v;
        })
        .wait(ctx);
        Ok(RaftIndexResponse::None)
    }

    pub fn write_logs(&mut self,ctx:&mut Context<Self>,logs:Vec<LogRange>) -> anyhow::Result<RaftIndexResponse> {
        let mut index_info = self.inner.as_ref().unwrap().raft_index.clone();
        index_info.logs = logs;
        self.write_index(ctx, index_info)
    }

    pub fn write_snapshots(&mut self,ctx:&mut Context<Self>,snapshots:Vec<SnapshotRange>) -> anyhow::Result<RaftIndexResponse> {
        let mut index_info = self.inner.as_ref().unwrap().raft_index.clone();
        index_info.snapshots = snapshots;
        self.write_index(ctx, index_info)
    }
}

impl Actor for RaftIndexManager {
    type Context=Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!("RaftIndexManager started");
        self.init(ctx);
    }
}

#[derive(Message, Debug)]
#[rtype(result = "anyhow::Result<RaftIndexResponse>")]
pub enum RaftIndexRequest {
    LoadIndexInfo,
    SaveLogs(Vec<LogRange>),
    SaveSnapshots(Vec<SnapshotRange>),
    SaveLastAppliedLog(u64),
}

pub enum RaftIndexResponse {
    None,
    RaftIndexInfo {
        raft_index: RaftIndex,
        last_applied_log: u64,
    }
}

impl Handler<RaftIndexRequest> for RaftIndexManager {
    type Result = anyhow::Result<RaftIndexResponse>;

    fn handle(&mut self, msg: RaftIndexRequest, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            RaftIndexRequest::LoadIndexInfo => {
                self.load_index_info()
            },
            RaftIndexRequest::SaveSnapshots(snapshots) => {
                self.write_snapshots(ctx, snapshots)
            },
            RaftIndexRequest::SaveLastAppliedLog(last_applied_log) => {
                self.write_last_applied_log(ctx, last_applied_log)
            },
            RaftIndexRequest::SaveLogs(logs) => {
                self.write_logs(ctx, logs)
            },
        }
    }
}
