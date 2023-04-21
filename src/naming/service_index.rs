use std::{sync::Arc, collections::{HashMap, HashSet}};

use crate::common::string_utils::StringUtils;

use super::model::ServiceKey;

#[derive(Debug,Clone,Default)]
pub struct ServiceQueryParam {
    pub namespace_id:Option<Arc<String>>,
    pub group:Option<Arc<String>>,
    pub service:Option<Arc<String>>,
    pub like_group:Option<String>,
    pub like_service:Option<String>,
    pub offset:usize,
    pub limit:usize,
}

impl ServiceQueryParam {
    pub fn match_group(&self,g:&Arc<String>) -> bool {
        if let Some(group) = &self.group {
            if StringUtils::is_empty(group) || StringUtils::eq(g, group) {
                true
            }
            else{
                false
            }
        }
        else if let Some(like_group) = &self.like_group {
            if StringUtils::is_empty(like_group) || StringUtils::like(g, &like_group).is_some() {
                true
            }
            else{
                false
            }
        }
        else{
            true
        }
    }
    pub fn match_service(&self,s:&Arc<String>) -> bool {
        if let Some(service) = &self.service{
            if StringUtils::is_empty(service) || StringUtils::eq(s, service) {
                true
            }
            else{
                false
            }
        }
        else if let Some(like_service) = &self.like_service{
            if StringUtils::is_empty(like_service) || StringUtils::like(s, &like_service).is_some() {
                true
            }
            else{
                false
            }
        }
        else{
            true
        }
    }
}

#[derive(Debug,Clone,Default)]
pub struct ServiceIndex {
    //pub namespace_id:Arc<String>,
    pub(crate) group_service: HashMap<Arc<String>,HashSet<Arc<String>>>,
    pub(crate) service_size:usize,
}

impl ServiceIndex {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    pub(crate) fn insert_service(&mut self,group:Arc<String>,service:Arc<String>) -> bool {
        if let Some(set) = self.group_service.get_mut(&group) {
            if set.insert(service) {
                self.service_size +=1;
                return true;
            }
        }
        else{
            let mut set = HashSet::new();
            set.insert(service);
            self.group_service.insert(group, set);
            self.service_size +=1;
            return true;
        }
        false
    }

    pub(crate) fn remove_service(&mut self,group:&Arc<String>,service:&Arc<String>) -> (bool,usize) {
        let b=if let Some(set) = self.group_service.get_mut(group) {
            let b=set.remove(service);
            if b {
                self.service_size-=1;
                if set.len()==0 {
                    self.group_service.remove(group);
                }
            }
            b
        }
        else{
            false
        };
        (b,self.group_service.len())
    }

    pub(crate) fn query_service_list(&self,namespace_id:&Arc<String>,group_key:&Arc<String>,service_key:&Arc<String>) 
        -> Vec<ServiceKey> {
        let mut rlist=vec![];
        for (g,set) in &self.group_service{
            if StringUtils::is_empty(group_key) || StringUtils::like(g, &group_key).is_some() {
                for s in set {
                    if StringUtils::is_empty(service_key) || StringUtils::like(s, &service_key).is_some() {
                        let service_key = ServiceKey::new_by_arc(namespace_id.clone(),g.clone(),s.clone());
                        rlist.push(service_key);
                    }
                }
            }
        }
        rlist
    }

    pub(crate) fn query_service_page(&self,namespace_id:&Arc<String>,limit:usize,param:&ServiceQueryParam) 
        -> (usize,Vec<ServiceKey>) {
        let mut rlist=vec![];
        let end_index = param.offset+limit;
        let mut index = 0;
        for (g,set) in &self.group_service{
            if param.match_group(g) {
                for s in set {
                    if param.match_service(s) {
                        if index>=param.offset && index<end_index {
                            let service_key = ServiceKey::new_by_arc(namespace_id.clone(),g.clone(),s.clone());
                            rlist.push(service_key);
                        }
                        index+=1;
                    }
                }
            }
        }
        (index,rlist)
    }
}

#[derive(Debug,Clone,Default)]
pub struct NamespaceIndex {
    pub namespace_group: HashMap<Arc<String>,ServiceIndex>,
    pub service_size:usize,
}

impl NamespaceIndex {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn insert_service(&mut self,key:ServiceKey) -> bool {
        self.do_insert_service(key.namespace_id, key.group_name, key.service_name)
    }

    pub fn remove_service(&mut self,key:&ServiceKey) -> bool {
        self.do_remove_service(&key.namespace_id,&key.group_name,&key.service_name)
    }

    pub(crate) fn do_insert_service(&mut self,namespace_id:Arc<String>,group:Arc<String>,service:Arc<String>) -> bool {
        let mut result = false;
        if let Some(service_index) = self.namespace_group.get_mut(&namespace_id) {
            if service_index.insert_service(group, service){
                self.service_size+=1;
                result = true;
            }
        }
        else{
            let mut service_index = ServiceIndex::new();
            if service_index.insert_service(group, service){
                self.service_size+=1;
                result = true;
            }
            self.namespace_group.insert(namespace_id, service_index);
        }
        result
    }

    pub(crate) fn do_remove_service(&mut self,namespace_id:&Arc<String>,group:&Arc<String>,service:&Arc<String>) -> bool {
        let mut result = false;
        if let Some(service_index) = self.namespace_group.get_mut(namespace_id) {
            let (b,group_size) = service_index.remove_service(group, service);
            if b {
                self.service_size -=1;
                result = true;
            }
            if group_size == 0 {
                self.namespace_group.remove(namespace_id);
            }
        }
        result
    }

    pub fn query_service_page(&self,param:&ServiceQueryParam) 
        -> (usize,Vec<ServiceKey>) {
        let mut rlist=vec![];
        let mut size=0;
        let mut limit = param.limit;
        if let Some(namespace_id) = &param.namespace_id {
            if let Some(index) = self.namespace_group.get(namespace_id) {
                return index.query_service_page(namespace_id,limit, &param);
            }
        }
        else{
            for (namespace_id,service_index) in &self.namespace_group {
                let (sub_size,mut sub_list) = service_index.query_service_page(&namespace_id,limit, &param);
                size +=sub_size;
                limit -= sub_list.len();
                rlist.append(&mut sub_list);
            }
        }
        (size,rlist)
    }

}

#[test]
fn add_service(){
    let mut index = NamespaceIndex::new();
    let key1 = ServiceKey::new("1","1","1");
    let key2 = ServiceKey::new("1","1","2");
    let key3 = ServiceKey::new("1","2","1");
    index.insert_service(key1.clone());
    assert!(index.service_size==1);
    index.insert_service(key2.clone());
    assert!(index.service_size==2);
    index.insert_service(key3.clone());
    assert!(index.service_size==3);
    assert!(index.namespace_group.len()==1);
    //remove
    index.remove_service(&key1);
    assert!(index.service_size==2);
    index.remove_service(&key1);
    assert!(index.service_size==2);
    index.remove_service(&key2);
    assert!(index.service_size==1);
    index.remove_service(&key3);
    index.remove_service(&key3);
    assert!(index.service_size==0);
    assert!(index.namespace_group.len()==0);
}

#[test]
fn query_service(){
    let mut index = NamespaceIndex::new();
    let key1 = ServiceKey::new("1","1","1");
    let key2 = ServiceKey::new("1","1","2");
    let key3 = ServiceKey::new("1","2","1");
    let key4 = ServiceKey::new("1","2","2");
    let key5 = ServiceKey::new("2","1","1");
    index.insert_service(key1.clone());
    index.insert_service(key2.clone());
    index.insert_service(key3.clone());
    index.insert_service(key4.clone());
    index.insert_service(key5.clone());

    let mut param = ServiceQueryParam {
        namespace_id:None,
        limit:0xffff_ffff ,
        ..ServiceQueryParam::default()
    };
    let (size,list)=index.query_service_page(&param);
    assert!(size==5);
    assert!(list.len()==5);

    param.limit=2;
    let (size,list)=index.query_service_page(&param);
    assert!(size==5);
    assert!(list.len()==2);

    index.remove_service(&key1);
    index.remove_service(&key2);
    index.remove_service(&key3);
    index.remove_service(&key4);
    index.remove_service(&key5);

    param.limit=2;
    let (size,list)=index.query_service_page(&param);
    assert!(size==0);
    assert!(list.len()==0);

}


