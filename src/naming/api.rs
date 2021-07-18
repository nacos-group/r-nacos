
use super::core::{Instance, InstanceUpdateTag,NamingActor,NamingCmd,NamingResult,ServiceKey,NamingUtils};
use super::super::utils::{select_option_by_clone,get_bool_from_string};

use actix_web::{
    App,web,HttpRequest,HttpResponse,Responder,HttpMessage,middleware,HttpServer
};

use chrono::Local;
use serde::{Serialize,Deserialize};
use actix::prelude::*;
use std::collections::HashMap;

#[derive(Debug,Serialize,Deserialize)]
pub struct InstanceWebParams {
    pub ip:Option<String>,
    pub port:Option<u32>,
    pub namespaceId:Option<String>,
    pub weight: Option<f32>,
    pub enabled:Option<String>,
    pub healthy:Option<String>,
    pub ephemeral:Option<String>,
    pub metadata:Option<String>,
    pub clusterName:Option<String>,
    pub serviceName:Option<String>,
    pub groupName:Option<String>,
}

impl InstanceWebParams {
    fn select_option(&self,o:&Self) -> Self{
        Self {
            ip: select_option_by_clone(&self.ip, &o.ip),
            port: select_option_by_clone(&self.port, &o.port),
            namespaceId: select_option_by_clone(&self.namespaceId, &o.namespaceId),
            weight: select_option_by_clone(&self.weight, &o.weight),
            enabled: select_option_by_clone(&self.enabled, &o.enabled),
            healthy: select_option_by_clone(&self.healthy, &o.healthy),
            ephemeral: select_option_by_clone(&self.ephemeral, &o.ephemeral),
            metadata: select_option_by_clone(&self.metadata, &o.metadata),
            clusterName: select_option_by_clone(&self.clusterName, &o.clusterName),
            serviceName: select_option_by_clone(&self.serviceName, &o.serviceName),
            groupName: select_option_by_clone(&self.groupName, &o.groupName),
        }
    }

    fn to_instance(&self) -> Result<Instance,String> {
        let mut instance = Instance::default();
        instance.ip = self.ip.as_ref().unwrap().to_owned();
        instance.port = self.port.as_ref().unwrap().to_owned();
        let grouped_name = self.serviceName.as_ref().unwrap().to_owned();
        if let Some((group_name,service_name)) = NamingUtils::split_group_and_serivce_name(&grouped_name) {
            instance.service_name = service_name;
            instance.group_name = group_name;
        }
        else{
            return Err("serivceName is unvaild!".to_owned());
        }
        if let Some(group_name) = self.groupName.as_ref() {
            if group_name.len() > 0 {
                instance.group_name = group_name.to_owned();
            }
        }
        instance.weight = self.weight.unwrap_or(1f32);
        instance.enabled = get_bool_from_string(&self.enabled, true);
        instance.healthy= get_bool_from_string(&self.healthy, true);
        instance.ephemeral= get_bool_from_string(&self.ephemeral, true);
        instance.cluster_name = self.clusterName.as_ref().unwrap_or(&"DEFAULT".to_owned()).to_owned();
        instance.namespace_id= self.namespaceId.as_ref().unwrap_or(&"public".to_owned()).to_owned();
        let metadata_str= self.metadata.as_ref().unwrap_or(&"{}".to_owned()).to_owned();
        match serde_json::from_str::<HashMap<String,String>>(&metadata_str) {
            Ok(metadata) => {
                instance.metadata = metadata;
            },
            Err(e) => {}
        };
        instance.generate_key();
        Ok(instance)
    }
}

#[derive(Debug,Serialize,Deserialize)]
pub struct InstanceWebQueryListParams {
    pub namespaceId:Option<String>,
    pub serviceName:Option<String>,
    pub groupName:Option<String>,
    pub clusters:Option<String>,
    pub healthyOnly:Option<bool>,
}

impl InstanceWebQueryListParams {
    fn to_clusters_key(&self) -> Result<(ServiceKey,Vec<String>),String> {
        let mut service_name = "".to_owned();
        let mut group_name = "".to_owned();
        let grouped_name = self.serviceName.as_ref().unwrap().to_owned();
        if let Some((_group_name,_service_name)) = NamingUtils::split_group_and_serivce_name(&grouped_name) {
            service_name = _service_name;
            group_name = _group_name;
        }
        else{
            return Err("serivceName is unvaild!".to_owned());
        }
        if let Some(_group_name) = self.groupName.as_ref() {
            if _group_name.len() > 0 {
                group_name = _group_name.to_owned();
            }
        }
        let namespace_id = self.namespaceId.as_ref().unwrap_or(&"public".to_owned()).to_owned();
        let key = ServiceKey::new(&namespace_id,&group_name,&service_name);

        let mut clusters = vec![];
        if let Some(cluster_str) = self.clusters.as_ref() {
            clusters = cluster_str.split(",").into_iter()
                .filter(|e|{e.len()>0}).map(|e|{e.to_owned()}).collect::<Vec<_>>();
        }
        Ok((key,clusters))
    }
}

#[derive(Debug,Serialize,Deserialize,Default)]
pub struct InstanceVO {
    service:String,
    ip:String,
    port:u32,
    clusterName:String,
    weight:f32,
    healthy:bool,
    instanceId:String,
    metadata:HashMap<String,String>,
    marked:Option<bool>,
    enabled:Option<bool>,
    serviceName:Option<String>,
    ephemeral:Option<bool>,
}

impl InstanceVO {
    fn from_instance(instance:&Instance) -> Self {
        Self {
            service:NamingUtils::get_group_and_service_name(&instance.service_name,&instance.group_name),
            ip:instance.ip.to_owned(),
            port:instance.port,
            clusterName:instance.cluster_name.to_owned(),
            weight:instance.weight,
            healthy:instance.healthy,
            instanceId:instance.id.to_owned(),
            metadata:instance.metadata.clone(),
            marked:Some(true),
            enabled:Some(instance.enabled),
            serviceName:Some(instance.service_name.to_owned()),
            ephemeral:Some(instance.ephemeral),
        }
    }
}


#[derive(Debug,Serialize,Deserialize,Default)]
pub struct QueryListResult {
    name:String,
    clusters:String,
    cacheMillis:u64,
    hosts:Vec<InstanceVO>,
    lastRefTime:Option<i64>,
    checksum:Option<String>,
    useSpecifiedURL:Option<bool>,
    env:Option<String>,
    protectThreshold:Option<f32>,
    reachLocalSiteCallThreshold:Option<bool>,
    dom:Option<String>,
    metadata:Option<HashMap<String,String>>,
}

#[derive(Debug,Serialize,Deserialize,Default)]
pub struct BeatRequest{
    pub namespaceId:Option<String>,
    pub serviceName:Option<String>,
    pub clusterName:Option<String>,
    pub groupName:Option<String>,
    pub ephemeral:Option<String>,
    pub beat:Option<String>,
}

impl BeatRequest {
    fn select_option(&self,o:&Self) -> Self {
        Self {
            namespaceId: select_option_by_clone(&self.namespaceId, &o.namespaceId),
            clusterName: select_option_by_clone(&self.clusterName, &o.clusterName),
            serviceName: select_option_by_clone(&self.serviceName, &o.serviceName),
            groupName: select_option_by_clone(&self.groupName, &o.groupName),
            ephemeral: select_option_by_clone(&self.ephemeral, &o.ephemeral),
            beat: select_option_by_clone(&self.beat, &o.beat),
        }
    }

    pub fn to_instance(&self) -> Result<Instance,String> {
        let beat = self.beat.as_ref().unwrap();
        let mut beat_info = serde_json::from_str::<BeatInfo>(beat).unwrap();
        let mut instance = beat_info.to_instance();
        if beat_info.serviceName.is_none() {
            let grouped_name = self.serviceName.as_ref().unwrap().to_owned();
            if let Some((group_name,service_name)) = NamingUtils::split_group_and_serivce_name(&grouped_name) {
                instance.service_name = service_name;
                instance.group_name = group_name;
            }
            if let Some(group_name) = self.groupName.as_ref(){
                if group_name.len()>0{
                    instance.group_name = group_name.to_owned();
                }
            }
        }
        instance.ephemeral = get_bool_from_string(&self.ephemeral, true);
        instance.cluster_name = self.clusterName.as_ref().unwrap_or(&"DEFAULT".to_owned()).to_owned();
        instance.namespace_id= self.namespaceId.as_ref().unwrap_or(&"public".to_owned()).to_owned();
        instance.generate_key();
        Ok(instance)
    }
}

#[derive(Debug,Serialize,Deserialize,Default)]
pub struct BeatInfo {
    pub cluster:Option<String>,
    pub ip:Option<String>,
    pub port:Option<u32>,
    pub metadata:Option<HashMap<String,String>>,
    pub period:Option<i64>,
    pub scheduled:Option<bool>,
    pub serviceName:Option<String>,
    pub stopped:Option<bool>,
    pub weight:Option<f32>,
}

impl BeatInfo {
    pub fn to_instance(&self) -> Instance {
        let mut instance = Instance::default();
        instance.ip = self.ip.as_ref().unwrap().to_owned();
        instance.port = self.port.as_ref().unwrap().to_owned();
        let grouped_name = self.serviceName.as_ref().unwrap().to_owned();
        if let Some((group_name,service_name)) = NamingUtils::split_group_and_serivce_name(&grouped_name) {
            instance.service_name = service_name;
            instance.group_name = group_name;
        }
        instance.cluster_name = self.cluster.as_ref().unwrap_or(&"DEFAULT".to_owned()).to_owned();
        if let Some(metadata) = self.metadata.as_ref() {
            instance.metadata = metadata.clone();
        }
        instance.generate_key();
        instance
    }
}

#[derive(Debug,Serialize,Deserialize,Default)]
pub struct ServiceQueryListRequest {
    pub pageNo:Option<usize>,
    pub pageSize:Option<usize>,
    pub namespaceId:Option<String>,
    pub groupName:Option<String>,
    pub serviceName:Option<String>,
}

#[derive(Debug,Serialize,Deserialize,Default)]
pub struct ServiceQueryListResponce {
    pub count:usize,
    pub doms:Vec<String>,
}


pub async fn get_instance(param:web::Query<InstanceWebParams>,naming_addr:web::Data<Addr<NamingActor>>) -> impl Responder {
    let instance = param.to_instance();
    let return_val = match instance {
        Ok(instance) => {
            match naming_addr.send(NamingCmd::QUERY(instance)).await {
                Ok(res) => {
                    let result: NamingResult = res.unwrap();
                    match result {
                        NamingResult::INSTANCE(v) => {
                            let vo = InstanceVO::from_instance(&v);
                            serde_json::to_string(&vo).unwrap()
                        },
                        _ => {"error".to_owned()}
                    }
                },
                Err(e) => {"error".to_owned()}
            }
        },
        Err(e) => {e}
    };
    return_val
}

pub async fn get_instance_list(param:web::Query<InstanceWebQueryListParams>,naming_addr:web::Data<Addr<NamingActor>>) -> impl Responder {
    let only_healthy = param.healthyOnly.unwrap_or(true);
    let return_val = match param.to_clusters_key() {
        Ok((key,clusters)) => {
            match naming_addr.send(NamingCmd::QUERY_LIST(key.clone(),clusters,only_healthy)).await {
                Ok(res) => {
                    let result: NamingResult = res.unwrap();
                    match result {
                        NamingResult::INSTANCE_LIST(v) => {
                            do_get_instance_list(&param,&key,v)
                        },
                        _ => {"error".to_owned()}
                    }
                },
                Err(e) => {"error".to_owned()}
            }
        },
        Err(e) => {e}
    };
    return_val
}

fn do_get_instance_list(param:&InstanceWebQueryListParams,key:&ServiceKey,v:Vec<Instance>) -> String {
    let mut result = QueryListResult::default();
    result.name = key.get_join_service_name();
    result.cacheMillis = 3000u64;
    let now = Local::now().timestamp_millis();
    result.lastRefTime = Some(now);
    result.checksum = Some(now.to_string());
    result.useSpecifiedURL = Some(false);
    result.clusters = param.clusters.as_ref().unwrap_or(&"".to_owned()).to_owned();
    result.env = Some("".to_owned());
    result.hosts = v.into_iter().map(|e| InstanceVO::from_instance(&e)).collect::<Vec<_>>();
    result.dom = Some(key.service_name.to_owned());
    serde_json::to_string(&result).unwrap()
}

pub async fn add_instance(a:web::Query<InstanceWebParams>,b:web::Form<InstanceWebParams>,naming_addr:web::Data<Addr<NamingActor>>) -> impl Responder {
    let param = a.select_option(&b);
    let instance = param.to_instance();
    let return_val = match instance {
        Ok(instance) => {
            if !instance .check_vaild() {
                "instance check is invalid".to_owned()
            }
            else{
                let res = naming_addr.send(NamingCmd::UPDATE(instance,None)).await ;
                "ok".to_owned()
            }
        },
        Err(e) => {e}
    };
    return_val
}

pub async fn del_instance(a:web::Query<InstanceWebParams>,b:web::Form<InstanceWebParams>,naming_addr:web::Data<Addr<NamingActor>>) -> impl Responder {
    let param = a.select_option(&b);
    let instance = param.to_instance();
    let return_val = match instance {
        Ok(instance) => {
            if !instance .check_vaild() {
                "instance check is invalid".to_owned()
            }
            else{
                let res = naming_addr.send(NamingCmd::DELETE(instance)).await ;
                "ok".to_owned()
            }
        },
        Err(e) => {e}
    };
    return_val
}

pub async fn beat_instance(a:web::Query<BeatRequest>,b:web::Form<BeatRequest>,naming_addr:web::Data<Addr<NamingActor>>) -> impl Responder {
    let param = a.select_option(&b);
    let instance = param.to_instance();
    let return_val = match instance {
        Ok(instance) => {
            if !instance .check_vaild() {
                "instance check is invalid".to_owned()
            }
            else{
                let tag = InstanceUpdateTag::default();
                let res = naming_addr.send(NamingCmd::UPDATE(instance,Some(tag))).await ;
                "ok".to_owned()
            }
        },
        Err(e) => {e}
    };
    return_val
}

pub async fn query_service(param:web::Query<ServiceQueryListRequest>,naming_addr:web::Data<Addr<NamingActor>>) -> impl Responder {
    let return_val ="error".to_owned();
    return_val
}

pub async fn query_service_list(param:web::Query<ServiceQueryListRequest>,naming_addr:web::Data<Addr<NamingActor>>) -> impl Responder {
    let page_size = param.pageSize.unwrap_or(0x7fffffff);
    let page_index = param.pageNo.unwrap_or(1);
    let namespace_id = param.namespaceId.as_ref().unwrap_or(&"public".to_owned()).to_owned();
    let group = param.groupName.as_ref().unwrap_or(&"DEFAULT_GROUP".to_owned()).to_owned();
    let key = ServiceKey::new(&namespace_id,&group,"");
    let return_val = match naming_addr.send(NamingCmd::QUERY_SERVICE_PAGE(key,page_size,page_index)).await {
        Ok(res) => {
            let result: NamingResult = res.unwrap();
            match result {
                NamingResult::SERVICE_PAGE((c,v)) => {
                    let resp = ServiceQueryListResponce {
                        count:c,
                        doms:v,
                    };
                    serde_json::to_string(&resp).unwrap()
                },
                _ => {"error".to_owned()}
            }
        },
        Err(e) => {"error".to_owned()}
    };
    return_val
}

pub fn app_config(config:&mut web::ServiceConfig) {
    config.service(
        web::scope("/nacos/v1/ns")
            .service(web::resource("/instance")
                .route( web::get().to(get_instance))
                .route( web::post().to(add_instance))
                .route( web::put().to(add_instance))
                .route( web::delete().to(del_instance))
            )
            .service(web::resource("/instance/beat")
                .route( web::put().to(beat_instance))
            )
            .service(web::resource("/instance/list")
                .route( web::get().to(get_instance_list))
            )
            .service(web::resource("/service")
                .route( web::get().to(query_service))
            )
            .service(web::resource("/service/list")
                .route( web::get().to(query_service_list))
            )
    );
}
