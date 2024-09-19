use crate::transfer::model::{TransferHeaderDto, TransferPrefix, TransferRecordDto};
use actix::prelude::*;
use binrw::BinWriterExt;
use quick_protobuf::Writer;
use std::io::Cursor;
use std::sync::Arc;
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

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
    path: Arc<String>,
    header: TransferHeaderDto,
    inner_writer: Option<TransferWriter>,
}

impl TransferWriterActor {
    pub fn new(path: Arc<String>, version: u64) -> Self {
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
        log::info!("TransferWriterActor stopped,file path:{}", &self.path);
    }
}

#[derive(Message, Debug)]
#[rtype(result = "anyhow::Result<TransferWriterResponse>")]
pub enum TransferWriterRequest {
    AddTableNameMap(Arc<String>),
    AddRecord(TransferRecordDto),
}

#[derive(Message, Debug)]
#[rtype(result = "anyhow::Result<TransferWriterResponse>")]
pub enum TransferWriterAsyncRequest {
    Flush,
}

pub enum TransferWriterResponse {
    Path(Arc<String>),
    None,
}

impl Handler<TransferWriterRequest> for TransferWriterActor {
    type Result = anyhow::Result<TransferWriterResponse>;

    fn handle(&mut self, msg: TransferWriterRequest, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            TransferWriterRequest::AddTableNameMap(table_name) => {
                self.add_table_name_map(table_name);
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
