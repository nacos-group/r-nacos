use crate::common::pb::service_meta;
use crate::common::protobuf_utils::MessageBufReader;
use crate::naming::model::{InstanceShortKey, ServiceKey};
use actix::prelude::*;
use quick_protobuf::BytesReader;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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

#[derive(Debug, Clone)]
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

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    #[inline]
    fn build_file_path(&self, file_name: &str) -> String {
        Path::new(self.base_path.as_str())
            .join(file_name)
            .to_string_lossy()
            .into_owned()
    }

    async fn load_file_map(&mut self) -> anyhow::Result<()> {
        let file_map_path = self.build_file_path(FILE_MAP_NAME);
        let mut file = match tokio::fs::File::open(&file_map_path).await {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                log::info!("file_map not found, init empty map");
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        };

        let mut message_reader = MessageBufReader::new();
        let mut data_buf = vec![0u8; 1024];
        let read_len = file.read(&mut data_buf).await?;
        if read_len == 0 {
            return Ok(());
        }
        message_reader.append_next_buf(&data_buf[..read_len]);

        loop {
            loop {
                if let Some(v) = message_reader.next_message_vec() {
                    let mut bytes_reader = BytesReader::from_bytes(v);
                    let file_do: service_meta::InstanceFileDo = bytes_reader.read_message(v)?;
                    let service_key = ServiceKey::new(
                        &file_do.namespace_id.to_string(),
                        &file_do.group_name.to_string(),
                        &file_do.service_name.to_string(),
                    );
                    let file_name = file_do.file_name.to_string();
                    self.file_map.insert(service_key, file_name);
                } else {
                    break;
                }
            }
            let read_len = file.read(&mut data_buf).await?;
            if read_len == 0 {
                return Ok(());
            }
            message_reader.append_next_buf(&data_buf[..read_len]);
        }
    }

    async fn save_file_map(&self) -> anyhow::Result<()> {
        let file_map_path = self.build_file_path(FILE_MAP_NAME);
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
        let file_path = self.build_file_path(file_name);
        let mut file = tokio::fs::File::create(&file_path).await?;

        for record in records {
            let proto = record.to_proto();
            let mut buf = Vec::new();
            {
                let mut writer = quick_protobuf::Writer::new(&mut buf);
                writer.write_message(&proto)?;
            }
            file.write_all(&buf).await?;
        }

        file.flush().await?;
        Ok(())
    }

    async fn read_records_from_file(
        &self,
        file_name: &str,
    ) -> anyhow::Result<Vec<InstanceMetaDoOwned>> {
        let file_path = self.build_file_path(file_name);
        let mut file = match tokio::fs::File::open(&file_path).await {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                log::warn!("file not found: {}", file_path);
                return Ok(Vec::new());
            }
            Err(e) => return Err(e.into()),
        };

        let mut message_reader = MessageBufReader::new();
        let mut data_buf = vec![0u8; 1024];
        let mut records = Vec::new();
        let read_len = file.read(&mut data_buf).await?;
        if read_len == 0 {
            return Ok(records);
        }
        message_reader.append_next_buf(&data_buf[..read_len]);

        loop {
            loop {
                if let Some(v) = message_reader.next_message_vec() {
                    let mut bytes_reader = BytesReader::from_bytes(v);
                    match bytes_reader.read_message::<service_meta::InstanceMetaDo>(v) {
                        Ok(meta_do) => {
                            let owned: InstanceMetaDoOwned = meta_do.into();
                            records.push(owned);
                        }
                        Err(e) => {
                            log::error!("failed to parse InstanceMetaDo: {}", e);
                        }
                    }
                } else {
                    break;
                }
            }
            let read_len = file.read(&mut data_buf).await?;
            if read_len == 0 {
                break;
            }
            message_reader.append_next_buf(&data_buf[..read_len]);
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

    pub async fn remove_metadata(&mut self, service_keys: &[ServiceKey]) -> anyhow::Result<()> {
        for service_key in service_keys {
            if let Some(file_name) = self.file_map.remove(service_key) {
                let file_path = self.build_file_path(&file_name);
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

    pub async fn get_metadata(
        &self,
        service_key: &ServiceKey,
    ) -> anyhow::Result<Vec<InstanceMetaDto>> {
        match self.file_map.get(service_key) {
            Some(file_name) => {
                let records = self.read_records_from_file(file_name).await?;
                Ok(records.into_iter().map(|r| r.into()).collect())
            }
            None => Ok(Vec::new()),
        }
    }

    pub async fn get_all_metadata(
        &self,
    ) -> anyhow::Result<Vec<(ServiceKey, Vec<InstanceMetaDto>)>> {
        let mut result = Vec::new();
        for (service_key, file_name) in &self.file_map {
            let records = self.read_records_from_file(file_name).await?;
            let metas = records.into_iter().map(|r| r.into()).collect();
            result.push((service_key.clone(), metas))
        }
        Ok(result)
    }

    async fn clear(&mut self) -> anyhow::Result<()> {
        tokio::fs::remove_dir_all(&self.base_path).await?;
        tokio::fs::create_dir_all(&self.base_path).await?;
        self.file_map.clear();
        Ok(())
    }

    pub async fn clear_unref_files(&self) -> anyhow::Result<()> {
        let referenced_files: std::collections::HashSet<&str> =
            self.file_map.values().map(|s| s.as_str()).collect();
        let mut entries = tokio::fs::read_dir(&self.base_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();
            if file_name_str.len() < 16 {
                continue;
            }
            if file_name_str == FILE_MAP_NAME {
                continue;
            }
            if referenced_files.contains(file_name_str.as_ref()) {
                continue;
            }
            let file_path = entry.path();
            if let Err(e) = tokio::fs::remove_file(&file_path).await {
                log::warn!("failed to remove unreferenced file {:?}: {}", file_path, e);
            } else {
                log::info!("removed unreferenced file: {:?}", file_path);
            }
        }
        Ok(())
    }

    pub async fn handle_msg(
        &mut self,
        msg: InstanceMetaRepositoryReq,
    ) -> anyhow::Result<InstanceMetaRepositoryResult> {
        match msg {
            InstanceMetaRepositoryReq::UpdateMetadata(service_key, metas) => {
                self.update_metadata(&service_key, metas).await?
            }
            InstanceMetaRepositoryReq::RemoveMetadata(keys) => self.remove_metadata(&keys).await?,
            InstanceMetaRepositoryReq::GetServiceList => {
                let keys = self.list_services();
                return Ok(InstanceMetaRepositoryResult::ServiceList(keys));
            }
            InstanceMetaRepositoryReq::GetServiceMetadata(key) => {
                let metas = self.get_metadata(&key).await?;
                return Ok(InstanceMetaRepositoryResult::ServiceMetadata(metas));
            }
            InstanceMetaRepositoryReq::Clear => {
                self.clear().await?;
            }
            InstanceMetaRepositoryReq::ClearUnRefFile => {
                self.clear_unref_files().await?;
            }
            InstanceMetaRepositoryReq::GetAllMetaData => {
                let data = self.get_all_metadata().await?;
                return Ok(InstanceMetaRepositoryResult::AllMetaData(data));
            }
        }
        Ok(InstanceMetaRepositoryResult::None)
    }
}

impl Actor for InstanceMetaRepository {
    type Context = Context<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!(
            "InstanceMetaRepository started, base_path: {}",
            self.base_path
        );
    }
}

#[derive(Debug, Message)]
#[rtype(result = "anyhow::Result<InstanceMetaRepositoryResult>")]
pub enum InstanceMetaRepositoryReq {
    UpdateMetadata(ServiceKey, Vec<InstanceMetaDto>),
    RemoveMetadata(Vec<ServiceKey>),
    Clear,
    GetServiceList,
    GetServiceMetadata(ServiceKey),
    GetAllMetaData,
    ClearUnRefFile,
}

pub enum InstanceMetaRepositoryResult {
    None,
    ServiceList(Vec<ServiceKey>),
    ServiceMetadata(Vec<InstanceMetaDto>),
    AllMetaData(Vec<(ServiceKey, Vec<InstanceMetaDto>)>),
}

impl Handler<InstanceMetaRepositoryReq> for InstanceMetaRepository {
    type Result = ResponseActFuture<Self, anyhow::Result<InstanceMetaRepositoryResult>>;

    fn handle(&mut self, msg: InstanceMetaRepositoryReq, _ctx: &mut Self::Context) -> Self::Result {
        let this = self.clone();
        let fut = async move {
            let mut that: InstanceMetaRepository = this;
            let result = that.handle_msg(msg).await;
            (that.file_map, result)
        }
        .into_actor(self)
        .map(
            |(file_map, result): (
                HashMap<ServiceKey, String>,
                anyhow::Result<InstanceMetaRepositoryResult>,
            ),
             act,
             _ctx| {
                act.file_map = file_map;
                result
            },
        );
        Box::pin(fut)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_meta_dto(
        namespace_id: &str,
        group_name: &str,
        service_name: &str,
        ip: &str,
        port: u32,
        metadata: HashMap<String, String>,
    ) -> InstanceMetaDto {
        InstanceMetaDto::new(
            ServiceKey::new(namespace_id, group_name, service_name),
            InstanceShortKey::new(Arc::new(ip.to_owned()), port),
            Arc::new(metadata),
        )
    }

    fn assert_meta_eq(
        actual: &InstanceMetaDto,
        expected_ns: &str,
        expected_group: &str,
        expected_svc: &str,
        expected_ip: &str,
        expected_port: u32,
        expected_metadata: &HashMap<String, String>,
    ) {
        assert_eq!(actual.service_key.namespace_id.as_str(), expected_ns);
        assert_eq!(actual.service_key.group_name.as_str(), expected_group);
        assert_eq!(actual.service_key.service_name.as_str(), expected_svc);
        assert_eq!(actual.instance_key.ip.as_str(), expected_ip);
        assert_eq!(actual.instance_key.port, expected_port);
        assert_eq!(&*actual.metadata, expected_metadata);
    }

    #[tokio::test]
    async fn test_update_metadata_new_service() {
        let dir = tempfile::tempdir().unwrap();
        let mut repo = InstanceMetaRepository::new(dir.path().to_str().unwrap().to_owned())
            .await
            .unwrap();

        let service_key = ServiceKey::new("public", "DEFAULT", "test-service");
        let metadata: HashMap<String, String> = {
            let mut m = HashMap::new();
            m.insert("version".to_owned(), "1.0.0".to_owned());
            m.insert("env".to_owned(), "prod".to_owned());
            m
        };
        let dto = build_meta_dto(
            "public",
            "DEFAULT",
            "test-service",
            "127.0.0.1",
            8080,
            metadata.clone(),
        );

        repo.update_metadata(&service_key, vec![dto]).await.unwrap();

        let services = repo.list_services();
        assert_eq!(services.len(), 1);
        assert!(services.contains(&service_key));

        let records = repo.get_metadata(&service_key).await.unwrap();
        assert_eq!(records.len(), 1);
        assert_meta_eq(
            &records[0],
            "public",
            "DEFAULT",
            "test-service",
            "127.0.0.1",
            8080,
            &metadata,
        );
    }

    #[tokio::test]
    async fn test_update_metadata_multiple_records() {
        let dir = tempfile::tempdir().unwrap();
        let mut repo = InstanceMetaRepository::new(dir.path().to_str().unwrap().to_owned())
            .await
            .unwrap();

        let service_key = ServiceKey::new("public", "DEFAULT", "multi-service");
        let meta1: HashMap<String, String> = {
            let mut m = HashMap::new();
            m.insert("k1".to_owned(), "v1".to_owned());
            m
        };
        let meta2: HashMap<String, String> = {
            let mut m = HashMap::new();
            m.insert("k2".to_owned(), "v2".to_owned());
            m
        };

        let dto1 = build_meta_dto(
            "public",
            "DEFAULT",
            "multi-service",
            "10.0.0.1",
            8080,
            meta1.clone(),
        );
        let dto2 = build_meta_dto(
            "public",
            "DEFAULT",
            "multi-service",
            "10.0.0.2",
            8081,
            meta2.clone(),
        );

        repo.update_metadata(&service_key, vec![dto1, dto2])
            .await
            .unwrap();

        let records = repo.get_metadata(&service_key).await.unwrap();
        assert_eq!(records.len(), 2);
        assert_meta_eq(
            &records[0],
            "public",
            "DEFAULT",
            "multi-service",
            "10.0.0.1",
            8080,
            &meta1,
        );
        assert_meta_eq(
            &records[1],
            "public",
            "DEFAULT",
            "multi-service",
            "10.0.0.2",
            8081,
            &meta2,
        );
    }

    #[tokio::test]
    async fn test_update_metadata_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let mut repo = InstanceMetaRepository::new(dir.path().to_str().unwrap().to_owned())
            .await
            .unwrap();

        let service_key = ServiceKey::new("ns1", "G1", "svc1");
        let old_meta: HashMap<String, String> = {
            let mut m = HashMap::new();
            m.insert("old".to_owned(), "data".to_owned());
            m
        };
        let new_meta: HashMap<String, String> = {
            let mut m = HashMap::new();
            m.insert("new".to_owned(), "data".to_owned());
            m
        };

        let old_dto = build_meta_dto("ns1", "G1", "svc1", "192.168.1.1", 3000, old_meta);
        repo.update_metadata(&service_key, vec![old_dto])
            .await
            .unwrap();

        let new_dto = build_meta_dto("ns1", "G1", "svc1", "192.168.1.2", 3001, new_meta.clone());
        repo.update_metadata(&service_key, vec![new_dto])
            .await
            .unwrap();

        let records = repo.get_metadata(&service_key).await.unwrap();
        assert_eq!(records.len(), 1);
        assert_meta_eq(
            &records[0],
            "ns1",
            "G1",
            "svc1",
            "192.168.1.2",
            3001,
            &new_meta,
        );
    }

    #[tokio::test]
    async fn test_get_metadata_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let repo = InstanceMetaRepository::new(dir.path().to_str().unwrap().to_owned())
            .await
            .unwrap();

        let service_key = ServiceKey::new("missing", "G", "svc");
        let records = repo.get_metadata(&service_key).await.unwrap();
        assert!(records.is_empty());
    }

    #[tokio::test]
    async fn test_list_services_empty() {
        let dir = tempfile::tempdir().unwrap();
        let repo = InstanceMetaRepository::new(dir.path().to_str().unwrap().to_owned())
            .await
            .unwrap();

        let services = repo.list_services();
        assert!(services.is_empty());
    }

    #[tokio::test]
    async fn test_list_services_multiple() {
        let dir = tempfile::tempdir().unwrap();
        let mut repo = InstanceMetaRepository::new(dir.path().to_str().unwrap().to_owned())
            .await
            .unwrap();

        let key1 = ServiceKey::new("ns", "G", "svc1");
        let key2 = ServiceKey::new("ns", "G", "svc2");
        let key3 = ServiceKey::new("ns2", "G", "svc3");

        let empty_meta = HashMap::new();
        for key in [&key1, &key2, &key3] {
            let dto = build_meta_dto(
                key.namespace_id.as_str(),
                key.group_name.as_str(),
                key.service_name.as_str(),
                "127.0.0.1",
                8080,
                empty_meta.clone(),
            );
            repo.update_metadata(key, vec![dto]).await.unwrap();
        }

        let services = repo.list_services();
        assert_eq!(services.len(), 3);
        assert!(services.contains(&key1));
        assert!(services.contains(&key2));
        assert!(services.contains(&key3));
    }

    #[tokio::test]
    async fn test_remove_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let mut repo = InstanceMetaRepository::new(dir.path().to_str().unwrap().to_owned())
            .await
            .unwrap();

        let key1 = ServiceKey::new("ns", "G", "svc1");
        let key2 = ServiceKey::new("ns", "G", "svc2");
        let empty_meta = HashMap::new();

        let dto1 = build_meta_dto("ns", "G", "svc1", "10.0.0.1", 8080, empty_meta.clone());
        let dto2 = build_meta_dto("ns", "G", "svc2", "10.0.0.2", 8081, empty_meta);
        repo.update_metadata(&key1, vec![dto1]).await.unwrap();
        repo.update_metadata(&key2, vec![dto2]).await.unwrap();

        assert_eq!(repo.list_services().len(), 2);

        repo.remove_metadata(&[key1.clone()]).await.unwrap();

        assert_eq!(repo.list_services().len(), 1);
        assert!(repo.get_metadata(&key1).await.unwrap().is_empty());
        assert_eq!(repo.get_metadata(&key2).await.unwrap().len(), 1);

        repo.remove_metadata(&[key2.clone()]).await.unwrap();
        assert!(repo.list_services().is_empty());
    }

    #[tokio::test]
    async fn test_new_loads_existing_data() {
        let dir = tempfile::tempdir().unwrap();
        let base_path = dir.path().to_str().unwrap().to_owned();

        let service_key = ServiceKey::new("public", "DEFAULT", "persist-svc");
        let metadata: HashMap<String, String> = {
            let mut m = HashMap::new();
            m.insert("region".to_owned(), "cn-east".to_owned());
            m
        };
        let dto = build_meta_dto(
            "public",
            "DEFAULT",
            "persist-svc",
            "172.16.0.1",
            9090,
            metadata.clone(),
        );

        {
            let mut repo = InstanceMetaRepository::new(base_path.clone())
                .await
                .unwrap();
            repo.update_metadata(&service_key, vec![dto]).await.unwrap();
        }

        let repo2 = InstanceMetaRepository::new(base_path).await.unwrap();
        let services = repo2.list_services();
        assert_eq!(services.len(), 1);
        assert!(services.contains(&service_key));

        let records = repo2.get_metadata(&service_key).await.unwrap();
        assert_eq!(records.len(), 1);
        assert_meta_eq(
            &records[0],
            "public",
            "DEFAULT",
            "persist-svc",
            "172.16.0.1",
            9090,
            &metadata,
        );
    }

    #[tokio::test]
    async fn test_clear_unref_files() {
        let dir = tempfile::tempdir().unwrap();
        let mut repo = InstanceMetaRepository::new(dir.path().to_str().unwrap().to_owned())
            .await
            .unwrap();

        let service_key = ServiceKey::new("ns", "G", "svc1");
        let empty_meta = HashMap::new();
        let dto = build_meta_dto("ns", "G", "svc1", "10.0.0.1", 8080, empty_meta);
        repo.update_metadata(&service_key, vec![dto]).await.unwrap();

        let unreferenced_file_name = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let unreferenced_path = Path::new(dir.path()).join(unreferenced_file_name);
        tokio::fs::write(&unreferenced_path, b"orphan")
            .await
            .unwrap();

        let short_name_path = Path::new(dir.path()).join("short");
        tokio::fs::write(&short_name_path, b"short").await.unwrap();

        assert!(unreferenced_path.exists());
        assert!(short_name_path.exists());

        repo.clear_unref_files().await.unwrap();

        assert!(
            !unreferenced_path.exists(),
            "unreferenced file should be removed"
        );
        assert!(short_name_path.exists(), "short name file should be kept");

        let records = repo.get_metadata(&service_key).await.unwrap();
        assert_eq!(records.len(), 1, "referenced file should be kept");
    }
}
