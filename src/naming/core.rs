
use std::cmp::max;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::LinkedList;
use std::net::SocketAddr;
use chrono::Local;
use serde::{Serialize,Deserialize};
use super::udp_handler::{UdpSender,UdpSenderCmd,UdpReciver};

use actix::prelude::*;

pub struct NamingUtils;

impl NamingUtils {
    pub fn get_group_and_service_name(service_name:&str,group_name:&str) -> String {
        format!("{}@@{}",group_name,service_name)
    }

    pub fn split_group_and_serivce_name(grouped_name:&String) -> Option<(String,String)> {
        let split = grouped_name.split("@@").collect::<Vec<_>>();
        if split.len() ==0 {
            return None
        }
        let a = split.get(0);
        let b = split.get(1);
        match b {
            Some(b) => {
                let a = a.unwrap();
                if a.len()==0 {
                    return None;
                }
                Some(((*a).to_owned(),(*b).to_owned()))
            },
            None=>{
                match a{
                    Some(a) => {
                        if a.len()==0{
                            return None;
                        }
                        Some(("DEFAULT_GROUP".to_owned(),(*a).to_owned()))
                    },
                    None => {
                        None
                    }
                }
            }
        }
    }
}

#[derive(Debug,Clone)]
pub struct InstanceUpdateTag{
    pub weight: bool,
    pub metadata: bool,
    pub enabled: bool,
    pub ephemeral: bool,
}

impl InstanceUpdateTag {
    pub fn is_al(&self) -> bool {
        self.weight && self.metadata && self.enabled && self.ephemeral
    }
    pub fn is_none(&self) -> bool {
        !self.weight && !self.metadata && !self.enabled && !self.ephemeral
    }
}

impl Default for InstanceUpdateTag {
    fn default() -> Self {
        Self {
            weight: true,
            metadata: true,
            enabled: true,
            ephemeral: true,
        }
    }
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct Instance {
    pub id:String,
    pub ip:String,
    pub port:u32,
    pub weight:f32,
    pub enabled:bool,
    pub healthy:bool,
    pub ephemeral: bool,
    pub cluster_name:String,
    pub service_name:String,
    pub group_name:String,
    pub metadata:HashMap<String,String>,
    pub last_modified_millis:i64,
    pub namespace_id:String,
    pub app_name:String,
}

impl Instance{
    pub fn new(ip:String,port:u32) -> Self {
        let mut s = Self::default();
        s.ip = ip;
        s.port = port;
        s
    }

    pub fn generate_key(&mut self) {
        self.id = format!("{}#{}#{}#{}#{}",&self.ip,&self.port,&self.cluster_name,&self.service_name,&self.group_name)
    }

    pub fn init(&mut self) {
        self.last_modified_millis = Local::now().timestamp_millis();
        if self.id.len()==0 {
            self.generate_key();
        }
    }

    pub fn check_vaild(&self) -> bool {
        if self.id.len()==0 || self.port<=0 || self.service_name.len()==0 || self.cluster_name.len()==0 
        {
            return false;
        }
        true
    }

    pub fn update_info(&mut self,o:Self,tag:Option<InstanceUpdateTag>) -> bool {
        let is_update=self.enabled != o.enabled 
        || self.healthy != o.healthy
        || self.weight != o.weight
        || self.ephemeral != o.ephemeral
        || self.metadata != o.metadata
        ;
        self.healthy = o.healthy;
        self.last_modified_millis = o.last_modified_millis;
        if let Some(tag) = tag {
            if tag.weight {
                self.weight = o.weight;
            }
            if tag.metadata {
                self.metadata = o.metadata;
            }
            if tag.enabled {
                self.enabled = o.enabled;
            }
            if tag.ephemeral {
                self.ephemeral = o.ephemeral;
            }
        }
        else{
            self.weight = o.weight;
            self.metadata = o.metadata;
            self.enabled = o.enabled;
            self.ephemeral = o.ephemeral;
        }
        is_update
    }

    pub fn get_service_key(&self) -> ServiceKey {
        ServiceKey::new(&self.namespace_id,&self.group_name,&self.service_name)
    }

    fn get_time_info(&self) -> InstanceTimeInfo {
        InstanceTimeInfo::new(self.id.to_owned(),self.last_modified_millis)
    }
}

impl Default for Instance {
    fn default() -> Self {
        Self {
            id:Default::default(),
            ip:Default::default(),
            port:Default::default(),
            weight:1f32,
            enabled:true,
            healthy:true,
            ephemeral:true,
            cluster_name:"DEFAULT".to_owned(),
            service_name:Default::default(),
            group_name:Default::default(),
            metadata:Default::default(),
            last_modified_millis:Default::default(),
            namespace_id:Default::default(),
            app_name:Default::default(),
        }
    }
}

#[derive(Debug,Clone,Default,Hash,Eq)]
struct InstanceTimeInfo {
    time:i64,
    instance_id:String,
    enable:bool,
}

impl InstanceTimeInfo {
    fn new(instance_id:String,time:i64) -> Self {
        Self {
            time,
            instance_id,
            enable:true,
        }
    }
}

impl PartialEq for InstanceTimeInfo {
    fn eq(&self, o:&Self) -> bool {
        self.time == o.time
    }
}

pub enum UpdateInstanceType{
    None,
    New,
    Remove,
    UpdateTime,
    UpdateValue,
}

#[derive(Debug,Clone,Default)]
pub struct Cluster {
    pub cluster_name:String,
    instances: HashMap<String,Instance>,
    timeinfos: LinkedList<InstanceTimeInfo>,
}

impl Cluster {
    fn update_instance(&mut self,instance:Instance,update_tag:Option<InstanceUpdateTag>) -> UpdateInstanceType {
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

    fn update_timeinfos(&mut self,time_info:InstanceTimeInfo) {
        for item in &mut self.timeinfos {
            if item.instance_id == time_info.instance_id {
                item.enable = false;
            }
        }
        self.timeinfos.push_back(time_info);
    }

    fn time_check(&mut self,healthy_time:i64,offline_time:i64) -> (Vec<String>,Vec<String>) {
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

    fn remove_instance(&mut self,instance_id:&str) {
        self.instances.remove(instance_id);
    }

    fn update_instance_healthy_unvaild(&mut self,instance_id:&str) {
        if let Some(i) = self.instances.get_mut(instance_id){
            i.healthy = false;
        }
    }

    fn get_instance(&self,instance_key:&str) -> Option<&Instance> {
        return self.instances.get(instance_key);
    }

    fn get_all_instances(&self,only_healthy:bool) -> Vec<&Instance> {
        self.instances.values().filter(|x|
            x.enabled && (x.healthy || !only_healthy)).collect::<Vec<_>>()
    }
}

#[derive(Debug,Clone)]
pub struct ServiceKey {
    pub namespace_id:String,
    pub group_name:String,
    pub service_name:String,
}

impl ServiceKey {
    pub fn new(namespace_id:&str,group_name:&str,serivce_name:&str) -> Self {
        Self {
            namespace_id:namespace_id.to_owned(),
            group_name:group_name.to_owned(),
            service_name:serivce_name.to_owned(),
        }
    }

    pub fn get_join_service_name(&self) -> String {
        format!("{}@@{}",self.group_name,self.service_name)
    }

    fn get_namespace_group(&self) -> String {
        format!("{}#{}",self.namespace_id,self.group_name)
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
    pub cluster_map:HashMap<String,Cluster>,
}

impl Service {
    fn recalculate_checksum(&mut self){
        self.check_sum="".to_owned();
    }

    fn remove_instance(&mut self,cluster_name:&str,instance_id:&str) -> UpdateInstanceType {
        if let Some(cluster) = self.cluster_map.get_mut(cluster_name){
            cluster.remove_instance(instance_id);
            return UpdateInstanceType::Remove;
        }
        UpdateInstanceType::None
    }

    fn update_instance(&mut self,instance:Instance,tag:Option<InstanceUpdateTag>) -> UpdateInstanceType {
        let cluster_name = instance.cluster_name.to_owned();
        if let Some(v) = self.cluster_map.get_mut(&cluster_name){
            return v.update_instance(instance, tag);
        }
        let mut cluster = Cluster::default();
        cluster.cluster_name = cluster_name.to_owned();
        let rtype=cluster.update_instance(instance, None);
        self.cluster_map.insert(cluster_name,cluster);
        rtype

    }

    fn get_instance(&self,cluster_name:&str,instance_key:&str) -> Option<Instance> {
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
    
    fn get_instance_list(&self,cluster_names:Vec<String>,only_healthy:bool) -> Vec<Instance> {
        let mut list = vec![];
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
        list
    }

    fn time_check(&mut self,healthy_time:i64,offline_time:i64) -> (Vec<String>,Vec<String>) {
        let mut remove_list = vec![];
        let mut update_list = vec![];
        for item in self.cluster_map.values_mut() {
            let (mut rlist,mut ulist)=item.time_check(healthy_time, offline_time);
            remove_list.append(&mut rlist);
            update_list.append(&mut ulist);
        }
        (remove_list,update_list)
    }
}

#[derive(Debug)]
pub struct NamingActor {
    service_map:HashMap<String,Service>,
    namespace_group_service:HashMap<String,BTreeSet<String>>,
    last_id:u64,
    udp_sender:Addr<UdpSender>,
}

impl NamingActor {
    pub fn new(udp_sender:Addr<UdpSender>) -> Self {
        Self {
            service_map: Default::default(),
            namespace_group_service: Default::default(),
            last_id:0u64,
            udp_sender,
        }
    }

    fn get_service(&mut self,key:&ServiceKey) -> Option<&Service> {
        match self.service_map.get_mut(&key.get_join_service_name()){
            Some(v) => {
                Some(v)
            },
            None => None
        }
    }
    
    fn create_empty_service(&mut self,key:&ServiceKey) {
        let namspace_group = key.get_namespace_group();
        let ng_service_name = key.service_name.to_owned();
        if let Some(set) = self.namespace_group_service.get_mut(&namspace_group){
            set.insert(ng_service_name);
        }
        else{
            let mut set = BTreeSet::new();
            set.insert(ng_service_name);
            self.namespace_group_service.insert(namspace_group,set);
        }
        match self.get_service(key) {
            Some(_) => {},
            None => {
                let mut service = Service::default();
                let current_time = Local::now().timestamp_millis();
                service.service_name = key.service_name.to_owned();
                service.namespace_id = key.namespace_id.to_owned();
                service.group_name = key.group_name.to_owned();
                service.last_modified_millis = current_time;
                service.recalculate_checksum();
                self.service_map.insert(key.get_join_service_name(),service);
            }
        }
    }

    fn add_instance(&mut self,key:&ServiceKey,instance:Instance) -> UpdateInstanceType {
        let service = self.service_map.get_mut(&key.get_join_service_name()).unwrap();
        service.update_instance(instance,None)
    }

    pub fn remove_instance(&mut self,key:&ServiceKey ,cluster_name:&str,instance_id:&str) -> UpdateInstanceType {
        let service = self.service_map.get_mut(&key.get_join_service_name()).unwrap();
        service.remove_instance(cluster_name, instance_id)
    }

    pub fn update_instance(&mut self,key:&ServiceKey,mut instance:Instance,tag:Option<InstanceUpdateTag>) -> UpdateInstanceType {
        instance.init();
        assert!(instance.check_vaild());
        self.create_empty_service(key);
        let service = self.service_map.get_mut(&key.get_join_service_name()).unwrap();
        service.update_instance(instance, tag)
    }

    pub fn get_instance(&self,key:&ServiceKey,cluster_name:&str,instance_id:&str) -> Option<Instance> {
        if let Some(service) = self.service_map.get(&key.get_join_service_name()) {
            return service.get_instance(cluster_name, instance_id);
        }
        None
    }

    pub fn get_instance_list(&self,key:&ServiceKey,cluster_names:Vec<String>,only_healthy:bool) -> Vec<Instance> {
        if let Some(service) = self.service_map.get(&key.get_join_service_name()) {
            return service.get_instance_list(cluster_names, only_healthy);
        }
        vec![]
    }

    pub fn time_check(&mut self) -> (Vec<String>,Vec<String>) {
        let current_time = Local::now().timestamp_millis();
        let healthy_time = current_time - 15000;
        let offline_time = current_time - 30000;
        let mut remove_list = vec![];
        let mut update_list = vec![];
        for item in self.service_map.values_mut(){
            let (mut rlist,mut ulist)=item.time_check(healthy_time, offline_time);
            remove_list.append(&mut rlist);
            update_list.append(&mut ulist);
        }
        (remove_list,update_list)
    }

    pub fn get_service_list(&self,page_size:usize,page_index:usize,key:&ServiceKey) -> (usize,Vec<String>) {
        let offset = page_size * max(page_index-1,0);
        if let Some(set) = self.namespace_group_service.get(&key.get_namespace_group()){
            let size = set.len();
            return (size,set.into_iter().skip(offset).take(page_size).map(|e| e.to_owned()).collect::<Vec<_>>());
        }
        (0,vec![])
    }
}

#[derive(Debug,Message)]
#[rtype(result = "Result<NamingResult,std::io::Error>")]
pub enum NamingCmd {
    UPDATE(Instance,Option<InstanceUpdateTag>),
    DELETE(Instance),
    QUERY(Instance),
    QUERY_LIST(ServiceKey,Vec<String>,bool),
    QUERY_SERVICE_PAGE(ServiceKey,usize,usize),
    PEEK_LISTENER_TIME_OUT,
    ClientReceiveMsg((SocketAddr,Vec<u8>)),
}

pub enum NamingResult {
    NULL,
    INSTANCE(Instance),
    INSTANCE_LIST(Vec<Instance>),
    SERVICE_PAGE((usize,Vec<String>)),
}

impl Actor for NamingActor {
    type Context = Context<Self>;
}

impl Handler<NamingCmd> for NamingActor {
    type Result = Result<NamingResult,std::io::Error>;

    fn handle(&mut self,msg:NamingCmd,ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            NamingCmd::UPDATE(instance,tag) => {
                self.update_instance(&instance.get_service_key(), instance, tag);
                Ok(NamingResult::NULL)
            },
            NamingCmd::DELETE(instance) => {
                self.remove_instance(&instance.get_service_key(), &instance.cluster_name, &instance.id);
                Ok(NamingResult::NULL)
            },
            NamingCmd::QUERY(instance) => {
                if let Some(i) = self.get_instance(&instance.get_service_key(),&instance.cluster_name, &instance.id) {
                    return Ok(NamingResult::INSTANCE(i));
                }
                Ok(NamingResult::NULL)
            },
            NamingCmd::QUERY_LIST(service_key,cluster_names,only_healthy) => {
                let list = self.get_instance_list(&service_key, cluster_names, only_healthy);
                Ok(NamingResult::INSTANCE_LIST(list))
            },
            NamingCmd::QUERY_SERVICE_PAGE(service_key, page_size, page_index) => {
                Ok(NamingResult::SERVICE_PAGE(self.get_service_list(page_size, page_index, &service_key)))
            },
            NamingCmd::PEEK_LISTENER_TIME_OUT => {
                self.time_check();
                Ok(NamingResult::NULL)
            },
            _ => {Ok(NamingResult::NULL)}
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::net::UdpSocket;
    use super::*;
    #[tokio::test()]
    async fn test01(){
        let local_addr:SocketAddr = "0.0.0.0:0".parse().unwrap();
        let socket = UdpSocket::bind(local_addr).await.unwrap();
        let (r,w) = socket.split();
        let sender_addr = UdpSender::new(w).start();
        let mut naming = NamingActor::new(sender_addr);
        let mut instance = Instance::new("127.0.0.1".to_owned(),8080);
        instance.namespace_id = "public".to_owned();
        instance.service_name = "foo".to_owned();
        instance.group_name = "DEFUALT".to_owned();
        instance.cluster_name= "DEFUALT".to_owned();
        instance.init();
        let key = instance.get_service_key();
        naming.update_instance(&key, instance, None);

        println!("-------------");
        let items = naming.get_instance_list(&key, vec!["DEFUALT".to_owned()], true);
        println!("DEFUALT list:{}",serde_json::to_string(&items).unwrap());
        let items = naming.get_instance_list(&key, vec![], true);
        println!("empty cluster list:{}",serde_json::to_string(&items).unwrap());
        std::thread::sleep(std::time::Duration::from_millis(18000));
        naming.time_check();
        println!("-------------");
        let items = naming.get_instance_list(&key, vec![], false);
        println!("empty cluster list:{}",serde_json::to_string(&items).unwrap());
        std::thread::sleep(std::time::Duration::from_millis(18000));
        naming.time_check();
        println!("-------------");
        let items = naming.get_instance_list(&key, vec![], false);
        println!("empty cluster list:{}",serde_json::to_string(&items).unwrap());

    }
}

