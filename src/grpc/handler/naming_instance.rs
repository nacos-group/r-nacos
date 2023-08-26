#![allow(unused_imports)]

use std::sync::{
    atomic::{AtomicBool, AtomicI64},
    Arc,
};

use crate::{
    grpc::{
        api_model::{
            Instance as ApiInstance, InstanceRequest, InstanceResponse, ERROR_CODE, SUCCESS_CODE,
        },
        nacos_proto::Payload,
        PayloadHandler, PayloadUtils,
    },
    naming::{
        core::{NamingActor, NamingCmd},
        model::{Instance, InstanceUpdateTag},
        NamingUtils,
    },
    now_millis_i64, common::appdata::AppShareData,
};
use actix::prelude::Addr;
use async_trait::async_trait;

const REGISTER_INSTANCE: &str = "registerInstance";

const DE_REGISTER_INSTANCE: &str = "deregisterInstance";

pub struct InstanceRequestHandler {
    app_data: Arc<AppShareData>,
}

impl InstanceRequestHandler {
    pub fn new(app_data: Arc<AppShareData>) -> Self {
        Self { app_data }
    }

    pub(crate) fn convert_to_instance(
        request: InstanceRequest,
        client_id: Arc<String>,
    ) -> anyhow::Result<Instance> {
        let input = request.instance;
        if let Some(input) = input {
            if request.group_name.is_none() {
                return Err(anyhow::format_err!("groupName is empty!"));
            }
            let group_name = Arc::new(NamingUtils::default_group(
                request.group_name.unwrap_or_default(),
            ));
            let service_name = if let Some(v) = input.service_name {
                v
            } else if let Some(v) = request.service_name {
                Arc::new(v)
            } else {
                return Err(anyhow::format_err!("serivceName is unvaild!"));
            };
            let mut instance = Instance {
                id: Default::default(),
                ip: input.ip.unwrap(),
                port: input.port,
                weight: input.weight,
                enabled: input.enabled,
                healthy: input.healthy,
                ephemeral: input.ephemeral,
                cluster_name: NamingUtils::default_cluster(input.cluster_name.unwrap_or_default()),
                service_name,
                group_name,
                group_service: Default::default(),
                metadata: input.metadata,
                last_modified_millis: now_millis_i64(),
                namespace_id: Arc::new(NamingUtils::default_namespace(
                    request.namespace.unwrap_or_default(),
                )),
                app_name: "".to_owned(),
                from_grpc: true,
                from_cluster: false,
                client_id,
            };
            instance.generate_key();
            Ok(instance)
        } else {
            Err(anyhow::format_err!("instance is empty"))
        }
    }
}

#[async_trait]
impl PayloadHandler for InstanceRequestHandler {
    async fn handle(
        &self,
        request_payload: crate::grpc::nacos_proto::Payload,
        request_meta: crate::grpc::RequestMeta,
    ) -> anyhow::Result<Payload> {
        let body_vec = request_payload.body.unwrap_or_default().value;
        let request: InstanceRequest = serde_json::from_slice(&body_vec)?;
        let request_id = request.request_id.clone();
        let mut is_de_register = false;
        if let Some(t) = &request.r#type {
            if t == DE_REGISTER_INSTANCE {
                is_de_register = true;
            }
        }
        let instance = Self::convert_to_instance(request, request_meta.connection_id)?;
        let res= if is_de_register {
            self.app_data.naming_route.delete_instance(instance).await
        } else {
            let update_tag = InstanceUpdateTag {
                weight: instance.weight != 1.0f32,
                metadata: true,
                enabled: !instance.enabled,
                ephemeral: false,
                from_update: false,
            };
            self.app_data.naming_route.update_instance(instance, Some(update_tag)).await
        };
        let mut response = InstanceResponse {
            request_id,
            ..Default::default()
        };
        match res {
            Ok(_) => {
                //let res:ConfigResult = res.unwrap();
                response.result_code = SUCCESS_CODE;
                if is_de_register {
                    response.r#type = Some(DE_REGISTER_INSTANCE.to_string());
                } else {
                    response.r#type = Some(REGISTER_INSTANCE.to_string());
                }
            }
            Err(err) => {
                response.result_code = ERROR_CODE;
                response.error_code = 500u16;
                response.message = Some(err.to_string());
                return Ok(PayloadUtils::build_payload(
                    "ErrorResponse",
                    serde_json::to_string(&response)?,
                ));
            }
        };
        Ok(PayloadUtils::build_payload(
            "InstanceResponse",
            serde_json::to_string(&response)?,
        ))
    }
}
