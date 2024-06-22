use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use crate::common::string_utils::StringUtils;
use crate::config::core::ConfigKey;

#[derive(Debug, Clone, Default)]
pub struct ConfigQueryParam {
    pub tenant: Option<Arc<String>>,
    pub group: Option<Arc<String>>,
    pub data_id: Option<Arc<String>>,
    pub like_group: Option<String>,
    pub like_data_id: Option<String>,
    pub query_context: bool,
    pub offset: usize,
    pub limit: usize,
}

impl ConfigQueryParam {
    pub fn match_group(&self, g: &Arc<String>) -> bool {
        if let Some(group) = &self.group {
            group.is_empty() || StringUtils::eq(g, group)
        } else if let Some(like_group) = &self.like_group {
            like_group.is_empty() || StringUtils::like(g, like_group).is_some()
        } else {
            true
        }
    }
    pub fn match_data_id(&self, s: &Arc<String>) -> bool {
        if let Some(data_id) = &self.data_id {
            data_id.is_empty() || StringUtils::eq(s, data_id)
        } else if let Some(like_data_id) = &self.like_data_id {
            like_data_id.is_empty() || StringUtils::like(s, like_data_id).is_some()
        } else {
            true
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConfigIndex {
    pub(crate) group_data: BTreeMap<Arc<String>, BTreeSet<Arc<String>>>,
}

impl ConfigIndex {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    pub(crate) fn insert_config(&mut self, group: Arc<String>, config: Arc<String>) -> bool {
        if let Some(set) = self.group_data.get_mut(&group) {
            if set.insert(config) {
                return true;
            }
        } else {
            let mut set = BTreeSet::new();
            set.insert(config);
            self.group_data.insert(group, set);
            return true;
        }
        false
    }

    pub(crate) fn remove_config(
        &mut self,
        group: &Arc<String>,
        config: &Arc<String>,
    ) -> (bool, usize) {
        let b = if let Some(set) = self.group_data.get_mut(group) {
            let b = set.remove(config);
            if b && set.is_empty() {
                self.group_data.remove(group);
            }
            b
        } else {
            false
        };
        (b, self.group_data.len())
    }

    pub(crate) fn query_config_page(
        &self,
        tenant: &Arc<String>,
        limit: usize,
        param: &ConfigQueryParam,
    ) -> (usize, Vec<ConfigKey>) {
        let mut rlist = vec![];
        let end_index = param.offset + limit;
        let mut index = 0;
        for (g, set) in &self.group_data {
            if param.match_group(g) {
                for s in set {
                    if param.match_data_id(s) {
                        if index >= param.offset && index < end_index {
                            let key = ConfigKey::new_by_arc(s.clone(), g.clone(), tenant.clone());
                            rlist.push(key);
                        }
                        index += 1;
                    }
                }
            }
        }
        (index, rlist)
    }

    pub fn get_config_count(&self) -> usize {
        let mut sum = 0;
        for set in self.group_data.values() {
            sum += set.len();
        }
        sum
    }
}

#[derive(Debug, Clone, Default)]
pub struct TenantIndex {
    pub tenant_group: BTreeMap<Arc<String>, ConfigIndex>,
    pub size: usize,
}

impl TenantIndex {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn insert_config(&mut self, key: ConfigKey) -> bool {
        self.do_insert_config(key.tenant, key.group, key.data_id)
    }

    pub fn remove_config(&mut self, key: &ConfigKey) -> bool {
        self.do_remove_config(&key.tenant, &key.group, &key.data_id)
    }

    pub(crate) fn do_insert_config(
        &mut self,
        tenant: Arc<String>,
        group: Arc<String>,
        data_id: Arc<String>,
    ) -> bool {
        let mut result = false;
        if let Some(config_index) = self.tenant_group.get_mut(&tenant) {
            if config_index.insert_config(group, data_id) {
                self.size += 1;
                result = true;
            }
        } else {
            let mut config_index = ConfigIndex::new();
            if config_index.insert_config(group, data_id) {
                self.size += 1;
                result = true;
            }
            self.tenant_group.insert(tenant, config_index);
        }
        result
    }

    pub(crate) fn do_remove_config(
        &mut self,
        tenant: &Arc<String>,
        group: &Arc<String>,
        data_id: &Arc<String>,
    ) -> bool {
        let mut result = false;
        if let Some(config_index) = self.tenant_group.get_mut(tenant) {
            let (b, group_size) = config_index.remove_config(group, data_id);
            if b {
                self.size -= 1;
                result = true;
            }
            if group_size == 0 {
                self.tenant_group.remove(tenant);
            }
        }
        result
    }

    pub fn query_config_page(&self, param: &ConfigQueryParam) -> (usize, Vec<ConfigKey>) {
        let mut rlist = vec![];
        let mut size = 0;
        let mut limit = param.limit;
        if let Some(tenant) = &param.tenant {
            if let Some(index) = self.tenant_group.get(tenant) {
                return index.query_config_page(tenant, limit, param);
            }
        } else {
            for (tenant, service_index) in &self.tenant_group {
                let (sub_size, mut sub_list) =
                    service_index.query_config_page(tenant, limit, param);
                size += sub_size;
                limit -= sub_list.len();
                rlist.append(&mut sub_list);
            }
        }
        (size, rlist)
    }

    pub fn get_tenant_count(&self) -> usize {
        self.tenant_group.len()
    }

    pub fn get_config_count(&self) -> (usize, usize) {
        let mut group_sum = 0;
        let mut sum = 0;
        for service_index in self.tenant_group.values() {
            group_sum += service_index.group_data.len();
            sum += service_index.get_config_count();
        }
        (group_sum, sum)
    }
}

#[test]
fn add_service() {
    let mut index = TenantIndex::new();
    let key1 = ConfigKey::new("1", "1", "1");
    let key2 = ConfigKey::new("1", "1", "2");
    let key3 = ConfigKey::new("1", "2", "1");
    index.insert_config(key1.clone());
    assert!(index.size == 1);
    index.insert_config(key2.clone());
    assert!(index.size == 2);
    index.insert_config(key3.clone());
    assert!(index.size == 3);
    assert!(index.tenant_group.len() == 2);
    //remove
    index.remove_config(&key1);
    assert!(index.size == 2);
    index.remove_config(&key1);
    assert!(index.size == 2);
    index.remove_config(&key2);
    assert!(index.size == 1);
    index.remove_config(&key3);
    index.remove_config(&key3);
    assert!(index.size == 0);
    assert!(index.tenant_group.is_empty());
}

#[test]
fn query_service() {
    let mut index = TenantIndex::new();
    let key1 = ConfigKey::new("1", "1", "1");
    let key2 = ConfigKey::new("1", "1", "2");
    let key3 = ConfigKey::new("1", "2", "1");
    let key4 = ConfigKey::new("1", "2", "2");
    let key5 = ConfigKey::new("2", "1", "1");
    index.insert_config(key1.clone());
    index.insert_config(key2.clone());
    index.insert_config(key3.clone());
    index.insert_config(key4.clone());
    index.insert_config(key5.clone());

    let mut param = ConfigQueryParam {
        tenant: None,
        limit: 0xffff_ffff,
        ..ConfigQueryParam::default()
    };
    let (size, list) = index.query_config_page(&param);
    assert!(size == 5);
    assert!(list.len() == 5);

    param.limit = 2;
    let (size, list) = index.query_config_page(&param);
    assert!(size == 5);
    assert!(list.len() == 2);

    index.remove_config(&key1);
    index.remove_config(&key2);
    index.remove_config(&key3);
    index.remove_config(&key4);
    index.remove_config(&key5);

    param.limit = 2;
    let (size, list) = index.query_config_page(&param);
    assert!(size == 0);
    assert!(list.is_empty());
}
