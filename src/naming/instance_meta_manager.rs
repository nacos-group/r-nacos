use crate::naming::core::{NamingActor, NamingCmd};
use crate::naming::instance_meta_repository::InstanceMetaRepository;
use crate::naming::instance_meta_repository::{
    InstanceMetaDto, InstanceMetaRepositoryReq, InstanceMetaRepositoryResult,
};
use crate::naming::model::ServiceKey;
use actix::prelude::*;
use bean_factory::{bean, Inject};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

const META_FILE_NAME: &str = "meta";
const SURVIVOR_DIR_S0: &str = "s0";
const DEFAULT_VERSION: &str = "1.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstanceMetaInfo {
    version: String,
}

impl Default for InstanceMetaInfo {
    fn default() -> Self {
        Self {
            version: DEFAULT_VERSION.to_string(),
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
#[allow(unused)]
pub struct InstanceMetaManager {
    parent_url: String,
    meta_path: String,
    survivor_repo: Addr<InstanceMetaRepository>,
    naming_actor: Option<Addr<NamingActor>>,
    cache_meta: HashMap<ServiceKey, Vec<InstanceMetaDto>>,
    last_clear_time: i64,
    clear_interval: i64,
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
        if let Ok(meta_info) = Self::load_meta(meta_path.clone()).await {
            meta_info
        } else {
            let meta = InstanceMetaInfo::default();
            Self::save_meta(meta_path.as_str(), &meta).await.ok();
            meta
        };

        let s0_path = Path::new(parent_url.as_str())
            .join(SURVIVOR_DIR_S0)
            .to_string_lossy()
            .into_owned();

        let survivor_repo = InstanceMetaRepository::new(s0_path).await?.start();

        let last_clear_time = crate::now_millis_i64();

        Ok(Self {
            parent_url,
            meta_path,
            survivor_repo,
            naming_actor: None,
            clear_interval: 86_400_000,
            cache_meta: HashMap::new(),
            last_clear_time,
        })
    }

    async fn load_meta(meta_path: String) -> anyhow::Result<InstanceMetaInfo> {
        let path = meta_path;
        match tokio::fs::read_to_string(&path).await {
            Ok(content) => match serde_json::from_str::<InstanceMetaInfo>(&content) {
                Ok(info) => Ok(info),
                Err(e) => {
                    log::warn!("failed to parse meta file {}: {}", path, e);
                    Ok(InstanceMetaInfo::default())
                }
            },
            Err(_) => {
                let info = InstanceMetaInfo::default();
                let temp_path = format!("{}.tmp", &path);
                let content = serde_json::to_string(&info)?;
                let mut file = tokio::fs::File::create(&temp_path).await?;
                file.write_all(content.as_bytes()).await?;
                file.flush().await?;
                tokio::fs::rename(&temp_path, &path).await?;
                Ok(info)
            }
        }
    }

    async fn save_meta(meta_path: &str, info: &InstanceMetaInfo) -> anyhow::Result<()> {
        let path = meta_path;
        let temp_path = format!("{}.tmp", path);
        let content = serde_json::to_string(info)?;
        let mut file = tokio::fs::File::create(&temp_path).await?;
        file.write_all(content.as_bytes()).await?;
        file.flush().await?;
        tokio::fs::rename(&temp_path, &path).await?;
        Ok(())
    }

    fn current_repo(&self) -> Addr<InstanceMetaRepository> {
        self.survivor_repo.clone()
    }

    fn update_instance_metadata(&self, service_key: ServiceKey, records: Vec<InstanceMetaDto>) {
        log::info!(
            "init service instance metadata: service_key={:?}, record_count={}",
            &service_key,
            records.len()
        );
        if let Some(naming_actor) = self.naming_actor.as_ref() {
            naming_actor.do_send(NamingCmd::InitInstanceMeta(service_key, records));
        }
    }

    fn check_and_clear_unref_files(&mut self) {
        let now = crate::now_millis_i64();
        if now - self.last_clear_time > self.clear_interval {
            self.last_clear_time = now;
            self.survivor_repo
                .do_send(InstanceMetaRepositoryReq::ClearUnRefFile);
        }
    }

    fn init(&mut self, ctx: &mut Context<InstanceMetaManager>) {
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
                    act.update_instance_metadata(service_key, metadata);
                }
                log::info!("InstanceMetaManager initialization completed");
            },
        );
        fut.wait(ctx);
        self.delay_handle_meta(ctx);
    }

    fn delay_handle_meta(&mut self, ctx: &mut Context<Self>) {
        self.update_delay_meta();
        self.check_and_clear_unref_files();
        ctx.run_later(Duration::from_millis(2000), |act, ctx| {
            act.delay_handle_meta(ctx)
        });
    }

    fn update_delay_meta(&mut self) {
        if self.cache_meta.is_empty() {
            return;
        }
        let current_repo = self.current_repo();
        let records_map = std::mem::take(&mut self.cache_meta);
        current_repo.do_send(InstanceMetaRepositoryReq::UpdateBatch(records_map));
    }
}

impl Actor for InstanceMetaManager {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
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
        self.init(ctx);
    }
}

impl Handler<InstanceMetaManagerReq> for InstanceMetaManager {
    type Result = anyhow::Result<InstanceMetaManagerResult>;

    fn handle(&mut self, msg: InstanceMetaManagerReq, _ctx: &mut Self::Context) -> Self::Result {
        let current_repo = self.current_repo();
        match msg {
            InstanceMetaManagerReq::UpdateServiceMeta {
                service_key,
                records,
            } => {
                if records.is_empty() {
                    self.cache_meta.remove(&service_key);
                } else {
                    self.cache_meta.insert(service_key, records);
                }
            }
            InstanceMetaManagerReq::RemoveServiceMeta { service_keys } => {
                for key in &service_keys {
                    self.cache_meta.remove(key);
                }
                current_repo.do_send(InstanceMetaRepositoryReq::RemoveMetadata(service_keys));
            }
        }
        Ok(InstanceMetaManagerResult::None)
    }
}
