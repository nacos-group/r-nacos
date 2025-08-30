#![allow(unused_assignments, unused_imports)]

use std::{
    collections::{HashMap, LinkedList},
    hash::Hash,
    sync::{atomic::Ordering, Arc},
};

use crate::common::constant::EMPTY_ARC_STRING;
use crate::naming::cluster::model::ProcessRange;
use crate::now_millis;
use actix_web::rt;
use inner_mem_cache::TimeoutSet;
use rand::prelude::IteratorRandom;
use serde::{Deserialize, Serialize};

use super::{
    api_model::QueryListResult,
    model::{
        Instance, InstanceShortKey, InstanceUpdateTag, ServiceDetailDto, ServiceKey,
        UpdateInstanceType,
    },
};

#[derive(Debug, Clone, Default)]
pub struct ServiceMetadata {
    pub protect_threshold: f32,
}

type InstanceMetaData = Arc<HashMap<String, String>>;

#[derive(Default)]
pub struct Service {
    pub service_name: Arc<String>,
    pub group_name: Arc<String>,
    pub group_service: Arc<String>,
    pub metadata: Arc<HashMap<String, String>>,
    pub protect_threshold: f32,
    pub last_modified_millis: i64,
    //pub has_instance:bool,
    pub namespace_id: Arc<String>,
    pub app_name: String,
    pub check_sum: String,
    pub(crate) last_empty_times: u64,
    pub(crate) instance_size: i64,
    pub(crate) healthy_instance_size: i64,
    //pub cluster_map:HashMap<String,Cluster>,
    pub(crate) instances: HashMap<InstanceShortKey, Arc<Instance>>,
    pub(crate) instance_metadata_map: HashMap<InstanceShortKey, InstanceMetaData>,
    /// 健康状态过期记录，过期后把实例状态改为不健康
    pub(crate) healthy_timeout_set: TimeoutSet<InstanceShortKey>,
    /// 不健康状态过期记录，过期后反实例删除
    pub(crate) unhealthy_timeout_set: TimeoutSet<InstanceShortKey>,
}

impl Service {
    pub(crate) fn recalculate_checksum(&mut self) {
        "".clone_into(&mut self.check_sum);
    }

    /*
    pub(crate) fn remove_instance(&mut self,cluster_name:&str,instance_id:&str) -> UpdateInstanceType {
        if let Some(cluster) = self.cluster_map.get_mut(cluster_name){
            cluster.remove_instance(instance_id);
            return UpdateInstanceType::Remove;
        }
        UpdateInstanceType::None
    }
    */

    pub(crate) fn update_instance(
        &mut self,
        mut instance: Instance,
        update_tag: Option<InstanceUpdateTag>,
        from_sync: bool,
    ) -> (UpdateInstanceType, Option<Arc<String>>) {
        //!("test update_instance {:?}", &instance);
        instance.namespace_id = self.namespace_id.clone();
        instance.group_name = self.group_name.clone();
        instance.service_name = self.service_name.clone();
        instance.group_service = self.group_service.clone();
        let key = instance.get_short_key();
        //let mut update_mark = true;
        let mut rtype = UpdateInstanceType::None;
        let short_key = instance.get_short_key();
        let old_instance = self.instances.get(&key);
        let mut replace_old_client_id = None;
        if let Some(old_instance) = old_instance {
            instance.register_time = old_instance.register_time;
            if !instance.from_grpc && old_instance.from_grpc {
                /*
                match (old_instance.from_grpc, old_instance.is_from_cluster()) {
                    (true, true) => {
                        //需要路由到远程服务更新
                        rtype = UpdateInstanceType::UpdateOtherClusterMetaData(
                            old_instance.from_cluster,
                            instance,
                        );
                        return (rtype, replace_old_client_id);
                    }
                    (true, false) => {
                        //如果新实例来自http,旧实例来自grpc,则保持grpc的实例信息
                        instance.from_grpc = old_instance.from_grpc;
                        instance.client_id = old_instance.client_id.clone();
                    }
                    //直接更新
                    (false, _) => {}
                };
                 */
                instance.from_grpc = old_instance.from_grpc;
                instance.client_id = old_instance.client_id.clone();
                instance.from_cluster = old_instance.from_cluster;
            }
            if !old_instance.client_id.is_empty() && instance.client_id != old_instance.client_id {
                replace_old_client_id = Some(old_instance.client_id.clone());
            }
            if !old_instance.healthy && instance.healthy {
                self.healthy_instance_size += 1;
                #[cfg(feature = "debug")]
                log::info!(
                    "[on update_instance] instance healthy status change to healthy,{:?}",
                    instance.get_instance_key()
                )
            } else if old_instance.healthy && !instance.healthy {
                self.healthy_instance_size -= 1;
                #[cfg(feature = "debug")]
                log::info!(
                    "[on update_instance] instance healthy status change to unhealthy,{:?}",
                    instance.get_instance_key()
                )
            }
            rtype = UpdateInstanceType::UpdateValue;
            if let Some(update_tag) = update_tag {
                if !update_tag.is_none() {
                    if !update_tag.enabled {
                        old_instance.enabled.clone_into(&mut instance.enabled);
                    }
                    if !update_tag.ephemeral {
                        old_instance.ephemeral.clone_into(&mut instance.ephemeral);
                    }
                    if !update_tag.weight {
                        old_instance.weight.clone_into(&mut instance.weight);
                    }
                    if !update_tag.metadata {
                        instance.metadata = old_instance.metadata.clone();
                    } else if update_tag.from_update {
                        //从控制台设置的metadata
                        self.instance_metadata_map
                            .insert(short_key, instance.metadata.clone());
                    } else if let Some(priority_metadata) =
                        self.instance_metadata_map.get(&short_key)
                    {
                        //sdk更新尝试使用高优先级metadata
                        instance.metadata = priority_metadata.clone();
                    }
                } else {
                    //不更新
                    old_instance.enabled.clone_into(&mut instance.enabled);
                    old_instance.ephemeral.clone_into(&mut instance.ephemeral);
                    old_instance.weight.clone_into(&mut instance.weight);
                    instance.metadata = old_instance.metadata.clone();
                    rtype = UpdateInstanceType::UpdateTime;
                }
            }
        } else {
            //新增的尝试使用高优先级metadata
            if let Some(priority_metadata) = self.instance_metadata_map.get(&short_key) {
                instance.metadata = priority_metadata.clone();
            }
            self.instance_size += 1;
            if instance.healthy {
                self.healthy_instance_size += 1;
            }
            rtype = UpdateInstanceType::New;
        }
        let new_instance = Arc::new(instance);
        // 非来自集群的更新才维护实例心跳检测
        if new_instance.is_enable_timeout() && !from_sync {
            self.healthy_timeout_set.add(
                new_instance.last_modified_millis as u64,
                new_instance.get_short_key(),
            );
        }
        self.instances.insert(key, new_instance);
        (rtype, replace_old_client_id)
    }

    ///
    /// 刷新重新纳入本节点管理的实例
    /// 增量http实例增加过期管理
    pub(crate) fn do_refresh_process_range(&mut self) {
        let instances: Vec<&Arc<Instance>> = self
            .instances
            .values()
            .filter(|instance| !instance.from_grpc && instance.is_from_cluster())
            .collect();
        //log::info!("do_refresh_process_range instance size:{}", instances.len());
        for instance in instances {
            /*
            log::info!(
                "do_refresh_process_range item,key:{:?},last_modified_millis:{},client_id:{}",
                instance.get_short_key(),
                instance.last_modified_millis,
                &instance.client_id
            );
             */
            self.healthy_timeout_set.add(
                instance.last_modified_millis as u64,
                instance.get_short_key(),
            );
        }
    }

    pub(crate) fn time_check(
        &mut self,
        healthy_time: i64,
        offline_time: i64,
    ) -> (Vec<InstanceShortKey>, Vec<InstanceShortKey>) {
        let mut remove_list = vec![];
        #[cfg(feature = "debug")]
        log::info!(
            "time_check,healthy_time:{},offline_time:{}",
            healthy_time,
            offline_time
        );
        for key in self.unhealthy_timeout_set.timeout(offline_time as u64) {
            if let Some(instance) = self.instances.get(&key) {
                if !instance.is_enable_timeout() || instance.last_modified_millis > offline_time {
                    continue;
                }
            }
            self.remove_instance(&key, None);
            remove_list.push(key);
        }
        let mut update_list = vec![];
        for key in self.healthy_timeout_set.timeout(healthy_time as u64) {
            if let Some(instance) = self.instances.get(&key) {
                if !instance.is_enable_timeout() || instance.last_modified_millis > healthy_time {
                    continue;
                }
            }
            self.update_instance_healthy_invalid(&key);
            update_list.push(key);
        }
        (remove_list, update_list)
    }

    pub(crate) fn remove_instance(
        &mut self,
        instance_key: &InstanceShortKey,
        client_id: Option<&Arc<String>>,
    ) -> Option<Arc<Instance>> {
        #[cfg(feature = "debug")]
        log::info!(
            "remove_instance,instance_key:{:?},client_id:{:?}",
            instance_key,
            &client_id
        );
        if let Some(client_id) = client_id {
            if let Some(old) = self.instances.get(instance_key) {
                if !client_id.is_empty() && old.client_id.as_str() != client_id.as_str() {
                    //不同的client_id不能删除
                    return None;
                }
            }
        }
        if let Some(old) = self.instances.remove(instance_key) {
            self.instance_size -= 1;
            if self.instance_size == 0 {
                self.last_empty_times = now_millis();
            }
            if old.healthy {
                self.healthy_instance_size -= 1;
            }
            Some(old)
        } else {
            None
        }
    }

    pub(crate) fn update_instance_healthy_invalid(&mut self, instance_id: &InstanceShortKey) {
        if let Some(i) = self.instances.remove(instance_id) {
            if i.healthy {
                self.healthy_instance_size -= 1;
            }
            let mut i = i.as_ref().clone();
            i.healthy = false;
            self.unhealthy_timeout_set
                .add(i.last_modified_millis as u64, instance_id.clone());
            self.instances.insert(instance_id.clone(), Arc::new(i));
        }
    }

    pub(crate) fn get_instance(&self, instance_key: &InstanceShortKey) -> Option<Arc<Instance>> {
        self.instances.get(instance_key).cloned()
    }

    pub(crate) fn get_all_instances(
        &self,
        only_healthy: bool,
        only_enable: bool,
    ) -> Vec<Arc<Instance>> {
        self.instances
            .values()
            .filter(|x| (x.enabled || !only_enable) && (x.healthy || !only_healthy))
            .cloned()
            .collect::<Vec<_>>()
    }

    pub(crate) fn select_one_instance(
        &self,
        only_healthy: bool,
        only_enable: bool,
    ) -> Option<Arc<Instance>> {
        //后续可以考虑支持按权重随机选择
        self.instances
            .values()
            .filter(|x| (x.enabled || !only_enable) && (x.healthy || !only_healthy))
            .cloned()
            .choose(&mut rand::thread_rng())
    }

    /*
    pub(crate) fn notify_listener(&mut self,cluster_name:&str,updateType:UpdateInstanceType) -> UpdateInstanceType {
        if match updateType {
            UpdateInstanceType::New =>{false},
            UpdateInstanceType::Remove =>{false},
            UpdateInstanceType::UpdateValue =>{false},
            _ => {true}
        }{
            return updateType;
        }
        updateType
    }
    */

    pub(crate) fn get_instance_list(
        &self,
        _cluster_names: Vec<String>,
        only_healthy: bool,
        only_enable: bool,
    ) -> Vec<Arc<Instance>> {
        self.get_all_instances(only_healthy, only_enable)
    }

    pub fn get_service_key(&self) -> ServiceKey {
        ServiceKey::new_by_arc(
            self.namespace_id.clone(),
            self.group_name.clone(),
            self.service_name.clone(),
        )
    }

    pub fn get_metadata(&self) -> ServiceMetadata {
        ServiceMetadata {
            protect_threshold: self.protect_threshold,
        }
    }

    pub fn get_service_info(&self) -> ServiceInfoDto {
        ServiceInfoDto {
            service_name: self.service_name.clone(),
            group_name: self.group_name.clone(),
            instance_size: self.instance_size,
            healthy_instance_size: self.healthy_instance_size,
            cluster_count: 0,
            trigger_flag: false,
            metadata: Some(self.metadata.clone()),
            protect_threshold: Some(self.protect_threshold),
        }
    }

    pub fn get_service_detail(&self) -> ServiceDetailDto {
        let metadata = if self.metadata.is_empty() {
            None
        } else {
            Some(self.metadata.clone())
        };
        ServiceDetailDto {
            namespace_id: self.namespace_id.clone(),
            service_name: self.service_name.clone(),
            group_name: self.group_name.clone(),
            metadata,
            protect_threshold: Some(self.protect_threshold),
            ..Default::default()
        }
    }

    pub fn get_owner_http_instances(&self) -> Vec<Arc<Instance>> {
        self.instances
            .values()
            .filter(|x| x.client_id.is_empty())
            .cloned()
            .collect::<Vec<_>>()
    }

    pub(crate) fn exist_priority_metadata(&self, instance_key: &InstanceShortKey) -> bool {
        self.instance_metadata_map.contains_key(instance_key)
    }

    pub(crate) fn get_healthy_timeout_set_item_size(&self) -> usize {
        self.healthy_timeout_set.item_size()
    }

    pub(crate) fn get_unhealthy_timeout_set_item_size(&self) -> usize {
        self.unhealthy_timeout_set.item_size()
    }
}

#[derive(Debug, Default, Clone)]
pub struct ServiceInfoDto {
    pub service_name: Arc<String>,
    pub group_name: Arc<String>,
    pub instance_size: i64,
    pub healthy_instance_size: i64,
    pub cluster_count: i64,
    pub trigger_flag: bool,
    pub metadata: Option<Arc<HashMap<String, String>>>,
    pub protect_threshold: Option<f32>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SubscriberInfoDto {
    pub service_name: Arc<String>,
    pub group_name: Arc<String>,
    pub namespace_id: Arc<String>,
    pub ip: Arc<String>,
    pub port: u16,
}
