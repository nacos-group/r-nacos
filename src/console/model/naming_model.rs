use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::naming::{model::{ServiceKey, Instance}, NamingUtils};


/* 
#[derive(Debug,Serialize,Deserialize,Default)]
#[serde(rename_all = "camelCase")]
pub struct ServiceKeyParam{
    pub namespace_id:Option<String>,
    pub group_name:Option<String>,
    pub service_name:Option<String>,
}
*/

#[derive(Debug,Serialize,Deserialize,Default)]
#[serde(rename_all = "camelCase")]
pub struct QueryAllInstanceListParam{
    pub namespace_id:Option<String>,
    pub group_name:Option<String>,
    pub service_name:Option<String>,
}

impl QueryAllInstanceListParam  {
    pub fn to_service_key(self) -> anyhow::Result<ServiceKey> {
        if let Some(service_name) = self.service_name {
            if service_name.is_empty(){
                return Err(anyhow::anyhow!("param error,service is empty"))
            }
            let namespace_id=Arc::new(NamingUtils::default_namespace(self.namespace_id.unwrap_or_default()));
            let group_name=Arc::new(NamingUtils::default_group(self.group_name.unwrap_or_default()));
            Ok(ServiceKey::new_by_arc(namespace_id, group_name, Arc::new(service_name)))
        }
        else{
            Err(anyhow::anyhow!("param error,service is empty"))
        }
    }
}


#[derive(Debug,Serialize,Deserialize,Default)]
#[serde(rename_all = "camelCase")]
pub struct OpsNamingQueryListResponse{
    pub count:u64,
    pub list:Vec<Arc<Instance>>,
}