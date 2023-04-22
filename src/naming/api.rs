#![allow(unused_imports,unused_assignments,unused_variables)]
use super::core::{NamingActor,NamingCmd,NamingResult};
use super::model::{Instance,InstanceUpdateTag,ServiceKey};
use super::NamingUtils;
use super::super::utils::{select_option_by_clone,get_bool_from_string};
use super::api_model::{InstanceVO,QueryListResult};
use super::ops::ops_api::query_opt_service_list;

use actix_web::{
    App,web,HttpRequest,HttpResponse,Responder,HttpMessage,middleware,HttpServer
};

use serde::{Serialize,Deserialize};
use actix::prelude::*;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Debug,Serialize,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceWebParams {
    pub ip:Option<String>,
    pub port:Option<u32>,
    pub namespace_id:Option<String>,
    pub weight: Option<f32>,
    pub enabled:Option<String>,
    pub healthy:Option<String>,
    pub ephemeral:Option<String>,
    pub metadata:Option<String>,
    pub cluster_name:Option<String>,
    pub service_name:Option<String>,
    pub group_name:Option<String>,
}

impl InstanceWebParams {
    fn select_option(&self,o:&Self) -> Self{
        Self {
            ip: select_option_by_clone(&self.ip, &o.ip),
            port: select_option_by_clone(&self.port, &o.port),
            namespace_id: select_option_by_clone(&self.namespace_id, &o.namespace_id),
            weight: select_option_by_clone(&self.weight, &o.weight),
            enabled: select_option_by_clone(&self.enabled, &o.enabled),
            healthy: select_option_by_clone(&self.healthy, &o.healthy),
            ephemeral: select_option_by_clone(&self.ephemeral, &o.ephemeral),
            metadata: select_option_by_clone(&self.metadata, &o.metadata),
            cluster_name: select_option_by_clone(&self.cluster_name, &o.cluster_name),
            service_name: select_option_by_clone(&self.service_name, &o.service_name),
            group_name: select_option_by_clone(&self.group_name, &o.group_name),
        }
    }

    fn to_instance(&self) -> Result<Instance,String> {
        let mut instance = Instance::default();
        instance.ip = self.ip.as_ref().unwrap().to_owned();
        instance.port = self.port.as_ref().unwrap().to_owned();
        let grouped_name = self.service_name.as_ref().unwrap().to_owned();
        if let Some((group_name,service_name)) = NamingUtils::split_group_and_serivce_name(&grouped_name) {
            instance.service_name = service_name;
            instance.group_name = group_name;
        }
        else{
            return Err("serivceName is unvaild!".to_owned());
        }
        if let Some(group_name) = self.group_name.as_ref() {
            if group_name.len() > 0 {
                instance.group_name = group_name.to_owned();
            }
        }
        instance.weight = self.weight.unwrap_or(1f32);
        instance.enabled = get_bool_from_string(&self.enabled, true);
        instance.healthy= get_bool_from_string(&self.healthy, true);
        instance.ephemeral= get_bool_from_string(&self.ephemeral, true);
        instance.cluster_name = NamingUtils::default_cluster(self.cluster_name.as_ref().unwrap_or(&"".to_owned()).to_owned());
        instance.namespace_id= NamingUtils::default_namespace(self.namespace_id.as_ref().unwrap_or(&"".to_owned()).to_owned());
        let metadata_str= self.metadata.as_ref().unwrap_or(&"{}".to_owned()).to_owned();
        match serde_json::from_str::<HashMap<String,String>>(&metadata_str) {
            Ok(metadata) => {
                instance.metadata = metadata;
            },
            Err(_) => {}
        };
        instance.generate_key();
        Ok(instance)
    }
}

#[derive(Debug,Serialize,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceWebQueryListParams {
    pub namespace_id:Option<String>,
    pub service_name:Option<String>,
    pub group_name:Option<String>,
    pub clusters:Option<String>,
    pub healthy_only:Option<bool>,
    #[serde(rename = "clientIP")]
    pub client_ip:Option<String>,
    pub udp_port:Option<u16>,
}

impl InstanceWebQueryListParams {
    fn to_clusters_key(&self) -> Result<(ServiceKey,String),String> {
        let mut service_name = "".to_owned();
        let mut group_name = "".to_owned();
        let grouped_name = self.service_name.as_ref().unwrap().to_owned();
        if let Some((_group_name,_service_name)) = NamingUtils::split_group_and_serivce_name(&grouped_name) {
            service_name = _service_name;
            group_name = _group_name;
        }
        else{
            return Err("serivceName is unvaild!".to_owned());
        }
        if let Some(_group_name) = self.group_name.as_ref() {
            if _group_name.len() > 0 {
                group_name = _group_name.to_owned();
            }
        }
        let namespace_id = NamingUtils::default_namespace(self.namespace_id.as_ref().unwrap_or(&"".to_owned()).to_owned());
        let key = ServiceKey::new(&namespace_id,&group_name,&service_name);

        /* 
        let mut clusters = vec![];
        if let Some(cluster_str) = self.clusters.as_ref() {
            clusters = cluster_str.split(",").into_iter()
                .filter(|e|{e.len()>0}).map(|e|{e.to_owned()}).collect::<Vec<_>>();
        }
        */
        Ok((key,self.clusters.as_ref().unwrap_or(&"".to_owned()).to_owned()))
    }

    fn get_addr(&self) -> Option<SocketAddr> {
        if let Some(port) = &self.udp_port {
            if *port==0u16 {
                return None;
            }
            if let Some(ip_str) = &self.client_ip {
                match ip_str.parse(){
                    Ok(ip) => {
                        return Some(SocketAddr::new(ip, *port));
                    },
                    _ => {}
                }
            }
        }
        None
    }
}

#[derive(Debug,Serialize,Deserialize,Default)]
#[serde(rename_all = "camelCase")]
pub struct BeatRequest{
    pub namespace_id:Option<String>,
    pub service_name:Option<String>,
    pub cluster_name:Option<String>,
    pub group_name:Option<String>,
    pub ephemeral:Option<String>,
    pub beat:Option<String>,
}

impl BeatRequest {
    fn select_option(&self,o:&Self) -> Self {
        Self {
            namespace_id: select_option_by_clone(&self.namespace_id, &o.namespace_id),
            cluster_name: select_option_by_clone(&self.cluster_name, &o.cluster_name),
            service_name: select_option_by_clone(&self.service_name, &o.service_name),
            group_name: select_option_by_clone(&self.group_name, &o.group_name),
            ephemeral: select_option_by_clone(&self.ephemeral, &o.ephemeral),
            beat: select_option_by_clone(&self.beat, &o.beat),
        }
    }

    pub fn to_instance(&self) -> Result<Instance,String> {
        let beat = self.beat.as_ref().unwrap();
        let beat_info = serde_json::from_str::<BeatInfo>(beat).unwrap();
        let mut instance = beat_info.to_instance();
        if beat_info.service_name.is_none() {
            let grouped_name = self.service_name.as_ref().unwrap().to_owned();
            if let Some((group_name,service_name)) = NamingUtils::split_group_and_serivce_name(&grouped_name) {
                instance.service_name = service_name;
                instance.group_name = group_name;
            }
            if let Some(group_name) = self.group_name.as_ref(){
                if group_name.len()>0{
                    instance.group_name = group_name.to_owned();
                }
            }
        }
        instance.ephemeral = get_bool_from_string(&self.ephemeral, true);
        instance.cluster_name = NamingUtils::default_cluster(self.cluster_name.as_ref().unwrap_or(&"".to_owned()).to_owned());
        instance.namespace_id= NamingUtils::default_namespace(self.namespace_id.as_ref().unwrap_or(&"".to_owned()).to_owned());
        instance.generate_key();
        Ok(instance)
    }
}

#[derive(Debug,Serialize,Deserialize,Default)]
#[serde(rename_all = "camelCase")]
pub struct BeatInfo {
    pub cluster:Option<String>,
    pub ip:Option<String>,
    pub port:Option<u32>,
    pub metadata:Option<HashMap<String,String>>,
    pub period:Option<i64>,
    pub scheduled:Option<bool>,
    pub service_name:Option<String>,
    pub stopped:Option<bool>,
    pub weight:Option<f32>,
}

impl BeatInfo {
    pub fn to_instance(&self) -> Instance {
        let mut instance = Instance::default();
        instance.ip = self.ip.as_ref().unwrap().to_owned();
        instance.port = self.port.as_ref().unwrap().to_owned();
        let grouped_name = self.service_name.as_ref().unwrap().to_owned();
        if let Some((group_name,service_name)) = NamingUtils::split_group_and_serivce_name(&grouped_name) {
            instance.service_name = service_name;
            instance.group_name = group_name;
        }
        instance.cluster_name = NamingUtils::default_cluster(self.cluster.as_ref().unwrap_or(&"".to_owned()).to_owned());
        if let Some(metadata) = self.metadata.as_ref() {
            instance.metadata = metadata.clone();
        }
        instance.generate_key();
        instance
    }
}

#[derive(Debug,Serialize,Deserialize,Default)]
#[serde(rename_all = "camelCase")]
pub struct ServiceQueryListRequest {
    pub page_no:Option<usize>,
    pub page_size:Option<usize>,
    pub namespace_id:Option<String>,
    pub group_name:Option<String>,
    pub service_name:Option<String>,
}

#[derive(Debug,Serialize,Deserialize,Default)]
pub struct ServiceQueryListResponce {
    pub count:usize,
    pub doms:Vec<Arc<String>>,
}


pub async fn get_instance(param:web::Query<InstanceWebParams>,naming_addr:web::Data<Addr<NamingActor>>) -> impl Responder {
    let instance = param.to_instance();
    let return_val = match instance {
        Ok(instance) => {
            match naming_addr.send(NamingCmd::Query(instance)).await {
                Ok(res) => {
                    let result: NamingResult = res.unwrap();
                    match result {
                        NamingResult::Instance(v) => {
                            let vo = InstanceVO::from_instance(&v);
                            serde_json::to_string(&vo).unwrap()
                        },
                        _ => {"error".to_owned()}
                    }
                },
                Err(_) => {"error".to_owned()}
            }
        },
        Err(e) => {e}
    };
    return_val
}

pub async fn get_instance_list(param:web::Query<InstanceWebQueryListParams>,naming_addr:web::Data<Addr<NamingActor>>) -> impl Responder {
    log::debug!("get_instance_list:{:?}",&param);
    let only_healthy = param.healthy_only.unwrap_or(true);
    let addr = param.get_addr();
    let return_val = match param.to_clusters_key() {
        Ok((key,clusters)) => {
            match naming_addr.send(NamingCmd::QueryListString(key.clone(),clusters,only_healthy,addr)).await {
                Ok(res) => {
                    let result: NamingResult = res.unwrap();
                    match result {
                        NamingResult::InstanceListString(v) => {
                            v
                        },
                        _ => {"error".to_owned()}
                    }
                },
                Err(_) => {"error".to_owned()}
            }
        },
        Err(e) => {e}
    };
    return_val
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
                let _= naming_addr.send(NamingCmd::Update(instance,None)).await ;
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
                let _= naming_addr.send(NamingCmd::Delete(instance)).await ;
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
                let tag = InstanceUpdateTag{
                    weight:false,
                    enabled:false,
                    ephemeral:false,
                    metadata:false,
                };
                let _= naming_addr.send(NamingCmd::Update(instance,Some(tag))).await ;
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
    let page_size = param.page_size.unwrap_or(0x7fffffff);
    let page_index = param.page_no.unwrap_or(1);
    let namespace_id = NamingUtils::default_namespace(param.namespace_id.as_ref().unwrap_or(&"".to_owned()).to_owned());
    let group = NamingUtils::default_group(param.group_name.as_ref().unwrap_or(&"".to_owned()).to_owned());
    let key = ServiceKey::new(&namespace_id,&group,"");
    let return_val = match naming_addr.send(NamingCmd::QueryServicePage(key,page_size,page_index)).await {
        Ok(res) => {
            let result: NamingResult = res.unwrap();
            match result {
                NamingResult::ServicePage((c,v)) => {
                    let resp = ServiceQueryListResponce {
                        count:c,
                        doms:v,
                    };
                    serde_json::to_string(&resp).unwrap()
                },
                _ => {"error".to_owned()}
            }
        },
        Err(_) => {"error".to_owned()}
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
            //ops
            .service(web::resource("/catalog/services")
                .route( web::get().to(query_opt_service_list))
            )
    );
}
