#![allow(
    unused_imports,
    unused_assignments,
    unused_variables,
    unused_mut,
    dead_code
)]

use super::api_model::QueryListResult;
use super::cluster::instance_delay_notify::{
    ClusterInstanceDelayNotifyActor, InstanceDelayNotifyRequest,
};
use super::cluster::model::{
    NamingRouteRequest, ProcessRange, SnapshotForReceive, SnapshotForSend,
};
use super::cluster::node_manage::{InnerNodeManage, NodeManageRequest};
use super::filter::InstanceFilterUtils;
use super::listener::{InnerNamingListener, ListenerItem, NamingListenerCmd};
use super::model::InstanceKey;
use super::model::InstanceShortKey;
use super::model::InstanceUpdateTag;
use super::model::ServiceDetailDto;
use super::model::ServiceInfo;
use super::model::ServiceKey;
use super::model::UpdateInstanceType;
use super::model::{DistroData, Instance};
use super::naming_delay_nofity::DelayNotifyActor;
use super::naming_delay_nofity::DelayNotifyCmd;
use super::naming_subscriber::NamingListenerItem;
use super::naming_subscriber::Subscriber;
use super::service::ServiceInfoDto;
use super::service::ServiceMetadata;
use super::service::{Service, SubscriberInfoDto};
use super::service_index::NamespaceIndex;
use super::service_index::ServiceQueryParam;
use super::NamingUtils;
use crate::common::hash_utils::get_hash_value;
use crate::common::NamingSysConfig;
use crate::common::{delay_notify, AppSysConfig};
use crate::grpc::bistream_manage::BiStreamManage;
use crate::now_millis;
use crate::now_millis_i64;
use crate::utils::gz_encode;
use bean_factory::{bean, Inject, InjectComponent};
use chrono::Local;
use inner_mem_cache::TimeoutSet;
use serde::{Deserialize, Serialize};
use std::cmp::max;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::LinkedList;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use crate::common::constant::EMPTY_ARC_STRING;
use crate::metrics::metrics_key::MetricsKey;
use crate::metrics::model::{MetricsItem, MetricsQuery, MetricsRecord};
use crate::namespace::NamespaceActor;
use actix::prelude::*;
use regex::Regex;

//#[derive(Default)]
#[bean(inject)]
pub struct NamingActor {
    pub(crate) service_map: HashMap<ServiceKey, Service>,
    last_id: u64,
    //用于1.x udp实例变更通知,暂时不启用
    listener_addr: Option<Addr<InnerNamingListener>>,
    delay_notify_addr: Option<Addr<DelayNotifyActor>>,
    pub(crate) subscriber: Subscriber,
    sys_config: NamingSysConfig,
    pub(crate) empty_service_set: TimeoutSet<ServiceKey>,
    pub(crate) instance_metadate_set: TimeoutSet<InstanceKey>,
    pub(crate) namespace_index: NamespaceIndex,
    pub(crate) client_instance_set: HashMap<Arc<String>, HashSet<InstanceKey>>,
    cluster_node_manage: Option<Addr<InnerNodeManage>>,
    cluster_delay_notify: Option<Addr<ClusterInstanceDelayNotifyActor>>,
    namespace_actor: Option<Addr<NamespaceActor>>,
    current_range: Option<ProcessRange>,
    node_id: u64,
    //dal_addr: Addr<ServiceDalActor>,
}

impl Actor for NamingActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.instance_time_out_heartbeat(ctx);
        log::info!(" NamingActor started");
    }
}

impl Inject for NamingActor {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        factory_data: bean_factory::FactoryData,
        _factory: bean_factory::BeanFactory,
        ctx: &mut Self::Context,
    ) {
        self.listener_addr = factory_data.get_actor();
        self.delay_notify_addr = factory_data.get_actor();
        if let Some(notify_addr) = self.delay_notify_addr.as_ref() {
            self.subscriber.set_notify_addr(notify_addr.clone());
        }
        self.cluster_node_manage = factory_data.get_actor();
        self.cluster_delay_notify = factory_data.get_actor();
        self.namespace_actor = factory_data.get_actor();
        self.namespace_index.namespace_actor = self.namespace_actor.clone();
        let sys_config: Option<Arc<AppSysConfig>> = factory_data.get_bean();
        if let Some(sys_config) = sys_config {
            self.sys_config.instance_health_timeout_millis =
                sys_config.naming_health_timeout as i64 + 3000;
            self.sys_config.instance_timeout_millis =
                sys_config.naming_instance_timeout as i64 + 3000;
            self.node_id = sys_config.raft_node_id;
            log::info!("NamingActor change naming timeout info from env,health_timeout:{},instance_timeout:{}"
                ,self.sys_config.instance_health_timeout_millis,self.sys_config.instance_timeout_millis)
        }
        log::info!("NamingActor inject complete");
    }
}

impl Default for NamingActor {
    fn default() -> Self {
        Self::new()
    }
}

impl NamingActor {
    pub fn new() -> Self {
        let mut subscriber = Subscriber::default();
        //let dal_addr = SyncArbiter::start(1,||ServiceDalActor::new());
        Self {
            service_map: Default::default(),
            last_id: 0u64,
            listener_addr: None,
            subscriber,
            delay_notify_addr: None,
            sys_config: NamingSysConfig::new(),
            empty_service_set: Default::default(),
            namespace_index: NamespaceIndex::new(),
            instance_metadate_set: Default::default(),
            client_instance_set: Default::default(),
            cluster_node_manage: None,
            cluster_delay_notify: None,
            current_range: None,
            namespace_actor: None,
            node_id: 0,
            //dal_addr,
        }
    }

    pub fn new_and_create() -> Addr<Self> {
        Self::new().start()
    }

    pub fn create_at_new_system() -> Addr<Self> {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let rt = System::new();
            let addrs = rt.block_on(async { Self::new().start() });
            tx.send(addrs).unwrap();
            rt.run().unwrap();
        });
        rx.recv().unwrap()
    }

    pub(crate) fn get_service(&mut self, key: &ServiceKey) -> Option<&mut Service> {
        match self.service_map.get_mut(key) {
            Some(v) => Some(v),
            None => None,
        }
    }

    pub(crate) fn create_empty_service(&mut self, key: &ServiceKey) {
        let ng_service_name = key.service_name.to_owned();
        match self.get_service(key) {
            Some(_) => {}
            None => {
                let mut service = Service::default();
                let current_time = Local::now().timestamp_millis();
                service.service_name = key.service_name.clone();
                service.namespace_id = key.namespace_id.clone();
                service.group_name = key.group_name.clone();
                service.group_service = Arc::new(NamingUtils::get_group_and_service_name(
                    key.service_name.as_ref(),
                    key.group_name.as_ref(),
                ));
                service.last_modified_millis = current_time;
                service.recalculate_checksum();
                self.namespace_index.insert_service(key.clone());
                //self.dal_addr.do_send(ServiceDalMsg::AddService(service.get_service_do()));
                self.service_map.insert(key.clone(), service);
                self.empty_service_set.add(
                    now_millis() + self.sys_config.service_time_out_millis,
                    key.clone(),
                );
            }
        }
    }

    pub(crate) fn update_service(&mut self, service_info: ServiceDetailDto) {
        let key = ServiceKey::new_by_arc(
            service_info.namespace_id,
            service_info.group_name,
            service_info.service_name,
        );
        match self.get_service(&key) {
            Some(service) => {
                if let Some(protect_threshold) = service_info.protect_threshold {
                    service.protect_threshold = protect_threshold;
                }
                if let Some(metadata) = service_info.metadata {
                    service.metadata = metadata;
                }
            }
            None => {
                let mut service = Service::default();
                let current_time = Local::now().timestamp_millis();
                service.service_name = key.service_name.clone();
                service.namespace_id = key.namespace_id.clone();
                service.group_name = key.group_name.clone();
                service.group_service = Arc::new(NamingUtils::get_group_and_service_name(
                    key.service_name.as_ref(),
                    key.group_name.as_ref(),
                ));
                service.last_modified_millis = current_time;
                if let Some(protect_threshold) = service_info.protect_threshold {
                    service.protect_threshold = protect_threshold;
                }
                if let Some(metadata) = service_info.metadata {
                    service.metadata = metadata;
                }
                service.recalculate_checksum();
                self.namespace_index.insert_service(key.clone());
                //self.dal_addr.do_send(ServiceDalMsg::AddService(service.get_service_do()));
                self.service_map.insert(key.clone(), service);
                self.empty_service_set.add(
                    now_millis() + self.sys_config.service_time_out_millis,
                    key.clone(),
                );
            }
        }
    }

    fn remove_empty_service(&mut self, service_map_key: ServiceKey) -> anyhow::Result<()> {
        if let Some(service) = self.service_map.get(&service_map_key) {
            if service.instance_size <= 0 {
                //控制台发起的不校验过期时间标记
                self.clear_one_empty_service(service_map_key.clone(), 0x7fff_ffff_ffff_ffff);
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "The service has instances,it can't remove!"
                ))
            }
        } else {
            Ok(())
        }
    }

    fn do_notify(
        &mut self,
        tag: &UpdateInstanceType,
        key: ServiceKey,
        instance: Option<Arc<Instance>>,
    ) {
        match tag {
            UpdateInstanceType::New => {
                self.subscriber.notify(key);
                if let (Some(cluster_delay_notify), Some(instance)) =
                    (&self.cluster_delay_notify, instance)
                {
                    cluster_delay_notify
                        .do_send(InstanceDelayNotifyRequest::UpdateInstance(instance));
                }
            }
            UpdateInstanceType::Remove => {
                self.subscriber.notify(key);
                if let (Some(cluster_delay_notify), Some(instance)) =
                    (&self.cluster_delay_notify, instance)
                {
                    cluster_delay_notify
                        .do_send(InstanceDelayNotifyRequest::RemoveInstance(instance));
                }
            }
            UpdateInstanceType::UpdateValue => {
                self.subscriber.notify(key);
                if let (Some(cluster_delay_notify), Some(instance)) =
                    (&self.cluster_delay_notify, instance)
                {
                    cluster_delay_notify
                        .do_send(InstanceDelayNotifyRequest::UpdateInstance(instance));
                }
            }
            _ => {}
        }
    }

    /*
    pub(crate) fn add_instance(&mut self,key:&ServiceKey,instance:Instance) -> UpdateInstanceType {
        let service = self.service_map.get_mut(&key).unwrap();
        let tag = service.update_instance(instance,None);
        self.do_notify(&tag, key.clone());
        tag
    }
     */

    pub fn remove_instance(
        &mut self,
        key: &ServiceKey,
        instance_id: &InstanceShortKey,
        client_id: Option<&Arc<String>>,
    ) -> UpdateInstanceType {
        let service = if let Some(service) = self.service_map.get_mut(key) {
            service
        } else {
            return UpdateInstanceType::None;
        };
        let mut real_client_id = None;
        let old_instance = service.remove_instance(instance_id, client_id);
        let now = now_millis();
        let tag = if let Some(old_instance) = &old_instance {
            real_client_id = Some(old_instance.client_id.clone());
            let short_key = old_instance.get_short_key();
            if service.exist_priority_metadata(&short_key) {
                let instance_key =
                    InstanceKey::new_by_service_key(key, short_key.ip, short_key.port);
                self.instance_metadate_set.add(
                    now + self.sys_config.instance_metadata_time_out_millis,
                    instance_key,
                );
            }
            UpdateInstanceType::Remove
        } else {
            UpdateInstanceType::None
        };
        if service.instance_size <= 0 {
            self.empty_service_set
                .add(now + self.sys_config.service_time_out_millis, key.clone());
        }
        let remove_instance = old_instance.filter(|e| !e.is_from_cluster());
        self.do_notify(&tag, key.clone(), remove_instance);
        if let Some(client_id) = real_client_id {
            if !client_id.as_ref().is_empty() {
                let instance_key =
                    InstanceKey::new_by_service_key(key, instance_id.ip.clone(), instance_id.port);
                self.remove_client_instance_key(&client_id, &instance_key);
            }
        }
        tag
    }

    pub fn update_instance(
        &mut self,
        key: &ServiceKey,
        mut instance: Instance,
        tag: Option<InstanceUpdateTag>,
        from_sync: bool,
    ) -> UpdateInstanceType {
        instance.init();
        //assert!(instance.check_vaild());
        self.create_empty_service(key);
        //let is_from_from_cluster = instance.is_from_cluster();
        let at_process_range = if let Some(range) = &self.current_range {
            range.is_range(get_hash_value(&key) as usize)
        } else {
            false
        };
        if at_process_range && !instance.from_grpc {
            instance.from_cluster = 0;
            instance.client_id = EMPTY_ARC_STRING.clone();
        }
        //let cluster_name = instance.cluster_name.clone();
        let service = if let Some(service) = self.service_map.get_mut(key) {
            service
        } else {
            log::warn!("update_instance not found service,{:?}", &key);
            return UpdateInstanceType::None;
        };
        let client_id = instance.client_id.clone();
        let instance_key =
            InstanceKey::new_by_service_key(key, instance.ip.clone(), instance.port.to_owned());
        if (instance.from_grpc || instance.is_from_cluster()) && !instance.client_id.is_empty() {
            if let Some(set) = self.client_instance_set.get_mut(&client_id) {
                set.insert(instance_key.clone());
            } else {
                let mut set = HashSet::new();
                set.insert(instance_key.clone());
                self.client_instance_set.insert(client_id, set);
            }
        }
        let instance_short_key = instance.get_short_key();

        let (tag, replace_old_client_id) = service.update_instance(instance, tag, from_sync);
        if let UpdateInstanceType::UpdateOtherClusterMetaData(_, _) = &tag {
            return tag;
        }
        if let Some(replace_old_client_id) = replace_old_client_id {
            if let Some(set) = self.client_instance_set.get_mut(&replace_old_client_id) {
                set.remove(&instance_key);
            }
        }
        if !from_sync {
            //change notify
            let instance = service.get_instance(&instance_short_key);
            self.do_notify(&tag, key.clone(), instance);
        }
        tag
    }

    pub(crate) fn remove_client_instance(&mut self, client_id: &Arc<String>) {
        if let Some(keys) = self.client_instance_set.remove(client_id) {
            for instance_key in keys {
                let service_key = instance_key.get_service_key();
                let short_key = instance_key.get_short_key();
                self.remove_instance(&service_key, &short_key, Some(client_id));
            }
        }
    }

    fn remove_client_instance_key(&mut self, client_id: &Arc<String>, key: &InstanceKey) {
        if let Some(keys) = self.client_instance_set.get_mut(client_id) {
            keys.remove(key);
        }
    }

    pub fn get_instance(
        &self,
        key: &ServiceKey,
        instance_id: &InstanceShortKey,
    ) -> Option<Arc<Instance>> {
        if let Some(service) = self.service_map.get(key) {
            service.get_instance(instance_id)
        } else {
            None
        }
    }

    pub fn get_instance_list(
        &self,
        key: &ServiceKey,
        cluster_str: &str,
        only_healthy: bool,
    ) -> Vec<Arc<Instance>> {
        let cluster_names = NamingUtils::split_filters(cluster_str);
        if let Some(service) = self.service_map.get(key) {
            return InstanceFilterUtils::default_instance_filter(
                service.get_instance_list(cluster_names, false, true),
                Some(service.get_metadata()),
                only_healthy,
            );
        }
        vec![]
    }

    pub fn get_instances_and_metadata(
        &self,
        key: &ServiceKey,
        cluster_str: &str,
        only_healthy: bool,
    ) -> (Vec<Arc<Instance>>, Option<ServiceMetadata>) {
        let cluster_names = NamingUtils::split_filters(cluster_str);
        if let Some(service) = self.service_map.get(key) {
            return (
                service.get_instance_list(cluster_names, only_healthy, true),
                Some(service.get_metadata()),
            );
        }
        (vec![], None)
    }

    pub fn get_metadata(&self, key: &ServiceKey) -> Option<ServiceMetadata> {
        self.service_map.get(key).map(|e| e.get_metadata())
    }

    pub fn get_instance_map(
        &self,
        key: &ServiceKey,
        cluster_names: Vec<String>,
        only_healthy: bool,
    ) -> HashMap<String, Vec<Arc<Instance>>> {
        let mut map: HashMap<String, Vec<Arc<Instance>>> = HashMap::new();
        if let Some(service) = self.service_map.get(key) {
            for item in service.get_instance_list(cluster_names, only_healthy, true) {
                if let Some(list) = map.get_mut(&item.cluster_name) {
                    list.push(item)
                } else {
                    map.insert(item.cluster_name.to_owned(), vec![item]);
                }
            }
        }
        map
    }

    pub(crate) fn get_service_info(
        &self,
        key: &ServiceKey,
        cluster_str: String,
        only_healthy: bool,
    ) -> ServiceInfo {
        let (hosts, metadata) = self.get_instances_and_metadata(key, &cluster_str, false);
        let service_info = ServiceInfo {
            name: Some(key.service_name.clone()),
            group_name: Some(key.group_name.clone()),
            cache_millis: 10000i64,
            last_ref_time: now_millis_i64(),
            reach_protection_threshold: false,
            hosts: Some(hosts),
            clusters: Some(cluster_str),
            ..Default::default()
        };
        InstanceFilterUtils::default_service_filter(service_info, metadata, only_healthy)
    }

    pub fn get_instance_list_string(
        &self,
        key: &ServiceKey,
        cluster_str: String,
        only_healthy: bool,
    ) -> String {
        let list = self.get_instance_list(key, &cluster_str, only_healthy);
        QueryListResult::get_instance_list_string(cluster_str, key, list)
    }

    pub fn time_check(&mut self) {
        let current_time = Local::now().timestamp_millis();
        let healthy_time = current_time - self.sys_config.instance_health_timeout_millis;
        let offline_time = current_time - self.sys_config.instance_timeout_millis;
        let mut size = 0;
        let now = now_millis();
        let mut change_list = vec![];
        for item in self.service_map.values_mut() {
            let service_key = item.get_service_key();
            let (rlist, ulist) = item.time_check(healthy_time, offline_time);
            size += rlist.len() + ulist.len();
            if !rlist.is_empty() {
                for short_key in &rlist {
                    if item.exist_priority_metadata(short_key) {
                        let instance_key = InstanceKey::new_by_service_key(
                            &service_key,
                            short_key.ip.clone(),
                            short_key.port,
                        );
                        self.instance_metadate_set.add(
                            now + self.sys_config.instance_metadata_time_out_millis,
                            instance_key,
                        );
                    }
                }
            }
            if item.instance_size <= 0 {
                self.empty_service_set.add(
                    now + self.sys_config.service_time_out_millis,
                    item.get_service_key(),
                );
            }
            change_list.push((service_key, rlist, ulist));
            if size >= self.sys_config.once_time_check_size {
                break;
            }
        }
        for (service_key, rlist, ulist) in change_list {
            self.time_check_notify(service_key, rlist, ulist);
        }
    }

    fn time_check_notify(
        &mut self,
        key: ServiceKey,
        remove_list: Vec<InstanceShortKey>,
        update_list: Vec<InstanceShortKey>,
    ) {
        if !remove_list.is_empty() {
            self.time_check_sync_remove_info_to_cluster(key.clone(), remove_list);
            self.do_notify(&UpdateInstanceType::Remove, key.clone(), None);
        }
        if !update_list.is_empty() {
            self.time_check_sync_update_info_to_cluster(key.clone(), update_list);
            self.do_notify(&UpdateInstanceType::UpdateValue, key, None);
        }
    }

    fn time_check_sync_update_info_to_cluster(
        &self,
        key: ServiceKey,
        update_list: Vec<InstanceShortKey>,
    ) {
        if let (Some(cluster_delay_notify), Some(service)) =
            (&self.cluster_delay_notify, self.service_map.get(&key))
        {
            if service.instances.is_empty() {
                return;
            }
            for instance_key in update_list {
                if let Some(instance) = service.get_instance(&instance_key) {
                    if instance.is_from_cluster() {
                        continue;
                    }
                    cluster_delay_notify
                        .do_send(InstanceDelayNotifyRequest::UpdateInstance(instance));
                }
            }
        }
    }

    fn time_check_sync_remove_info_to_cluster(
        &self,
        key: ServiceKey,
        remove_list: Vec<InstanceShortKey>,
    ) {
        if let Some(cluster_delay_notify) = &self.cluster_delay_notify {
            for instance_key in remove_list {
                let instance = Arc::new(Instance {
                    namespace_id: key.namespace_id.clone(),
                    group_name: key.group_name.clone(),
                    service_name: key.service_name.clone(),
                    ip: instance_key.ip,
                    port: instance_key.port,
                    ..Default::default()
                });
                cluster_delay_notify.do_send(InstanceDelayNotifyRequest::RemoveInstance(instance));
            }
        }
    }

    pub fn get_service_list(
        &self,
        page_size: usize,
        page_index: usize,
        key: &ServiceKey,
    ) -> (usize, Vec<Arc<String>>) {
        let offset = if page_index == 0 {
            0
        } else {
            page_size * (page_index - 1)
        };
        let param = ServiceQueryParam {
            offset,
            limit: page_size,
            namespace_id: Some(key.namespace_id.clone()),
            group: Some(key.group_name.clone()),
            ..Default::default()
        };
        let (size, list) = self.namespace_index.query_service_page(&param);
        let service_names = list.into_iter().map(|e| e.service_name).collect::<Vec<_>>();
        (size, service_names)
    }

    pub fn get_subscribers_list(
        &self,
        page_size: usize,
        page_index: usize,
        key: &ServiceKey,
    ) -> (usize, Vec<Arc<SubscriberInfoDto>>) {
        let mut ret = Vec::new();

        let res = self.subscriber.fuzzy_match_listener(
            &key.group_name,
            &key.service_name,
            &key.namespace_id,
        );

        for (service_key, val) in res {
            for (ip_port, _) in val {
                let parts: Vec<&str> = ip_port.split(':').collect();
                if parts.len() == 2 {
                    if let Ok(port) = parts[1].parse::<u16>() {
                        let subscriber_info = SubscriberInfoDto {
                            service_name: service_key.service_name.clone(),
                            group_name: service_key.group_name.clone(),
                            namespace_id: service_key.namespace_id.clone(),
                            ip: Arc::new(parts[0].to_string()),
                            port,
                        };

                        ret.push(Arc::new(subscriber_info));
                    }
                }
            }
        }

        let total = ret.len();
        let start = (page_index - 1) * page_size;
        ret.sort_by(|a, b| {
            a.service_name
                .cmp(&b.service_name)
                .then(a.group_name.cmp(&b.group_name))
                .then(a.ip.cmp(&b.ip))
                .then(a.port.cmp(&b.port))
        });
        let paginated_result = ret
            .into_iter()
            .skip(start)
            .take(page_size)
            .collect::<Vec<_>>();

        (total, paginated_result)
    }

    pub fn get_service_info_page(&self, param: ServiceQueryParam) -> (usize, Vec<ServiceInfoDto>) {
        let (size, list) = self.namespace_index.query_service_page(&param);

        if size == 0 {
            return (0, Vec::new());
        }

        let mut info_list = Vec::with_capacity(list.len());
        for item in &list {
            if let Some(service) = self.service_map.get(item) {
                info_list.push(service.get_service_info());
            }
        }
        (size, info_list)
    }

    fn update_listener(
        &mut self,
        key: &ServiceKey,
        cluster_names: &[String],
        addr: SocketAddr,
        only_healthy: bool,
    ) {
        if let Some(listener_addr) = self.listener_addr.as_ref() {
            let item = ListenerItem::new(cluster_names.to_owned(), only_healthy, addr);
            let msg = NamingListenerCmd::Add(key.clone(), item);
            listener_addr.do_send(msg);
        }
    }

    fn clear_empty_service(&mut self) {
        //println!("clear_empty_service");
        let now = now_millis();
        for service_map_key in self.empty_service_set.timeout(now) {
            self.clear_one_empty_service(service_map_key, now)
        }
    }

    fn clear_one_empty_service(&mut self, service_map_key: ServiceKey, now: u64) {
        if let Some(service) = self.service_map.get(&service_map_key) {
            if service.instance_size <= 0
                && now - self.sys_config.service_time_out_millis >= service.last_empty_times
            {
                //self.dal_addr.do_send(ServiceDalMsg::DeleteService(service.get_service_do().get_key_param().unwrap()));
                self.namespace_index
                    .remove_service(&service.get_service_key());
                self.service_map.remove(&service_map_key);
                log::info!("clear_empty_service:{:?}", &service_map_key);
            }
        }
    }

    fn clear_timeout_instance_metadata(&mut self) {
        for instance_key in self.instance_metadate_set.timeout(now_millis()) {
            self.clear_one_timeout_instance_metadata(instance_key);
        }
    }

    fn clear_one_timeout_instance_metadata(&mut self, instance_key: InstanceKey) {
        let service_key = instance_key.get_service_key();
        if let Some(service) = self.service_map.get_mut(&service_key) {
            let short_key = instance_key.get_short_key();
            if !service.instances.contains_key(&short_key) {
                service.instance_metadata_map.remove(&short_key);
            }
        }
    }

    pub fn instance_time_out_heartbeat(&self, ctx: &mut actix::Context<Self>) {
        ctx.run_later(Duration::from_millis(2000), |act, ctx| {
            act.clear_empty_service();
            act.clear_timeout_instance_metadata();
            let addr = ctx.address();
            addr.do_send(NamingCmd::PeekListenerTimeout);
            act.instance_time_out_heartbeat(ctx);
        });
    }

    ///
    /// 构建当前管理服务实例的镜像数据包,用于同步给其它集群实例
    pub fn build_snapshot_data(&self, ranges: Vec<ProcessRange>) -> SnapshotForSend {
        let mut service_details = vec![];
        let mut instances = vec![];
        for (service_key, service) in &self.service_map {
            let hash_value = get_hash_value(service_key) as usize;
            if !ProcessRange::is_range_at_list(hash_value, &ranges) {
                continue;
            }
            service_details.push(service.get_service_detail());
            instances.append(&mut service.get_owner_http_instances());
        }

        for keys in self.client_instance_set.values() {
            for instance_key in keys {
                let service_key = instance_key.get_service_key();
                let short_key = instance_key.get_short_key();
                if let Some(v) = self.get_instance(&service_key, &short_key) {
                    if !v.is_from_cluster() {
                        instances.push(v)
                    }
                }
            }
        }

        SnapshotForSend {
            route_index: 0,
            node_count: 0,
            services: service_details,
            instances,
            mode: 0,
        }
    }

    fn query_grpc_distro_data(&self) -> HashMap<Arc<String>, HashSet<InstanceKey>> {
        let mut client_data: HashMap<Arc<String>, HashSet<InstanceKey>> = HashMap::new();
        let client_id_pre = format!("{}_", &self.node_id);
        for (client_id, keys) in &self.client_instance_set {
            if !client_id.as_ref().starts_with(&client_id_pre) {
                //不是本节点的grpc client直接跳过
                continue;
            }
            client_data.insert(client_id.clone(), keys.clone());
        }
        client_data
    }
    fn diff_grpc_distro_data(
        &self,
        cluster_id: u64,
        data: HashMap<ServiceKey, u64>,
    ) -> HashMap<ServiceKey, i64> {
        let mut result = HashMap::new();
        for (service_key, count) in data.iter() {
            let mut i = *count as i64;
            if let Some(v) = self.service_map.get(service_key) {
                for item in v.instances.values() {
                    if item.from_cluster == cluster_id {
                        i -= 1;
                    }
                }
            }
            if i != 0 {
                log::info!("diff_grpc_distro_data:{:?},{}", service_key, i);
                result.insert(service_key.clone(), i);
            }
        }
        result
    }

    fn diff_grpc_distro_client_data(
        &mut self,
        cluster_id: u64,
        data: HashMap<Arc<String>, HashSet<InstanceKey>>,
    ) -> Vec<InstanceKey> {
        let mut remove_keys = vec![];
        let mut new_items: Vec<InstanceKey> = vec![];
        for (client_id, instances) in data {
            if let Some(v) = self.client_instance_set.get(&client_id) {
                for item in v.difference(&instances) {
                    remove_keys.push((item.get_service_key(), item.get_short_key()));
                }
                for item in instances.difference(v).into_iter() {
                    new_items.push(item.clone());
                }
            } else {
                for item in instances {
                    new_items.push(item);
                }
            }
        }
        for (service_key, client_key) in remove_keys {
            self.remove_instance(&service_key, &client_key, None);
        }
        new_items
    }

    fn build_distro_service_instance(&self, service_keys: Vec<ServiceKey>) -> Vec<Arc<Instance>> {
        let mut instances = vec![];
        instances
    }

    fn build_distro_instances(&self, instance_keys: Vec<InstanceKey>) -> Vec<Arc<Instance>> {
        let mut instances = vec![];
        for instance_key in instance_keys {
            let service_key = instance_key.get_service_key();
            let short_key = instance_key.get_short_key();
            if let Some(v) = self.get_instance(&service_key, &short_key) {
                if !v.is_from_cluster() {
                    instances.push(v)
                }
            }
        }
        instances
    }

    fn receive_diff_service_instance(&mut self, diff_data: SnapshotForReceive, from_cluster: u64) {
        let mut service_instance_map: HashMap<ServiceKey, HashMap<InstanceShortKey, Instance>> =
            HashMap::new();
        for instance in diff_data.instances {
            let service_key = instance.get_service_key();
            if let Some(v) = service_instance_map.get_mut(&service_key) {
                v.insert(instance.get_short_key(), instance);
            } else {
                service_instance_map.insert(
                    service_key,
                    HashMap::from([(instance.get_short_key(), instance)]),
                );
            }
        }
        let mut delete_keys = vec![];
        for (service_key, instance_map) in service_instance_map {
            let service_detail = ServiceDetailDto {
                namespace_id: service_key.namespace_id.clone(),
                service_name: service_key.service_name.clone(),
                group_name: service_key.group_name.clone(),
                metadata: None,
                protect_threshold: None,
                grpc_instance_count: None,
            };
            self.update_service(service_detail);
            if let Some(service) = self.service_map.get_mut(&service_key) {
                for (key, instance) in &service.instances {
                    if instance.from_cluster == from_cluster
                        && instance.from_grpc
                        && !instance_map.contains_key(key)
                    {
                        delete_keys.push((service_key.clone(), key.clone()));
                    }
                }
                for (short_key, instance) in instance_map {
                    service.update_instance(instance, None, true);
                }
            }
        }
        for (service_key, short_key) in delete_keys {
            self.remove_instance(&service_key, &short_key, None);
        }
    }

    ///
    /// 刷新服务管理范围，在集群节点有变化时触发
    /// 重新管理更新后的临时实例生命周期
    fn refresh_process_range(&mut self, range: ProcessRange) -> anyhow::Result<()> {
        for (service_key, service) in &mut self.service_map {
            let hash_value = get_hash_value(service_key) as usize;
            if !range.is_range(hash_value) {
                continue;
            }
            service.do_refresh_process_range();
        }
        self.current_range = Some(range);
        Ok(())
    }

    fn receive_snapshot(&mut self, snapshot: SnapshotForReceive) {
        for service_detail in snapshot.services {
            self.update_service(service_detail);
        }
        for mut instance in snapshot.instances {
            self.update_instance(&instance.get_service_key(), instance, None, true);
        }
    }

    fn notify_cluster_remove_client_id(&mut self, client_id: Arc<String>) {
        if let Some(node_manage) = self.cluster_node_manage.as_ref() {
            let req = NamingRouteRequest::RemoveClientId {
                client_id: client_id.clone(),
            };
            node_manage.do_send(NodeManageRequest::SendToOtherNodes(req));
            node_manage.do_send(NodeManageRequest::RemoveClientId(client_id));
        }
    }

    pub(crate) fn get_instance_size(&self) -> usize {
        let mut sum = 0;
        for service in self.service_map.values() {
            sum += service.instances.len();
        }
        sum
    }

    pub(crate) fn get_client_instance_set_item_size(&self) -> usize {
        let mut sum = 0;
        for set in self.client_instance_set.values() {
            sum += set.len();
        }
        sum
    }

    pub(crate) fn get_healthy_timeout_set_size(&self) -> usize {
        let mut sum = 0;
        for service in self.service_map.values() {
            sum += service.healthy_timeout_set.len();
        }
        sum
    }
    pub(crate) fn get_healthy_timeout_set_item_size(&self) -> usize {
        let mut sum = 0;
        for service in self.service_map.values() {
            sum += service.get_healthy_timeout_set_item_size();
        }
        sum
    }

    pub(crate) fn get_unhealthy_timeout_set_size(&self) -> usize {
        let mut sum = 0;
        for service in self.service_map.values() {
            sum += service.unhealthy_timeout_set.len();
        }
        sum
    }

    pub(crate) fn get_unhealthy_timeout_set_item_size(&self) -> usize {
        let mut sum = 0;
        for service in self.service_map.values() {
            sum += service.get_unhealthy_timeout_set_item_size();
        }
        sum
    }
}

#[derive(Debug, Message)]
#[rtype(result = "anyhow::Result<NamingResult>")]
pub enum NamingCmd {
    Update(Instance, Option<InstanceUpdateTag>),
    UpdateFromSync(Instance, Option<InstanceUpdateTag>),
    UpdateBatch(Vec<Instance>),
    Delete(Instance),
    DeleteBatch(Vec<Instance>),
    Query(Instance),
    QueryList(ServiceKey, String, bool, Option<SocketAddr>),
    QueryAllInstanceList(ServiceKey),
    QueryListString(ServiceKey, String, bool, Option<SocketAddr>),
    QueryServiceInfo(ServiceKey, String, bool),
    QueryServicePage(ServiceKey, usize, usize),
    QueryServiceSubscribersPage(ServiceKey, usize, usize),
    //查询服务实际信息列表
    QueryServiceInfoPage(ServiceQueryParam),
    //CreateService(ServiceDetailDto),
    UpdateService(ServiceDetailDto),
    UpdateServiceFromCluster(ServiceDetailDto),
    RemoveService(ServiceKey),
    PeekListenerTimeout,
    NotifyListener(ServiceKey, u64),
    Subscribe(Vec<NamingListenerItem>, Arc<String>),
    RemoveSubscribe(Vec<NamingListenerItem>, Arc<String>),
    RemoveClient(Arc<String>),
    RemoveClientsFromCluster(Vec<Arc<String>>),
    RemoveClientFromCluster(Arc<String>),
    QueryClientInstanceCount,
    QueryDalAddr,
    QuerySnapshot(Vec<ProcessRange>),
    ClusterRefreshProcessRange(ProcessRange),
    ReceiveSnapshot(SnapshotForReceive),
    QueryGrpcDistroData,
    DiffGrpcDistroData {
        cluster_id: u64,
        data: DistroData,
    },
    #[deprecated]
    QueryDistroServerSnapshot(Vec<ServiceKey>),
    QueryDistroInstanceSnapshot(Vec<InstanceKey>),
    #[deprecated]
    ReceiveDiffServiceInstance(u64, SnapshotForReceive),
}

pub enum NamingResult {
    NULL,
    Instance(Arc<Instance>),
    InstanceList(Vec<Arc<Instance>>),
    InstanceListString(String),
    ServiceInfo(ServiceInfo),
    ServicePage((usize, Vec<Arc<String>>)),
    ServiceSubscribersPage((usize, Vec<Arc<SubscriberInfoDto>>)),
    ServiceInfoPage((usize, Vec<ServiceInfoDto>)),
    ClientInstanceCount(Vec<(Arc<String>, usize)>),
    RewriteToCluster(u64, Instance),
    Snapshot(SnapshotForSend),
    GrpcDistroData(DistroData),
    DiffDistroData(DistroData),
    DistroInstancesSnapshot(Vec<Arc<Instance>>),
}

impl Supervised for NamingActor {
    fn restarting(&mut self, _ctx: &mut <Self as Actor>::Context) {
        log::warn!("NamingActor restart ...");
    }
}

impl Handler<NamingCmd> for NamingActor {
    type Result = anyhow::Result<NamingResult>;

    fn handle(&mut self, msg: NamingCmd, ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            NamingCmd::Update(instance, tag) => {
                let tag = self.update_instance(&instance.get_service_key(), instance, tag, false);
                if let UpdateInstanceType::UpdateOtherClusterMetaData(node_id, instance) = tag {
                    Ok(NamingResult::RewriteToCluster(node_id, instance))
                } else {
                    Ok(NamingResult::NULL)
                }
            }
            NamingCmd::UpdateFromSync(instance, tag) => {
                let tag = self.update_instance(&instance.get_service_key(), instance, tag, true);
                if let UpdateInstanceType::UpdateOtherClusterMetaData(node_id, instance) = tag {
                    Ok(NamingResult::RewriteToCluster(node_id, instance))
                } else {
                    Ok(NamingResult::NULL)
                }
            }
            NamingCmd::UpdateBatch(instances) => {
                for mut instance in instances {
                    self.update_instance(&instance.get_service_key(), instance, None, true);
                }
                Ok(NamingResult::NULL)
            }
            NamingCmd::Delete(instance) => {
                self.remove_instance(
                    &instance.get_service_key(),
                    &instance.get_short_key(),
                    Some(&instance.client_id),
                );
                Ok(NamingResult::NULL)
            }
            NamingCmd::DeleteBatch(instances) => {
                for instance in instances {
                    self.remove_instance(
                        &instance.get_service_key(),
                        &instance.get_short_key(),
                        Some(&instance.client_id),
                    );
                }
                Ok(NamingResult::NULL)
            }
            NamingCmd::Query(instance) => {
                if let Some(i) =
                    self.get_instance(&instance.get_service_key(), &instance.get_short_key())
                {
                    return Ok(NamingResult::Instance(i));
                }
                Ok(NamingResult::NULL)
            }
            NamingCmd::QueryList(service_key, cluster_str, only_healthy, addr) => {
                let cluster_names = NamingUtils::split_filters(&cluster_str);
                if let Some(addr) = addr {
                    self.update_listener(&service_key, &cluster_names, addr, only_healthy);
                }
                let list = self.get_instance_list(&service_key, &cluster_str, only_healthy);
                Ok(NamingResult::InstanceList(list))
            }
            NamingCmd::QueryListString(service_key, cluster_str, only_healthy, addr) => {
                //println!("QUERY_LIST_STRING addr: {:?}",&addr);
                let cluster_names = NamingUtils::split_filters(&cluster_str);
                if let Some(addr) = addr {
                    self.update_listener(&service_key, &cluster_names, addr, only_healthy);
                }
                let data = self.get_instance_list_string(&service_key, cluster_str, only_healthy);
                Ok(NamingResult::InstanceListString(data))
            }
            NamingCmd::QueryServiceInfo(service_key, cluster_str, only_healthy) => {
                let cluster_names = NamingUtils::split_filters(&cluster_str);
                let service_info = self.get_service_info(&service_key, cluster_str, only_healthy);
                Ok(NamingResult::ServiceInfo(service_info))
            }
            NamingCmd::QueryServicePage(service_key, page_size, page_index) => {
                Ok(NamingResult::ServicePage(self.get_service_list(
                    page_size,
                    page_index,
                    &service_key,
                )))
            }
            NamingCmd::QueryServiceSubscribersPage(service_key, page_size, page_index) => {
                Ok(NamingResult::ServiceSubscribersPage(
                    self.get_subscribers_list(page_size, page_index, &service_key),
                ))
            }
            NamingCmd::QueryServiceInfoPage(param) => Ok(NamingResult::ServiceInfoPage(
                self.get_service_info_page(param),
            )),
            NamingCmd::PeekListenerTimeout => {
                self.time_check();
                //self.notify_check();
                Ok(NamingResult::NULL)
            }
            NamingCmd::NotifyListener(service_key, id) => {
                if let Some(listener_addr) = self.listener_addr.as_ref() {
                    let map = self.get_instance_map(&service_key, vec![], false);
                    //notify listener
                    let msg = NamingListenerCmd::Notify(service_key, "".to_string(), map, id);
                    listener_addr.do_send(msg);
                }
                Ok(NamingResult::NULL)
            }
            NamingCmd::Subscribe(items, client_id) => {
                self.subscriber.add_subscribe(client_id, items.clone());
                //debug
                for item in items {
                    self.subscriber.notify(item.service_key);
                }
                Ok(NamingResult::NULL)
            }
            NamingCmd::RemoveSubscribe(items, client_id) => {
                self.subscriber.remove_subscribe(client_id, items);
                Ok(NamingResult::NULL)
            }
            NamingCmd::RemoveClient(client_id) => {
                self.subscriber.remove_client_subscribe(client_id.clone());
                self.remove_client_instance(&client_id);
                self.notify_cluster_remove_client_id(client_id);
                Ok(NamingResult::NULL)
            }
            NamingCmd::RemoveClientsFromCluster(client_ids) => {
                for client_id in client_ids {
                    self.subscriber.remove_client_subscribe(client_id.clone());
                    self.remove_client_instance(&client_id);
                }
                Ok(NamingResult::NULL)
            }
            NamingCmd::RemoveClientFromCluster(client_id) => {
                self.subscriber.remove_client_subscribe(client_id.clone());
                self.remove_client_instance(&client_id);
                Ok(NamingResult::NULL)
            }
            NamingCmd::QueryDalAddr => {
                //Ok(NamingResult::DalAddr(self.dal_addr.clone()))
                Ok(NamingResult::NULL)
            }
            NamingCmd::UpdateServiceFromCluster(service_info) => {
                //来源于集群的更新不再通知其它节点
                self.update_service(service_info);
                Ok(NamingResult::NULL)
            }
            NamingCmd::UpdateService(service_info) => {
                self.update_service(service_info.clone());
                if let Some(node_manage) = self.cluster_node_manage.as_ref() {
                    //来源于客户端的变更通知其它节点
                    node_manage.do_send(NodeManageRequest::SendToOtherNodes(
                        NamingRouteRequest::SyncUpdateService {
                            service: service_info,
                        },
                    ));
                }
                Ok(NamingResult::NULL)
            }
            NamingCmd::RemoveService(service_key) => {
                self.remove_empty_service(service_key)?;
                Ok(NamingResult::NULL)
            }
            NamingCmd::QueryAllInstanceList(key) => {
                if let Some(service) = self.service_map.get(&key) {
                    Ok(NamingResult::InstanceList(service.get_instance_list(
                        vec![],
                        false,
                        false,
                    )))
                } else {
                    Ok(NamingResult::InstanceList(vec![]))
                }
            }
            NamingCmd::QueryClientInstanceCount => {
                let mut client_instance_count = Vec::with_capacity(self.client_instance_set.len());
                for (k, v) in &self.client_instance_set {
                    client_instance_count.push((k.clone(), v.len()));
                }
                Ok(NamingResult::ClientInstanceCount(client_instance_count))
            }
            NamingCmd::QuerySnapshot(ranges) => {
                let res = self.build_snapshot_data(ranges);
                Ok(NamingResult::Snapshot(res))
            }
            NamingCmd::ClusterRefreshProcessRange(range) => {
                self.refresh_process_range(range)?;
                Ok(NamingResult::NULL)
            }
            NamingCmd::ReceiveSnapshot(snapshot) => {
                self.receive_snapshot(snapshot);
                if let Some(range) = &self.current_range {
                    self.refresh_process_range(range.clone()).ok();
                }
                Ok(NamingResult::NULL)
            }
            NamingCmd::QueryGrpcDistroData => {
                let res = self.query_grpc_distro_data();
                Ok(NamingResult::GrpcDistroData(DistroData::ClientInstances(
                    res,
                )))
            }
            NamingCmd::DiffGrpcDistroData { data, cluster_id } => {
                if let DistroData::ClientInstances(data) = data {
                    let res = self.diff_grpc_distro_client_data(cluster_id, data);
                    Ok(NamingResult::DiffDistroData(
                        DistroData::DiffClientInstances(res),
                    ))
                } else {
                    Ok(NamingResult::NULL)
                }
            }
            NamingCmd::QueryDistroServerSnapshot(service_keys) => {
                let instances = self.build_distro_service_instance(service_keys);
                Ok(NamingResult::DistroInstancesSnapshot(instances))
            }
            NamingCmd::QueryDistroInstanceSnapshot(instance_keys) => {
                let instances = self.build_distro_instances(instance_keys);
                Ok(NamingResult::DistroInstancesSnapshot(instances))
            }
            NamingCmd::ReceiveDiffServiceInstance(from_cluster, diff_data) => {
                self.receive_diff_service_instance(diff_data, from_cluster);
                Ok(NamingResult::NULL)
            }
        }
    }
}

#[actix_rt::test]
async fn query_healthy_instances() {
    use super::*;
    use tokio::net::UdpSocket;
    //let listener_addr = InnerNamingListener::new_and_create(5000, None);
    let mut naming = NamingActor::new();
    let mut instance = Instance::new("127.0.0.1".to_owned(), 8080);
    instance.namespace_id = Arc::new("public".to_owned());
    instance.service_name = Arc::new("foo".to_owned());
    instance.group_name = Arc::new("DEFUALT".to_owned());
    instance.cluster_name = "DEFUALT".to_owned();
    instance.init();
    let key = instance.get_service_key();
    naming.update_instance(&key, instance, None, false);
    if let Some(service) = naming.service_map.get_mut(&key) {
        service.protect_threshold = 0.1;
    }

    println!("-------------");
    let items = naming.get_instance_list(&key, "", true);
    assert!(!items.is_empty());
    println!("DEFUALT list:{}", serde_json::to_string(&items).unwrap());
    let items = naming.get_instance_list(&key, "", true);
    assert!(!items.is_empty());
    println!(
        "empty cluster list:{}",
        serde_json::to_string(&items).unwrap()
    );
    tokio::time::sleep(Duration::from_millis(16000)).await;
    naming.time_check();
    println!("-------------");
    let items = naming.get_instance_list(&key, "", false);
    assert!(!items.is_empty());
    println!(
        "empty cluster list:{}",
        serde_json::to_string(&items).unwrap()
    );
    tokio::time::sleep(Duration::from_millis(16000)).await;
    naming.time_check();
    println!("-------------");
    let items = naming.get_instance_list(&key, "", false);
    assert!(items.is_empty());
    println!(
        "empty cluster list:{}",
        serde_json::to_string(&items).unwrap()
    );
}

#[test]
fn test_add_service() {
    let mut naming = NamingActor::new();
    let service_key = ServiceKey::new("1", "1", "1");
    let service_info = ServiceDetailDto {
        namespace_id: service_key.namespace_id.clone(),
        service_name: service_key.service_name.clone(),
        group_name: service_key.group_name.clone(),
        metadata: Default::default(),
        protect_threshold: Some(0.5),
        ..Default::default()
    };
    assert!(naming.namespace_index.service_size == 0);
    naming.update_service(service_info);
    assert!(naming.namespace_index.service_size == 1);
    naming.remove_empty_service(service_key).unwrap();
    assert!(naming.namespace_index.service_size == 0);
}

#[test]
fn test_remove_has_instance_service() {
    let mut naming = NamingActor::new();
    let mut instance = Instance::new("127.0.0.1".to_owned(), 8080);
    instance.namespace_id = Arc::new("public".to_owned());
    instance.service_name = Arc::new("foo".to_owned());
    instance.group_name = Arc::new("DEFUALT".to_owned());
    instance.cluster_name = "DEFUALT".to_owned();
    instance.init();
    let service_key = instance.get_service_key();
    naming.update_instance(&service_key, instance.clone(), None, false);
    let service_info = ServiceDetailDto {
        namespace_id: service_key.namespace_id.clone(),
        service_name: service_key.service_name.clone(),
        group_name: service_key.group_name.clone(),
        metadata: Default::default(),
        protect_threshold: Some(0.5),
        ..Default::default()
    };
    assert!(naming.namespace_index.service_size == 1);
    naming.update_service(service_info);
    assert!(naming.namespace_index.service_size == 1);
    assert!(naming.remove_empty_service(service_key.clone()).is_err());
    assert!(naming.namespace_index.service_size == 1);

    naming.remove_instance(&service_key, &instance.get_short_key(), None);
    assert!(naming.namespace_index.service_size == 1);
    assert!(naming.remove_empty_service(service_key.clone()).is_ok());
    assert!(naming.namespace_index.service_size == 0);
}
