use actix::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::naming::instance_meta_repository::InstanceMetaDto;
use crate::naming::instance_meta_repository::InstanceMetaRepository;
use crate::naming::model::{InstanceShortKey, ServiceKey};

const META_FILE_NAME: &str = "meta";
const SURVIVOR_DIR_S0: &str = "s0";
const SURVIVOR_DIR_S1: &str = "s1";
const DEFAULT_VERSION: &str = "1.0";
const DAY_MILLIS: i64 = 86_400_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstanceMetaInfo {
    version: String,
    current_survivor: u8,
}

impl Default for InstanceMetaInfo {
    fn default() -> Self {
        Self {
            version: DEFAULT_VERSION.to_string(),
            current_survivor: 0,
        }
    }
}

#[derive(Debug, Message)]
#[rtype(result = "anyhow::Result<InstanceMetaManagerResult>")]
pub enum InstanceMetaManagerReq {
    UpdateServiceMeta {
        service_key: ServiceKey,
        records: Vec<InstanceMetaDto>,
    },
    RemoveServiceMeta {
        service_keys: Vec<ServiceKey>,
    },
}

pub enum InstanceMetaManagerResult {
    None,
}

pub struct InstanceMetaManager {
    parent_url: String,
    survivor_repo_0: InstanceMetaRepository,
    survivor_repo_1: InstanceMetaRepository,
    current_survivor: u8,
    start_time: i64,
}

impl InstanceMetaManager {
    pub async fn new(parent_url: String) -> anyhow::Result<Self> {
        tokio::fs::create_dir_all(&parent_url).await?;

        let meta_path = format!("{}/{}", parent_url, META_FILE_NAME);
        let meta_info = match tokio::fs::read_to_string(&meta_path).await {
            Ok(content) => {
                serde_json::from_str::<InstanceMetaInfo>(&content).unwrap_or_else(|e| {
                    log::warn!("failed to parse meta file: {}", e);
                    InstanceMetaInfo::default()
                })
            }
            Err(_) => {
                let info = InstanceMetaInfo::default();
                let temp_path = format!("{}.tmp", meta_path);
                let content = serde_json::to_string(&info)?;
                let mut file = tokio::fs::File::create(&temp_path).await?;
                file.write_all(content.as_bytes()).await?;
                file.flush().await?;
                tokio::fs::rename(&temp_path, &meta_path).await?;
                info
            }
        };

        let s0_path = format!("{}/{}", parent_url, SURVIVOR_DIR_S0);
        let s1_path = format!("{}/{}", parent_url, SURVIVOR_DIR_S1);

        let survivor_repo_0 = InstanceMetaRepository::new(s0_path).await?;
        let survivor_repo_1 = InstanceMetaRepository::new(s1_path).await?;

        let start_time = crate::now_millis_i64();

        Ok(Self {
            parent_url,
            survivor_repo_0,
            survivor_repo_1,
            current_survivor: meta_info.current_survivor,
            start_time,
        })
    }

    fn meta_file_path(&self) -> String {
        format!("{}/{}", self.parent_url, META_FILE_NAME)
    }

    async fn load_meta(&self) -> InstanceMetaInfo {
        let path = self.meta_file_path();
        match tokio::fs::read_to_string(&path).await {
            Ok(content) => match serde_json::from_str::<InstanceMetaInfo>(&content) {
                Ok(info) => info,
                Err(e) => {
                    log::warn!("failed to parse meta file {}: {}", path, e);
                    InstanceMetaInfo::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                log::info!("meta file not found, using defaults");
                InstanceMetaInfo::default()
            }
            Err(e) => {
                log::warn!("failed to read meta file {}: {}", path, e);
                InstanceMetaInfo::default()
            }
        }
    }

    async fn save_meta(&self, info: &InstanceMetaInfo) -> anyhow::Result<()> {
        let path = self.meta_file_path();
        let temp_path = format!("{}.tmp", path);
        let content = serde_json::to_string(info)?;
        let mut file = tokio::fs::File::create(&temp_path).await?;
        file.write_all(content.as_bytes()).await?;
        file.flush().await?;
        tokio::fs::rename(&temp_path, &path).await?;
        Ok(())
    }

    fn current_repo(&mut self) -> &mut InstanceMetaRepository {
        match self.current_survivor {
            0 => &mut self.survivor_repo_0,
            1 | _ => &mut self.survivor_repo_1,
        }
    }

    fn refill_survivor_repo(&self) {
        log::info!(
            "refill_survivor_repo called for survivor_{} (placeholder)",
            self.current_survivor
        );
    }

    fn update_instance_metadata(&self, service_key: &ServiceKey, records: &[InstanceMetaDto]) {
        log::info!(
            "update_instance_metadata: service_key={:?}, record_count={}",
            service_key,
            records.len()
        );
    }

    fn switch_survivor(&mut self, ctx: &mut Context<Self>) {
        self.current_survivor = if self.current_survivor == 0 { 1 } else { 0 };
        self.start_time = crate::now_millis_i64();
        let info = InstanceMetaInfo {
            version: DEFAULT_VERSION.to_string(),
            current_survivor: self.current_survivor,
        };
        let parent_url = self.parent_url.clone();
        let fut = async move {
            let path = format!("{}/{}", parent_url, META_FILE_NAME);
            let temp_path = format!("{}.tmp", path);
            let content = serde_json::to_string(&info)?;
            let mut file = tokio::fs::File::create(&temp_path).await?;
            file.write_all(content.as_bytes()).await?;
            file.flush().await?;
            tokio::fs::rename(&temp_path, &path).await?;
            Ok::<(), anyhow::Error>(())
        }
        .into_actor(self)
        .map(|result, act, _ctx| {
            if let Err(e) = result {
                log::error!("failed to save meta after switch: {}", e);
            }
            act.refill_survivor_repo();
        });
        fut.spawn(ctx);
    }

    fn check_and_switch_if_needed(&mut self, ctx: &mut Context<Self>) {
        let now = crate::now_millis_i64();
        if now - self.start_time > DAY_MILLIS {
            self.switch_survivor(ctx);
        }
    }
}

impl Actor for InstanceMetaManager {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!(
            "InstanceMetaManager started, parent_url: {}",
            self.parent_url
        );
        let current_survivor = self.current_survivor;
        let parent_url = self.parent_url.clone();

        let fut = async move {
            let repo_base_path = if current_survivor == 0 {
                format!("{}/{}", parent_url, SURVIVOR_DIR_S0)
            } else {
                format!("{}/{}", parent_url, SURVIVOR_DIR_S1)
            };
            let repo = InstanceMetaRepository::new(repo_base_path)
                .await
                .map_err(|e| {
                    log::error!("failed to load repository during init: {}", e);
                    e
                });

            if repo.is_err() {
                return Vec::new();
            }

            let repo = repo.unwrap();
            let services = repo.list_services();
            let mut results = Vec::new();
            for service_key in services {
                match repo.get_metadata(&service_key).await {
                    Ok(metadata) => {
                        results.push((service_key, metadata));
                    }
                    Err(e) => {
                        log::error!(
                            "failed to load metadata for service {:?}: {}",
                            service_key,
                            e
                        );
                    }
                }
            }
            results
        }
        .into_actor(self)
        .map(|results, act, _ctx| {
            for (service_key, metadata) in results {
                act.update_instance_metadata(&service_key, &metadata);
            }
            log::info!("InstanceMetaManager initialization completed");
        });
        fut.wait(ctx);
    }
}

impl Handler<InstanceMetaManagerReq> for InstanceMetaManager {
    type Result = ResponseActFuture<Self, anyhow::Result<InstanceMetaManagerResult>>;

    fn handle(
        &mut self,
        msg: InstanceMetaManagerReq,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        self.check_and_switch_if_needed(ctx);
        match msg {
            InstanceMetaManagerReq::UpdateServiceMeta {
                service_key,
                records,
            } => {
                let current_survivor = self.current_survivor;
                let parent_url = self.parent_url.clone();
                let fut = async move {
                    let repo_base_path = if current_survivor == 0 {
                        format!("{}/{}", parent_url, SURVIVOR_DIR_S0)
                    } else {
                        format!("{}/{}", parent_url, SURVIVOR_DIR_S1)
                    };
                    let mut repo = InstanceMetaRepository::new(repo_base_path).await?;
                    repo.update_metadata(&service_key, records).await
                }
                .into_actor(self)
                .map(|result, _act, _ctx| result.map(|_| InstanceMetaManagerResult::None));
                Box::pin(fut)
            }
            InstanceMetaManagerReq::RemoveServiceMeta { service_keys } => {
                let current_survivor = self.current_survivor;
                let parent_url = self.parent_url.clone();
                let fut = async move {
                    let repo_base_path = if current_survivor == 0 {
                        format!("{}/{}", parent_url, SURVIVOR_DIR_S0)
                    } else {
                        format!("{}/{}", parent_url, SURVIVOR_DIR_S1)
                    };
                    let mut repo = InstanceMetaRepository::new(repo_base_path).await?;
                    repo.remove_metadata(&service_keys).await
                }
                .into_actor(self)
                .map(|result, _act, _ctx| result.map(|_| InstanceMetaManagerResult::None));
                Box::pin(fut)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix::Actor;
    use std::collections::HashMap;
    use std::sync::Arc;

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

    #[tokio::test]
    async fn test_constructor_initialization() {
        let dir = tempfile::tempdir().unwrap();
        let parent_url = dir.path().to_str().unwrap().to_owned();

        let manager = InstanceMetaManager::new(parent_url.clone())
            .await
            .unwrap();

        assert_eq!(manager.parent_url, parent_url);
        assert_eq!(manager.current_survivor, 0);

        let s0_path = format!("{}/{}", parent_url, SURVIVOR_DIR_S0);
        let s1_path = format!("{}/{}", parent_url, SURVIVOR_DIR_S1);
        assert!(tokio::fs::metadata(&s0_path).await.is_ok());
        assert!(tokio::fs::metadata(&s1_path).await.is_ok());

        let meta_path = format!("{}/{}", parent_url, META_FILE_NAME);
        assert!(tokio::fs::metadata(&meta_path).await.is_ok());

        let meta_content = tokio::fs::read_to_string(&meta_path).await.unwrap();
        let meta_info: InstanceMetaInfo = serde_json::from_str(&meta_content).unwrap();
        assert_eq!(meta_info.version, DEFAULT_VERSION);
        assert_eq!(meta_info.current_survivor, 0);
    }

    #[tokio::test]
    async fn test_meta_file_exists_and_valid() {
        let dir = tempfile::tempdir().unwrap();
        let parent_url = dir.path().to_str().unwrap().to_owned();

        let manager = InstanceMetaManager::new(parent_url.clone())
            .await
            .unwrap();
        let loaded_meta = manager.load_meta().await;

        assert_eq!(loaded_meta.version, DEFAULT_VERSION);
        assert_eq!(loaded_meta.current_survivor, 0);
    }

    #[tokio::test]
    async fn test_meta_file_not_exists() {
        let dir = tempfile::tempdir().unwrap();
        let parent_url = dir.path().to_str().unwrap().to_owned();

        let manager = InstanceMetaManager::new(parent_url.clone())
            .await
            .unwrap();

        tokio::fs::remove_file(format!("{}/{}", parent_url, META_FILE_NAME))
            .await
            .unwrap();

        let loaded_meta = manager.load_meta().await;
        assert_eq!(loaded_meta.version, DEFAULT_VERSION);
        assert_eq!(loaded_meta.current_survivor, 0);
    }

    #[tokio::test]
    async fn test_meta_file_invalid_format() {
        let dir = tempfile::tempdir().unwrap();
        let parent_url = dir.path().to_str().unwrap().to_owned();

        let meta_path = format!("{}/{}", parent_url, META_FILE_NAME);
        tokio::fs::write(&meta_path, "invalid json content")
            .await
            .unwrap();

        let manager = InstanceMetaManager::new(parent_url.clone())
            .await
            .unwrap();
        let loaded_meta = manager.load_meta().await;

        assert_eq!(loaded_meta.version, DEFAULT_VERSION);
        assert_eq!(loaded_meta.current_survivor, 0);
    }

    #[test]
    fn test_survivor_repo_switch() {
        let dir = tempfile::tempdir().unwrap();
        let parent_url = dir.path().to_str().unwrap().to_owned();

        System::new().block_on(async move {
            let manager = InstanceMetaManager::new(parent_url.clone())
                .await
                .unwrap();
            let addr = manager.start();

            let meta_path = format!("{}/{}", parent_url, META_FILE_NAME);
            let meta_content_before = tokio::fs::read_to_string(&meta_path).await.unwrap();
            let meta_info_before: InstanceMetaInfo = serde_json::from_str(&meta_content_before).unwrap();
            let old_survivor = meta_info_before.current_survivor;

            let service_key = ServiceKey::new("public", "DEFAULT", "test-switch");
            let metadata: HashMap<String, String> = HashMap::new();
            let dto = build_meta_dto("public", "DEFAULT", "test-switch", "127.0.0.1", 8080, metadata);

            let msg = InstanceMetaManagerReq::UpdateServiceMeta {
                service_key,
                records: vec![dto],
            };
            let _ = addr.send(msg).await;

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let meta_content_after = tokio::fs::read_to_string(&meta_path).await.unwrap();
            let meta_info_after: InstanceMetaInfo = serde_json::from_str(&meta_content_after).unwrap();

            assert_eq!(meta_info_after.current_survivor, old_survivor);
            System::current().stop();
        });
    }

    #[test]
    fn test_update_service_meta_new_service() {
        let dir = tempfile::tempdir().unwrap();
        let parent_url = dir.path().to_str().unwrap().to_owned();

        System::new().block_on(async move {
            let manager = InstanceMetaManager::new(parent_url.clone())
                .await
                .unwrap();
            let addr = manager.start();

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

            let msg = InstanceMetaManagerReq::UpdateServiceMeta {
                service_key: service_key.clone(),
                records: vec![dto],
            };

            let result = addr.send(msg).await;
            assert!(result.is_ok());

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let s0_path = format!("{}/{}", parent_url, SURVIVOR_DIR_S0);
            let repo = InstanceMetaRepository::new(s0_path).await.unwrap();
            let records = repo.get_metadata(&service_key).await.unwrap();
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].service_key.namespace_id.as_str(), "public");
            assert_eq!(records[0].service_key.group_name.as_str(), "DEFAULT");
            assert_eq!(records[0].service_key.service_name.as_str(), "test-service");
            assert_eq!(records[0].instance_key.ip.as_str(), "127.0.0.1");
            assert_eq!(records[0].instance_key.port, 8080);
            assert_eq!(&*records[0].metadata, &metadata);
            System::current().stop();
        });
    }

    #[test]
    fn test_update_service_meta_overwrite_existing() {
        let dir = tempfile::tempdir().unwrap();
        let parent_url = dir.path().to_str().unwrap().to_owned();

        System::new().block_on(async move {
            let manager = InstanceMetaManager::new(parent_url.clone())
                .await
                .unwrap();
            let addr = manager.start();

            let service_key = ServiceKey::new("ns1", "G1", "svc1");
            let old_metadata: HashMap<String, String> = {
                let mut m = HashMap::new();
                m.insert("old".to_owned(), "data".to_owned());
                m
            };
            let new_metadata: HashMap<String, String> = {
                let mut m = HashMap::new();
                m.insert("new".to_owned(), "data".to_owned());
                m
            };

            let old_dto = build_meta_dto("ns1", "G1", "svc1", "192.168.1.1", 3000, old_metadata);
            let msg1 = InstanceMetaManagerReq::UpdateServiceMeta {
                service_key: service_key.clone(),
                records: vec![old_dto],
            };
            let _ = addr.send(msg1).await;

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let new_dto = build_meta_dto("ns1", "G1", "svc1", "192.168.1.2", 3001, new_metadata.clone());
            let msg2 = InstanceMetaManagerReq::UpdateServiceMeta {
                service_key: service_key.clone(),
                records: vec![new_dto],
            };
            let _ = addr.send(msg2).await;

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let s0_path = format!("{}/{}", parent_url, SURVIVOR_DIR_S0);
            let repo = InstanceMetaRepository::new(s0_path).await.unwrap();
            let records = repo.get_metadata(&service_key).await.unwrap();
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].instance_key.ip.as_str(), "192.168.1.2");
            assert_eq!(records[0].instance_key.port, 3001);
            assert_eq!(&*records[0].metadata, &new_metadata);
            System::current().stop();
        });
    }

    #[test]
    fn test_remove_service_meta_existing() {
        let dir = tempfile::tempdir().unwrap();
        let parent_url = dir.path().to_str().unwrap().to_owned();

        System::new().block_on(async move {
            let manager = InstanceMetaManager::new(parent_url.clone())
                .await
                .unwrap();
            let addr = manager.start();

            let service_key = ServiceKey::new("public", "DEFAULT", "to-remove");
            let metadata: HashMap<String, String> = HashMap::new();
            let dto = build_meta_dto("public", "DEFAULT", "to-remove", "10.0.0.1", 9090, metadata);

            let msg1 = InstanceMetaManagerReq::UpdateServiceMeta {
                service_key: service_key.clone(),
                records: vec![dto],
            };
            let _ = addr.send(msg1).await;

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let msg2 = InstanceMetaManagerReq::RemoveServiceMeta {
                service_keys: vec![service_key.clone()],
            };
            let result = addr.send(msg2).await;
            assert!(result.is_ok());

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let s0_path = format!("{}/{}", parent_url, SURVIVOR_DIR_S0);
            let repo = InstanceMetaRepository::new(s0_path).await.unwrap();
            let records = repo.get_metadata(&service_key).await.unwrap();
            assert!(records.is_empty());
            System::current().stop();
        });
    }

    #[test]
    fn test_remove_service_meta_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let parent_url = dir.path().to_str().unwrap().to_owned();

        System::new().block_on(async move {
            let manager = InstanceMetaManager::new(parent_url.clone())
                .await
                .unwrap();
            let addr = manager.start();

            let service_key = ServiceKey::new("missing", "G", "svc");
            let msg = InstanceMetaManagerReq::RemoveServiceMeta {
                service_keys: vec![service_key],
            };
            let result = addr.send(msg).await;
            assert!(result.is_ok());
            System::current().stop();
        });
    }

    #[test]
    fn test_remove_multiple_services() {
        let dir = tempfile::tempdir().unwrap();
        let parent_url = dir.path().to_str().unwrap().to_owned();

        System::new().block_on(async move {
            let manager = InstanceMetaManager::new(parent_url.clone())
                .await
                .unwrap();
            let addr = manager.start();

            let service_key1 = ServiceKey::new("ns", "G", "svc1");
            let service_key2 = ServiceKey::new("ns", "G", "svc2");

            let metadata: HashMap<String, String> = HashMap::new();
            let dto1 = build_meta_dto("ns", "G", "svc1", "10.0.0.1", 8080, metadata.clone());
            let dto2 = build_meta_dto("ns", "G", "svc2", "10.0.0.2", 8081, metadata);

            let _ = addr.send(InstanceMetaManagerReq::UpdateServiceMeta {
                service_key: service_key1.clone(),
                records: vec![dto1],
            })
            .await;

            let _ = addr.send(InstanceMetaManagerReq::UpdateServiceMeta {
                service_key: service_key2.clone(),
                records: vec![dto2],
            })
            .await;

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let msg = InstanceMetaManagerReq::RemoveServiceMeta {
                service_keys: vec![service_key1.clone(), service_key2.clone()],
            };
            let _ = addr.send(msg).await;

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let s0_path = format!("{}/{}", parent_url, SURVIVOR_DIR_S0);
            let repo = InstanceMetaRepository::new(s0_path).await.unwrap();
            let records1 = repo.get_metadata(&service_key1).await.unwrap();
            let records2 = repo.get_metadata(&service_key2).await.unwrap();
            assert!(records1.is_empty());
            assert!(records2.is_empty());
            System::current().stop();
        });
    }

    #[test]
    fn test_auto_switch_after_one_day() {
        let dir = tempfile::tempdir().unwrap();
        let parent_url = dir.path().to_str().unwrap().to_owned();

        System::new().block_on(async move {
            let mut manager = InstanceMetaManager::new(parent_url.clone())
                .await
                .unwrap();

            manager.start_time = crate::now_millis_i64() - DAY_MILLIS - 1000;

            let addr = manager.start();

            let meta_path = format!("{}/{}", parent_url, META_FILE_NAME);
            let meta_content_before = tokio::fs::read_to_string(&meta_path).await.unwrap();
            let meta_info_before: InstanceMetaInfo = serde_json::from_str(&meta_content_before).unwrap();
            let original_survivor = meta_info_before.current_survivor;

            let service_key = ServiceKey::new("public", "DEFAULT", "test-auto-switch");
            let metadata: HashMap<String, String> = HashMap::new();
            let dto = build_meta_dto("public", "DEFAULT", "test-auto-switch", "127.0.0.1", 8080, metadata);

            let msg = InstanceMetaManagerReq::UpdateServiceMeta {
                service_key,
                records: vec![dto],
            };
            let _ = addr.send(msg).await;

            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            let meta_content_after = tokio::fs::read_to_string(&meta_path).await.unwrap();
            let meta_info_after: InstanceMetaInfo = serde_json::from_str(&meta_content_after).unwrap();

            assert_ne!(meta_info_after.current_survivor, original_survivor);
            System::current().stop();
        });
    }

    #[test]
    fn test_no_auto_switch_within_one_day() {
        let dir = tempfile::tempdir().unwrap();
        let parent_url = dir.path().to_str().unwrap().to_owned();

        System::new().block_on(async move {
            let manager = InstanceMetaManager::new(parent_url.clone())
                .await
                .unwrap();
            let addr = manager.start();

            let meta_path = format!("{}/{}", parent_url, META_FILE_NAME);
            let meta_content_before = tokio::fs::read_to_string(&meta_path).await.unwrap();
            let meta_info_before: InstanceMetaInfo = serde_json::from_str(&meta_content_before).unwrap();
            let original_survivor = meta_info_before.current_survivor;

            let service_key = ServiceKey::new("public", "DEFAULT", "test-no-switch");
            let metadata: HashMap<String, String> = HashMap::new();
            let dto = build_meta_dto("public", "DEFAULT", "test-no-switch", "127.0.0.1", 8080, metadata);

            let msg = InstanceMetaManagerReq::UpdateServiceMeta {
                service_key,
                records: vec![dto],
            };
            let _ = addr.send(msg).await;

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let meta_content = tokio::fs::read_to_string(&meta_path).await.unwrap();
            let meta_info: InstanceMetaInfo = serde_json::from_str(&meta_content).unwrap();

            assert_eq!(meta_info.current_survivor, original_survivor);
            System::current().stop();
        });
    }

    #[tokio::test]
    async fn test_meta_info_default() {
        let info = InstanceMetaInfo::default();
        assert_eq!(info.version, DEFAULT_VERSION);
        assert_eq!(info.current_survivor, 0);
    }

    #[tokio::test]
    async fn test_meta_file_path() {
        let dir = tempfile::tempdir().unwrap();
        let parent_url = dir.path().to_str().unwrap().to_owned();

        let manager = InstanceMetaManager::new(parent_url.clone())
            .await
            .unwrap();

        let expected_path = format!("{}/{}", parent_url, META_FILE_NAME);
        assert_eq!(manager.meta_file_path(), expected_path);
    }
}
