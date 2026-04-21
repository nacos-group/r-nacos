use crate::naming::core::{NamingActor, NamingCmd, NamingResult};
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
const SURVIVOR_DIR_S1: &str = "s1";
const DEFAULT_VERSION: &str = "1.0";

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
    switch_timeout: i64,
    naming_actor: Option<Addr<NamingActor>>,
    cache_meta: HashMap<ServiceKey, Vec<InstanceMetaDto>>,
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
        let meta_info = Self::load_meta(meta_path.clone()).await.unwrap_or_default();

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
            switch_timeout: 86_400_000,
            cache_meta: HashMap::new(),
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

    async fn save_meta(meta_path: String, info: &InstanceMetaInfo) -> anyhow::Result<()> {
        let path = meta_path;
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

    fn switch_survivor(&mut self, ctx: &mut Context<Self>) {
        let next_survivor = if self.current_survivor == 0 { 1 } else { 0 };
        self.start_time = crate::now_millis_i64();
        let info = InstanceMetaInfo {
            version: DEFAULT_VERSION.to_string(),
            current_survivor: next_survivor,
        };
        let naming_actor = self.naming_actor.clone();
        let meta_path = self.meta_path.clone();
        self.update_delay_meta();
        let fut = async move {
            let mut meta_records = Vec::new();
            if let Some(naming_actor) = naming_actor.as_ref() {
                if let Ok(Ok(NamingResult::AllServiceInstanceMetaData(records))) = naming_actor
                    .send(NamingCmd::QueryAllServiceInstanceMetaData)
                    .await
                {
                    meta_records = records;
                }
            }
            Self::save_meta(meta_path, &info).await?;
            Ok((next_survivor, meta_records))
        }
        .into_actor(self)
        .map(
            |result: anyhow::Result<(u8, Vec<(ServiceKey, Vec<InstanceMetaDto>)>)>, act, _ctx| {
                if let Ok((next_survivor, meta_data_list)) = result {
                    act.current_repo().do_send(InstanceMetaRepositoryReq::Clear);
                    act.current_survivor = next_survivor;
                    for (service_key, records) in meta_data_list {
                        act.cache_meta.insert(service_key, records);
                    }
                    act.update_delay_meta();
                }
            },
        );
        fut.spawn(ctx);
    }

    fn check_and_switch_if_needed(&mut self, ctx: &mut Context<Self>) {
        let now = crate::now_millis_i64();
        if now - self.start_time > self.switch_timeout {
            self.switch_survivor(ctx);
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
        self.check_and_switch_if_needed(ctx);
        ctx.run_later(Duration::from_millis(2000), |act, ctx| {
            act.delay_handle_meta(ctx)
        });
    }

    fn update_delay_meta(&mut self) {
        let current_repo = self.current_repo();
        for (service_key, records) in &self.cache_meta {
            current_repo.do_send(InstanceMetaRepositoryReq::UpdateMetadata(
                service_key.clone(),
                records.clone(),
            ));
        }
        self.cache_meta.clear();
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
                self.cache_meta.insert(service_key, records);
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
