#![allow(unused_assignments,unused_imports)]

use std::collections::{HashMap, LinkedList};

use super::{model::{Instance, InstanceTimeInfo, InstanceUpdateTag, UpdateInstanceType, ServiceKey}, api_model::QueryListResult};


#[derive(Debug,Clone,Default)]
pub struct Cluster {
    pub cluster_name:String,
    pub(crate) instances: HashMap<String,Instance>,
    pub(crate) timeinfos: LinkedList<InstanceTimeInfo>,
}

impl Cluster {
    pub(crate) fn update_instance(&mut self,instance:Instance,update_tag:Option<InstanceUpdateTag>) -> UpdateInstanceType {
        let key = instance.id.to_owned();
        let time_info = instance.get_time_info();
        let mut update_mark = true;
        let mut rtype = UpdateInstanceType::None;
        if let Some(v) = self.instances.get_mut(&key){
            let old_last_time  = v.last_modified_millis;
            let is_update = v.update_info(instance, update_tag);
            if is_update {
                rtype=UpdateInstanceType::UpdateValue;
            }
            else{
                rtype=UpdateInstanceType::UpdateTime;
            }
            // 避免频繁更新
            if !is_update && time_info.time < old_last_time + 500 {
                rtype=UpdateInstanceType::None;
                update_mark = false;
                v.last_modified_millis = old_last_time;
            }
        }
        else{
            self.instances.insert(key,instance);
            rtype=UpdateInstanceType::New;
        }
        if update_mark {
            self.update_timeinfos(time_info);
        }
        rtype
    }

    pub(crate) fn update_timeinfos(&mut self,time_info:InstanceTimeInfo) {
        for item in &mut self.timeinfos {
            if item.instance_id == time_info.instance_id {
                item.enable = false;
            }
        }
        self.timeinfos.push_back(time_info);
    }

    pub(crate) fn time_check(&mut self,healthy_time:i64,offline_time:i64) -> (Vec<String>,Vec<String>) {
        assert!(healthy_time>=offline_time);
        let mut i=0;
        let t= self.timeinfos.iter();
        let mut remove_list = vec![];
        let mut update_list = vec![];
        let mut remove_index = 0;
        for item in t {
            i+=1;
            if !item.enable { continue;}
            if item.time <= healthy_time {
                if item.time <= offline_time {
                    remove_list.push(item.instance_id.to_owned());
                    remove_index=i;
                }
                else{
                    update_list.push(item.instance_id.to_owned());
                }
            }
            else{
                break;
            }
        }
        self.timeinfos = self.timeinfos.split_off(remove_index);
        for item in &remove_list {
            self.remove_instance(&item);
        }
        for item in &update_list {
            self.update_instance_healthy_unvaild(&item);
        }
        (remove_list,update_list)
    }

    pub(crate) fn remove_instance(&mut self,instance_id:&str) {
        self.instances.remove(instance_id);
    }

    pub(crate) fn update_instance_healthy_unvaild(&mut self,instance_id:&str) {
        if let Some(i) = self.instances.get_mut(instance_id){
            i.healthy = false;
        }
    }

    pub(crate) fn get_instance(&self,instance_key:&str) -> Option<&Instance> {
        return self.instances.get(instance_key);
    }

    pub(crate) fn get_all_instances(&self,only_healthy:bool) -> Vec<&Instance> {
        self.instances.values().filter(|x|
            x.enabled && (x.healthy || !only_healthy)).collect::<Vec<_>>()
    }
}



#[derive(Debug,Clone,Default)]
pub struct Service {
    pub service_name:String,
    pub group_name:String,
    pub metadata:HashMap<String,String>,
    pub protect_threshold:f32,
    pub last_modified_millis:i64,
    pub has_instance:bool,

    pub namespace_id:String,
    pub app_name:String,
    pub check_sum:String,
    //pub cluster_map:HashMap<String,Cluster>,

    pub(crate) instances: HashMap<String,Instance>,
    pub(crate) timeinfos: LinkedList<InstanceTimeInfo>,

}

impl Service {
    pub(crate) fn recalculate_checksum(&mut self){
        self.check_sum="".to_owned();
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

    pub(crate) fn update_instance(&mut self,instance:Instance,update_tag:Option<InstanceUpdateTag>) -> UpdateInstanceType {
        if instance.service_name=="service-consumer" {
            println!("service-consumer update_instance {:?}",&instance);
        }
        let key = instance.id.to_owned();
        let time_info = instance.get_time_info();
        let mut update_mark = true;
        let mut rtype = UpdateInstanceType::None;
        if let Some(v) = self.instances.get_mut(&key){
            let old_last_time  = v.last_modified_millis;
            let is_update = v.update_info(instance, update_tag);
            if is_update {
                rtype=UpdateInstanceType::UpdateValue;
            }
            else{
                rtype=UpdateInstanceType::UpdateTime;
            }
            // 避免频繁更新
            if !is_update && time_info.time < old_last_time + 500 {
                rtype=UpdateInstanceType::None;
                update_mark = false;
                v.last_modified_millis = old_last_time;
            }
        }
        else{
            self.instances.insert(key,instance);
            rtype=UpdateInstanceType::New;
        }
        if update_mark {
            self.update_timeinfos(time_info);
        }
        rtype
    }

    pub(crate) fn update_timeinfos(&mut self,time_info:InstanceTimeInfo) {
        for item in &mut self.timeinfos {
            if item.instance_id == time_info.instance_id {
                item.enable = false;
            }
        }
        self.timeinfos.push_back(time_info);
    }

    pub(crate) fn time_check(&mut self,healthy_time:i64,offline_time:i64) -> (Vec<String>,Vec<String>) {
        assert!(healthy_time>=offline_time);
        let mut i=0;
        let t= self.timeinfos.iter();
        let mut remove_list = vec![];
        let mut update_list = vec![];
        let mut remove_index = 0;
        for item in t {
            i+=1;
            if !item.enable { continue;}
            if item.time <= healthy_time {
                if item.time <= offline_time {
                    remove_list.push(item.instance_id.to_owned());
                    remove_index=i;
                }
                else{
                    update_list.push(item.instance_id.to_owned());
                }
            }
            else{
                break;
            }
        }
        self.timeinfos = self.timeinfos.split_off(remove_index);
        for item in &remove_list {
            self.remove_instance(&item);
        }
        for item in &update_list {
            self.update_instance_healthy_unvaild(&item);
        }
        (remove_list,update_list)
    }

    pub(crate) fn remove_instance(&mut self,instance_id:&str) -> UpdateInstanceType {
        if let Some(_)=self.instances.remove(instance_id){
            UpdateInstanceType::Remove
        }
        else{
            UpdateInstanceType::None
        }
    }

    pub(crate) fn update_instance_healthy_unvaild(&mut self,instance_id:&str) {
        if let Some(i) = self.instances.get_mut(instance_id){
            i.healthy = false;
        }
    }

    pub(crate) fn get_instance(&self,instance_key:&str) -> Option<Instance> {
        self.instances.get(instance_key).map_or(None, |i|Some(i.clone()))
    }

    pub(crate) fn get_all_instances(&self,only_healthy:bool) -> Vec<&Instance> {
        self.instances.values().filter(|x|
            x.enabled && (x.healthy || !only_healthy)).collect::<Vec<_>>()
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

    /* 
    pub(crate) fn get_instance(&self,cluster_name:&str,instance_key:&str) -> Option<Instance> {
        match self.cluster_map.get(cluster_name){
            Some(v) => {
                if let Some(i) = v.get_instance(instance_key) {
                    Some(i.clone())
                }
                else{ None}
            },
            None=> None,
        }
    }
    */
    
    pub(crate) fn get_instance_list(&self,cluster_names:Vec<String>,only_healthy:bool) -> Vec<Instance> {
        let mut list = vec![];
        for item in self.get_all_instances(only_healthy) {
            list.push(item.clone());
        }
        /* 
        let mut names = cluster_names;
        if names.len()==0 {
            for item in self.cluster_map.keys() {
                names.push(item.to_owned());
            }
        }
        for cluster_name in &names {
            if let Some(c) = self.cluster_map.get(cluster_name) {
                for item in c.get_all_instances(only_healthy) {
                    list.push(item.clone());
                }
            }
        }
        */
        list
    }

    /*
    pub(crate) fn get_instance_map(&self,cluster_names:Vec<String>,only_healthy:bool) -> HashMap<String,Vec<Instance>> {
        let mut map=HashMap::new();
        let mut names = cluster_names;
        if names.len()==0 {
            for item in self.cluster_map.keys() {
                names.push(item.to_owned());
            }
        }
        for cluster_name in &names {
            if let Some(c) = self.cluster_map.get(cluster_name) {
                let mut list = vec![];
                for item in c.get_all_instances(only_healthy) {
                    list.push(item.clone());
                }
                if list.len()>0 {
                    map.insert(cluster_name.to_owned(), list);
                }
            }
        }
        map
    }
     */

    /*
    pub(crate) fn get_instance_list_string(&self,cluster_names:Vec<String>,only_healthy:bool) -> String {
        let clusters = (&cluster_names).join(",");
        let list = self.get_instance_list(cluster_names, only_healthy);
        let key = ServiceKey::new(&self.namespace_id,&self.group_name,&self.service_name);
        QueryListResult::get_instance_list_string(clusters, &key, list)
    }
     */

     /* 
    pub(crate) fn time_check(&mut self,healthy_time:i64,offline_time:i64) -> (Vec<String>,Vec<String>) {
        let mut remove_list = vec![];
        let mut update_list = vec![];
        for item in self.cluster_map.values_mut() {
            let (mut rlist,mut ulist)=item.time_check(healthy_time, offline_time);
            remove_list.append(&mut rlist);
            update_list.append(&mut ulist);
        }
        (remove_list,update_list)
    }
     */
}
