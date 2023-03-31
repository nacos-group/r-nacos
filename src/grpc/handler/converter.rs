use std::collections::HashSet;
use std::sync::Arc;

use crate::grpc::api_model::{ServiceInfo as ApiServiceInfo,Instance as ApiInstance};
use crate::naming::NamingUtils;
use crate::naming::model::{ServiceInfo,Instance};




#[derive(Debug,Default)]
pub(crate) struct ModelConverter;

impl ModelConverter {
    pub fn to_api_service_info(info:ServiceInfo) -> ApiServiceInfo {
        let hosts=info.hosts.into_iter().map(|e|Self::to_api_instance(e)).collect::<Vec<_>>();
        ApiServiceInfo{
            name:info.name,
            group_name:info.group_name,
            clusters:info.clusters,
            cache_millis:info.cache_millis,
            last_ref_time:info.last_ref_time,
            checksum:info.checksum,
            all_ips:info.all_ips,
            reach_protection_threshold:info.reach_protection_threshold,
            hosts : Some(hosts),
        }
    }

    pub fn to_api_instance(instance:Instance) -> ApiInstance {
        /* 
        let service_name = NamingUtils::get_group_and_service_name(
            &instance.service_name,
            &instance.group_name,
        );
        */
        let service_name = instance.service_name;
        ApiInstance{
            instance_id:Some(instance.id),
            ip: Some(instance.ip),
            port: instance.port,
            weight: instance.weight,
            healthy: instance.healthy,
            enabled: instance.enabled,
            ephemeral: instance.ephemeral,
            cluster_name: Some(instance.cluster_name),
            service_name: Some(service_name),
            metadata: instance.metadata,
            ..Default::default()
        }
    }

    pub fn arc_to_api_service_info(info:Arc<ServiceInfo>,cluster_filter:Option<HashSet<String>>) -> ApiServiceInfo {
        let hosts = 
        if let Some(cluster_filter) = cluster_filter {
            let mut hosts = vec![];
            for e in &info.hosts {
                if cluster_filter.contains(&e.cluster_name) {
                    hosts.push(Self::to_api_instance(e.to_owned()));
                }
            }
            hosts
        }
        else{
            info.hosts.clone().into_iter().map(|e|Self::to_api_instance(e)).collect::<Vec<_>>()
        };
        //let hosts=info.hosts.clone().into_iter().map(|e|self.to_api_instance(e)).collect::<Vec<_>>();
        ApiServiceInfo{
            name:info.name.to_owned(),
            group_name:info.group_name.to_owned(),
            clusters:info.clusters.to_owned(),
            cache_millis:info.cache_millis,
            last_ref_time:info.last_ref_time,
            checksum:info.checksum,
            all_ips:info.all_ips,
            reach_protection_threshold:info.reach_protection_threshold,
            hosts : Some(hosts),
        }
    }
}