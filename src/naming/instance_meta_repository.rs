use std::collections::HashMap;
use std::sync::Arc;

use crate::common::pb::service_meta;
use crate::common::protobuf_utils::FileMessageReader;
use crate::naming::model::{InstanceShortKey, ServiceKey};
use quick_protobuf::MessageRead;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

const FILE_MAP_NAME: &str = "file_map";

#[derive(Debug, Clone)]
pub struct InstanceMetaDto {
    pub service_key: ServiceKey,
    pub instance_key: InstanceShortKey,
    pub metadata: Arc<HashMap<String, String>>,
}

impl InstanceMetaDto {
    pub fn new(
        service_key: ServiceKey,
        instance_key: InstanceShortKey,
        metadata: Arc<HashMap<String, String>>,
    ) -> Self {
        Self {
            service_key,
            instance_key,
            metadata,
        }
    }

    pub fn to_proto<'a>(&'a self) -> service_meta::InstanceMetaDo<'a> {
        let metadata: HashMap<_, _> = self
            .metadata
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        service_meta::InstanceMetaDo {
            namespace_id: self.service_key.namespace_id.as_str().into(),
            group_name: self.service_key.group_name.as_str().into(),
            service_name: self.service_key.service_name.as_str().into(),
            ip: self.instance_key.ip.as_str().into(),
            port: self.instance_key.port,
            metadata: metadata
                .into_iter()
                .map(|(k, v)| (std::borrow::Cow::Borrowed(k), std::borrow::Cow::Borrowed(v)))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct InstanceMetaDoOwned {
    pub namespace_id: String,
    pub group_name: String,
    pub service_name: String,
    pub ip: String,
    pub port: u32,
    pub metadata: HashMap<String, String>,
}

impl From<service_meta::InstanceMetaDo<'_>> for InstanceMetaDoOwned {
    fn from(value: service_meta::InstanceMetaDo) -> Self {
        Self {
            namespace_id: value.namespace_id.to_string(),
            group_name: value.group_name.to_string(),
            service_name: value.service_name.to_string(),
            ip: value.ip.to_string(),
            port: value.port,
            metadata: value
                .metadata
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }
}

impl From<InstanceMetaDoOwned> for InstanceMetaDto {
    fn from(value: InstanceMetaDoOwned) -> Self {
        Self {
            service_key: ServiceKey::new(
                &value.namespace_id,
                &value.group_name,
                &value.service_name,
            ),
            instance_key: InstanceShortKey {
                ip: Arc::new(value.ip),
                port: value.port,
            },
            metadata: Arc::new(value.metadata),
        }
    }
}

#[derive(Debug)]
pub struct InstanceMetaRepository {
    base_path: String,
    file_map: HashMap<ServiceKey, String>,
}

impl InstanceMetaRepository {
    pub async fn new(base_path: String) -> anyhow::Result<Self> {
        tokio::fs::create_dir_all(&base_path).await?;
        let mut repository = Self {
            base_path,
            file_map: HashMap::new(),
        };
        repository.load_file_map().await?;
        Ok(repository)
    }

    async fn load_file_map(&mut self) -> anyhow::Result<()> {
        let file_map_path = format!("{}/{}", self.base_path, FILE_MAP_NAME);
        let file = match tokio::fs::File::open(&file_map_path).await {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                log::info!("file_map not found, init empty map");
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        };

        let mut reader = FileMessageReader::new(file, 0);
        while let Ok(data_buf) = reader.read_next().await {
            let mut bytes_reader = quick_protobuf::BytesReader::from_bytes(&data_buf);
            let file_do = service_meta::InstanceFileDo::from_reader(&mut bytes_reader, &data_buf)?;
            let service_key = ServiceKey::new(
                &file_do.namespace_id.to_string(),
                &file_do.group_name.to_string(),
                &file_do.service_name.to_string(),
            );
            let file_name = file_do.file_name.to_string();
            self.file_map.insert(service_key, file_name);
        }
        Ok(())
    }

    async fn save_file_map(&self) -> anyhow::Result<()> {
        let file_map_path = format!("{}/{}", self.base_path, FILE_MAP_NAME);
        let temp_path = format!("{}.tmp", file_map_path);
        let mut file = tokio::fs::File::create(&temp_path).await?;

        for (service_key, file_name) in &self.file_map {
            let file_do = service_meta::InstanceFileDo {
                namespace_id: service_key.namespace_id.as_str().into(),
                group_name: service_key.group_name.as_str().into(),
                service_name: service_key.service_name.as_str().into(),
                file_name: file_name.as_str().into(),
            };
            let mut buf = Vec::new();
            {
                let mut writer = quick_protobuf::Writer::new(&mut buf);
                writer.write_message(&file_do)?;
            }
            let len_buf = crate::common::protobuf_utils::write_varint64(buf.len() as u64);
            file.write_all(&len_buf).await?;
            file.write_all(&buf).await?;
        }

        file.flush().await?;
        tokio::fs::rename(&temp_path, &file_map_path).await?;
        Ok(())
    }

    async fn write_records_to_file(
        &self,
        file_name: &str,
        records: &[InstanceMetaDto],
    ) -> anyhow::Result<()> {
        let file_path = format!("{}/{}", self.base_path, file_name);
        let mut file = tokio::fs::File::create(&file_path).await?;

        for record in records {
            let proto = record.to_proto();
            let mut buf = Vec::new();
            {
                let mut writer = quick_protobuf::Writer::new(&mut buf);
                writer.write_message(&proto)?;
            }
            let len_buf = crate::common::protobuf_utils::write_varint64(buf.len() as u64);
            file.write_all(&len_buf).await?;
            file.write_all(&buf).await?;
        }

        file.flush().await?;
        Ok(())
    }

    async fn read_records_from_file(
        &self,
        file_name: &str,
    ) -> anyhow::Result<Vec<InstanceMetaDoOwned>> {
        let file_path = format!("{}/{}", self.base_path, file_name);
        let file = match tokio::fs::File::open(&file_path).await {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                log::warn!("file not found: {}", file_path);
                return Ok(Vec::new());
            }
            Err(e) => return Err(e.into()),
        };

        let mut reader = FileMessageReader::new(file, 0);
        let mut records = Vec::new();

        while let Ok(data_buf) = reader.read_next().await {
            let mut bytes_reader = quick_protobuf::BytesReader::from_bytes(&data_buf);
            match service_meta::InstanceMetaDo::from_reader(&mut bytes_reader, &data_buf) {
                Ok(meta_do) => {
                    let owned: InstanceMetaDoOwned = meta_do.into();
                    records.push(owned);
                }
                Err(e) => {
                    log::error!("failed to parse InstanceMetaDo: {}", e);
                }
            }
        }

        Ok(records)
    }

    pub async fn update_metadata(
        &mut self,
        service_key: &ServiceKey,
        records: Vec<InstanceMetaDto>,
    ) -> anyhow::Result<()> {
        if let Some(file_name) = self.file_map.get(service_key) {
            self.write_records_to_file(file_name, &records).await?;
        } else {
            let file_name = Uuid::new_v4().simple().to_string();
            self.write_records_to_file(&file_name, &records).await?;
            self.file_map.insert(service_key.clone(), file_name);
            self.save_file_map().await?;
        }
        Ok(())
    }

    pub async fn remove_metadata(
        &mut self,
        service_keys: &[ServiceKey],
    ) -> anyhow::Result<()> {
        for service_key in service_keys {
            if let Some(file_name) = self.file_map.remove(service_key) {
                let file_path = format!("{}/{}", self.base_path, file_name);
                if let Err(e) = tokio::fs::remove_file(&file_path).await {
                    log::warn!("failed to remove file {}: {}", file_path, e);
                }
            }
        }
        self.save_file_map().await?;
        Ok(())
    }

    pub fn list_services(&self) -> Vec<ServiceKey> {
        self.file_map.keys().cloned().collect()
    }
}
