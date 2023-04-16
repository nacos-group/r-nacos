
pub mod model;
pub mod service;
pub mod core;
pub mod api;
pub mod api_model;
pub mod listener;
pub mod udp_actor;
pub mod naming_subscriber;
pub mod naming_delay_nofity;
pub(crate) mod filter;
pub(crate) mod dal;

pub struct NamingUtils;

pub const DEFAULT_NAMESPACE: &str = "public";
pub const DEFAULT_CLUSTER : &str = "DEFAULT";
pub const DEFAULT_GROUP : &str = "DEFAULT_GROUP";

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

    pub fn split_filters(cluster_str:&str) -> Vec<String> {
        cluster_str.split(",").into_iter()
                .filter(|e|{e.len()>0}).map(|e|{e.to_owned()}).collect::<Vec<_>>()
    }

    pub fn default_namespace(val:String) -> String {
        if val.is_empty() { DEFAULT_NAMESPACE.to_owned() } else{ val }
    }

    pub fn default_cluster(val:String) -> String {
        if val.is_empty() { DEFAULT_CLUSTER.to_owned()} else {val}
    }

    pub fn default_group(val:String) -> String {
        if val.is_empty() { DEFAULT_GROUP.to_owned()} else {val}
    }

}
