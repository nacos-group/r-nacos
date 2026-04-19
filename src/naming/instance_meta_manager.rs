use actix::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::naming::instance_meta_repository::InstanceMetaDto;
use crate::naming::instance_meta_repository::InstanceMetaRepository;
use crate::naming::model::ServiceKey;

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
        let services = if current_survivor == 0 {
            self.survivor_repo_0.list_services()
        } else {
            self.survivor_repo_1.list_services()
        };

        let repo = if current_survivor == 0 {
            &self.survivor_repo_0
        } else {
            &self.survivor_repo_1
        };

        let repo_ptr = repo as *const InstanceMetaRepository;
        let fut = async move {
            let mut results = Vec::new();
            for service_key in services {
                let repo_ref = unsafe { &*repo_ptr };
                let metadata = repo_ref.get_metadata(&service_key).await.unwrap_or_default();
                results.push((service_key, metadata));
            }
            results
        }
        .into_actor(self)
        .map(|results, act, _ctx| {
            for (service_key, metadata) in results {
                act.update_instance_metadata(&service_key, &metadata);
            }
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
                let repo_base_path = if self.current_survivor == 0 {
                    self.survivor_repo_0.base_path().to_string()
                } else {
                    self.survivor_repo_1.base_path().to_string()
                };
                let fut = async move {
                    let mut repo = InstanceMetaRepository::new(repo_base_path).await?;
                    repo.update_metadata(&service_key, records).await
                }
                .into_actor(self)
                .map(|result, _act, _ctx| result.map(|_| InstanceMetaManagerResult::None));
                Box::pin(fut)
            }
            InstanceMetaManagerReq::RemoveServiceMeta { service_keys } => {
                let repo_base_path = if self.current_survivor == 0 {
                    self.survivor_repo_0.base_path().to_string()
                } else {
                    self.survivor_repo_1.base_path().to_string()
                };
                let fut = async move {
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
