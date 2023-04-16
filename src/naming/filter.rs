use std::{sync::Arc};

use super::{model::{ServiceInfo, Instance}, service::ServiceMetadata};


pub(crate) struct InstanceFilterUtils;


impl InstanceFilterUtils {
    pub fn default_filter(mut serivce_info:ServiceInfo,metadata:ServiceMetadata) -> ServiceInfo{
        if let Some(all_instances) = serivce_info.hosts.as_ref() {
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
                serivce_info.reach_protection_threshold = true;
                let instances = all_instances.iter().map(|i|{
                    if !i.healthy {
                        let raw = i.as_ref().clone();
                        Arc::new(raw)
                    }
                    else{
                        i.clone()
                    }
                }).collect();
                serivce_info.hosts = Some(instances);
            }
        }
        serivce_info
    }
}