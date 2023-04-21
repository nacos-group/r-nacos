use std::{sync::Arc, collections::{HashMap, HashSet}};

#[derive(Debug,Clone,Default)]
pub struct ServiceIndex {
    //pub namespace_id:Arc<String>,
    pub group_service: HashMap<Arc<String>,HashSet<Arc<String>>>,
}

impl ServiceIndex {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn insert_service(&mut self,group:Arc<String>,service:Arc<String>) {
        if let Some(set) = self.group_service.get_mut(&group) {
            set.insert(service);
            self.group_service.insert(group, set);
        }
        else{
            let mut set = HashSet::new();
            set.insert(service);
            self.group_service.insert(group, set);
        }
    }

    pub fn remove_service(&mut self,group:Arc<String>,service:Arc<String>) -> usize {
        let group_size=if let Some(set) = self.group_service.get_mut(&group) {
            set.remove(&service);
            set.len()
        }
        else{
            0
        };
        if group_size==0 {
            self.group_service.remove(&group)
        }
        self.group_service.len()
    }

}

pub struct NamespaceIndex {
    pub namespace_group: HashMap<Arc<String>,ServiceIndex>,
}

impl NamespaceIndex {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn insert_service(&mut self,namespaceId:Arc<String>,group:Arc<String>,service:Arc<String>) {
        if let Some(service_index) = self.namespace_group.get_mut(&namespaceId) {
            service_index.insert_service(group, service);
        }
        else{
            let mut service_index = ServiceIndex::new();
            service_index.insert_service(group, service);
            self.namespace_group.insert(namespaceId, service_index);
        }
    }

    pub fn remove_service(&mut self,namespaceId:Arc<String>,group:Arc<String>,service:Arc<String>) {
        if let Some(service_index) = self.namespace_group.get_mut(&namespaceId) {
            let group_size = service_index.remove_service(group, service);
            if group_size == 0 {
                self.namespace_group.remove(&namespaceId);
            }
        }
    }

}


