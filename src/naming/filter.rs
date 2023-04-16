use std::{sync::Arc};

use tonic::metadata;

use super::{model::{ServiceInfo, Instance}, service::ServiceMetadata};


pub(crate) struct InstanceFilterUtils;


impl InstanceFilterUtils {
    pub fn default_instance_filter(all_instances:Vec<Arc<Instance>>,metadata:Option<ServiceMetadata>) -> Vec<Arc<Instance>>{
        if let Some(metadata) = metadata {
            let original_total = all_instances.len();
            // all_instances = from metadata select
            let mut healthy_count = 0;
            for item in &all_instances {
                if item.healthy {
                    healthy_count+=1;
                }
            }
            let threshold = if metadata.protect_threshold <= 0f32 {0f32} else {metadata.protect_threshold};
            if (healthy_count as f32)/original_total as f32 <= threshold {
                let instances = all_instances.iter().map(|i|{
                    if !i.healthy {
                        let mut raw = i.as_ref().clone();
                        raw.healthy=true;
                        Arc::new(raw)
                    }
                    else{
                        i.clone()
                    }
                }).collect();
                return instances;
            }
        };
        all_instances
    }

    pub fn default_service_filter(mut service_info:ServiceInfo,metadata:Option<ServiceMetadata>) -> ServiceInfo{
        if let (Some(all_instances),Some(metadata)) = (service_info.hosts.as_ref(),metadata) {
            let original_total = all_instances.len();
            // all_instances = from metadata select
            let mut healthy_count = 0;
            for item in all_instances {
                if item.healthy {
                    healthy_count+=1;
                }
            }
            let threshold = if metadata.protect_threshold <= 0f32 {0f32} else {metadata.protect_threshold};
            if (healthy_count as f32)/original_total as f32 <= threshold {
                service_info.reach_protection_threshold = true;
                let instances = all_instances.iter().map(|i|{
                    if !i.healthy {
                        let mut raw = i.as_ref().clone();
                        raw.healthy=true;
                        Arc::new(raw)
                    }
                    else{
                        i.clone()
                    }
                }).collect();
                service_info.hosts = Some(instances);
            }
        }
        service_info
    }
}