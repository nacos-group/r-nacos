use crate::naming::core::NamingActor;
use crate::naming::instance_meta_repository::InstanceMetaRepository;
use crate::naming::instance_meta_repository::{
    InstanceMetaDto, InstanceMetaRepositoryReq, InstanceMetaRepositoryResult,
};
use crate::naming::model::{InstanceShortKey, ServiceKey};
use actix::prelude::*;
use bean_factory::{bean, Inject};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::io::AsyncWriteExt;

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

#[bean(inject)]
pub struct InstanceMetaManager {
    parent_url: String,
    meta_path: String,
    survivor_repo_0: Addr<InstanceMetaRepository>,
    survivor_repo_1: Addr<InstanceMetaRepository>,
    current_survivor: u8,
    start_time: i64,
    naming_actor: Option<Addr<NamingActor>>,
}

impl InstanceMetaManager {
    pub async fn new(project_base_url: &str) -> anyhow::Result<Self> {
        let parent_url = Path::new(project_base_url)
            .join("ns_instance_meta")
            .to_string_lossy()
            .into_owned();
        tokio::fs::create_dir_all(&parent_url).await?;

        let meta_path = Path::new(parent_url.as_str())
            .join(META_FILE_NAME)
            .to_string_lossy()
            .into_owned();
        let meta_info = match tokio::fs::read_to_string(&meta_path).await {
            Ok(content) => serde_json::from_str::<InstanceMetaInfo>(&content).unwrap_or_else(|e| {
                log::warn!("failed to parse meta file: {}", e);
                InstanceMetaInfo::default()
            }),
            Err(_) => {
                let info = InstanceMetaInfo::default();
                let temp_path = format!("{}.tmp", &meta_path);
                let content = serde_json::to_string(&info)?;
                let mut file = tokio::fs::File::create(&temp_path).await?;
                file.write_all(content.as_bytes()).await?;
                file.flush().await?;
                tokio::fs::rename(&temp_path, &meta_path).await?;
                info
            }
        };

        let s0_path = Path::new(parent_url.as_str())
            .join(SURVIVOR_DIR_S0)
            .to_string_lossy()
            .into_owned();
        let s1_path = Path::new(parent_url.as_str())
            .join(SURVIVOR_DIR_S1)
            .to_string_lossy()
            .into_owned();

        let survivor_repo_0 = InstanceMetaRepository::new(s0_path).await?.start();
        let survivor_repo_1 = InstanceMetaRepository::new(s1_path).await?.start();

        let start_time = crate::now_millis_i64();

        Ok(Self {
            parent_url,
            meta_path,
            survivor_repo_0,
            survivor_repo_1,
            current_survivor: meta_info.current_survivor,
            start_time,
            naming_actor: None,
        })
    }

    fn meta_file_path(&self) -> String {
        self.meta_path.clone()
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

    fn current_repo(&mut self) -> Addr<InstanceMetaRepository> {
        match self.current_survivor {
            0 => self.survivor_repo_0.clone(),
            1 | _ => self.survivor_repo_1.clone(),
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
        let meta_path = self.meta_path.clone();
        let fut = async move {
            let temp_path = format!("{}.tmp", meta_path);
            let content = serde_json::to_string(&info)?;
            let mut file = tokio::fs::File::create(&temp_path).await?;
            file.write_all(content.as_bytes()).await?;
            file.flush().await?;
            tokio::fs::rename(&temp_path, &meta_path).await?;
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

    fn init_data(&mut self, ctx: &mut Context<InstanceMetaManager>) -> bool {
        let current_repo = self.current_repo();

        let fut = async move {
            if let Ok(Ok(InstanceMetaRepositoryResult::AllMetaData(data))) = current_repo
                .send(InstanceMetaRepositoryReq::GetAllMetaData)
                .await
            {
                data
            } else {
                Vec::new()
            }
        }
        .into_actor(self)
        .map(
            |results: Vec<(ServiceKey, Vec<InstanceMetaDto>)>, act, _ctx| {
                for (service_key, metadata) in results {
                    act.update_instance_metadata(&service_key, &metadata);
                }
                log::info!("InstanceMetaManager initialization completed");
            },
        );
        fut.wait(ctx);
        false
    }
}

impl Actor for InstanceMetaManager {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!(
            "InstanceMetaManager started, parent_url: {}",
            self.parent_url
        );
    }
}

impl Inject for InstanceMetaManager {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        factory_data: bean_factory::FactoryData,
        _factory: bean_factory::BeanFactory,
        ctx: &mut Self::Context,
    ) {
        self.naming_actor = factory_data.get_actor();
        self.init_data(ctx);
    }
}

impl Handler<InstanceMetaManagerReq> for InstanceMetaManager {
    type Result = anyhow::Result<InstanceMetaManagerResult>;

    fn handle(&mut self, msg: InstanceMetaManagerReq, ctx: &mut Self::Context) -> Self::Result {
        //self.check_and_switch_if_needed(ctx);
        let current_repo = self.current_repo();
        match msg {
            InstanceMetaManagerReq::UpdateServiceMeta {
                service_key,
                records,
            } => {
                current_repo.do_send(InstanceMetaRepositoryReq::UpdateMetadata(
                    service_key,
                    records,
                ));
            }
            InstanceMetaManagerReq::RemoveServiceMeta { service_keys } => {
                current_repo.do_send(InstanceMetaRepositoryReq::RemoveMetadata(service_keys));
            }
        }
        Ok(InstanceMetaManagerResult::None)
    }
}
