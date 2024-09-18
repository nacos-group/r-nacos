use crate::transfer::model::{TransferHeaderDto, TransferPrefix, TransferRecordDto};
use actix::prelude::*;
use binrw::BinWriterExt;
use quick_protobuf::Writer;
use std::io::Cursor;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

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
        writer.write_all(stream.get_mut()).await?;
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
}
