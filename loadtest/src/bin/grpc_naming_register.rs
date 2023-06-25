#![allow(unused_imports,unreachable_code)]
use nacos_rust_client::client::naming_client::{ServiceInstanceKey, InstanceDefaultListener};
use nacos_rust_client::conn_manage;
use std::sync::Arc;

use std::time::Duration;

use nacos_rust_client::client::{ HostInfo, AuthInfo, ClientBuilder };
use nacos_rust_client::client::{naming_client::{NamingClient, Instance,QueryInstanceListParams}};

pub(crate) const SERVER_COUNT:usize=1000;

fn get_service_ip_list(counts:u64) -> Vec<String> {
    let (icount,jcount)=if counts <= 100 {
        (1,counts)
    }
    else{
        assert!(counts>10000,"不支持超过1万个ip");
        (counts%100+1,100)
    };
    let mut sum= 0;
    let mut rlist=vec![];
    for i in 100..(100+icount) {
        for j in 100..(100+jcount) {
            sum+=1;
            if sum > counts {
                return rlist;
            }
            let ip = format!("192.168.{}.{}",&i,&j);
            rlist.push(ip);
        }
    }
    rlist
}

async fn register(service_name:&str,group_name:&str,ips:&Vec<String>,port:u32,client:&NamingClient) {
    log::info!("register,{},{}",service_name,group_name);
    for ip in ips {
        let instance = Instance::new_simple(&ip,port,service_name,group_name);
        //注册
        client.register(instance);
    }
}

#[tokio::main]
async fn main(){
    std::env::set_var("RUST_LOG","INFO");
    env_logger::init();
    //let host = HostInfo::parse("127.0.0.1:8848");
    //let client = NamingClient::new(host,"".to_owned());
    let namespace_id = "public".to_owned(); //default teant
    //let auth_info = Some(AuthInfo::new("nacos","nacos"));
    let auth_info = None;
    let client = ClientBuilder::new()
        .set_endpoint_addrs("127.0.0.1:8848,127.0.0.1:8848")
        .set_auth_info(auth_info)
        .set_tenant(namespace_id)
        .set_use_grpc(true)
        .build_naming_client();
    tokio::time::sleep(Duration::from_millis(1000)).await;
    
    let ips = get_service_ip_list(10);
    let group_name="DEFAULT_GROUP";
    for i in 0..SERVER_COUNT {
        let service_name = format!("foo_{:04}",i);
        register(&service_name,group_name,&ips,10000,&client).await;
    }
    tokio::signal::ctrl_c().await.expect("failed to listen for event");
}