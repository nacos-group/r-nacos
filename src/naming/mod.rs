pub mod api;
pub mod api_model;
pub mod core;
pub(crate) mod filter;
pub mod listener;
pub mod model;
pub mod naming_delay_nofity;
pub mod naming_subscriber;
pub mod service;
pub mod udp_actor;
//pub(crate) mod dal;
pub mod ops;
pub mod service_index;
pub mod cluster;

pub struct NamingUtils;

pub const DEFAULT_NAMESPACE: &str = "public";
pub const DEFAULT_CLUSTER: &str = "DEFAULT";
pub const DEFAULT_GROUP: &str = "DEFAULT_GROUP";

impl NamingUtils {
    pub fn get_group_and_service_name(service_name: &str, group_name: &str) -> String {
        format!("{}@@{}", group_name, service_name)
    }

    pub fn split_group_and_serivce_name(grouped_name: &str) -> Option<(String, String)> {
        let split = grouped_name.split("@@").collect::<Vec<_>>();
        if split.is_empty() {
            return None;
        }
        let a = split.first();
        let b = split.get(1);
        match b {
            Some(b) => {
                let a = a.unwrap();
                if a.is_empty() {
                    return None;
                }
                Some(((*a).to_owned(), (*b).to_owned()))
            }
            None => match a {
                Some(a) => {
                    if a.is_empty() {
                        return None;
                    }
                    Some(("DEFAULT_GROUP".to_owned(), (*a).to_owned()))
                }
                None => None,
            },
        }
    }

    pub fn split_filters(cluster_str: &str) -> Vec<String> {
        cluster_str
            .split(',')
            .filter(|e| !e.is_empty())
            .map(|e| e.to_owned())
            .collect::<Vec<_>>()
    }

    pub fn default_namespace(val: String) -> String {
        if val.is_empty() {
            DEFAULT_NAMESPACE.to_owned()
        } else {
            val
        }
    }

    pub fn default_cluster(val: String) -> String {
        if val.is_empty() {
            DEFAULT_CLUSTER.to_owned()
        } else {
            val
        }
    }

    pub fn default_group(val: String) -> String {
        if val.is_empty() {
            DEFAULT_GROUP.to_owned()
        } else {
            val
        }
    }
}
