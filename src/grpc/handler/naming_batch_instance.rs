#![allow(unused_imports)]

use std::sync::Arc;

use crate::{
    grpc::{
        api_model::{
            BatchInstanceRequest, Instance as ApiInstance, InstanceResponse, ERROR_CODE,
            SUCCESS_CODE,
        },
        nacos_proto::Payload,
        PayloadHandler, PayloadUtils,
    },
    naming::{
        core::{NamingActor, NamingCmd},
        model::{Instance, InstanceUpdateTag},
        NamingUtils,
    },
    now_millis_i64,
};
use actix::prelude::Addr;
use async_trait::async_trait;

const REGISTER_INSTANCE: &str = "registerInstance";

const DE_REGISTER_INSTANCE: &str = "deregisterInstance";

pub struct BatchInstanceRequestHandler {
    naming_addr: Addr<NamingActor>,
}

impl BatchInstanceRequestHandler {
    pub fn new(naming_addr: Addr<NamingActor>) -> Self {
        Self { naming_addr }
    }

    pub(crate) fn convert_to_instances(
        request: BatchInstanceRequest,
        client_id: Arc<String>,
    ) -> anyhow::Result<Vec<Instance>> {
        let mut list = vec![];
        if request.group_name.is_none() {
            return Err(anyhow::format_err!("groupName is empty!"));
        }
        let group_name = Arc::new(NamingUtils::default_group(
            request.group_name.unwrap_or_default(),
        ));
        let service_name = request.service_name.unwrap_or_default();

        let namesapce_id = Arc::new(NamingUtils::default_group(
            request.namespace.unwrap_or_default(),
        ));
        let input = request.instances;
        if let Some(instances) = input {
            let last_modified_millis = now_millis_i64();
            for input in instances {
                let service_name = if let Some(v) = input.service_name {
                    v
                } else if !service_name.is_empty() {
                    Arc::new(service_name.to_owned())
                } else {
                    return Err(anyhow::format_err!("serivceName is unvaild!"));
                };

                let mut instance = Instance {
                    id: Arc::new("".to_owned()),
                    ip: input.ip.unwrap(),
                    port: input.port,
                    weight: input.weight,
                    enabled: input.enabled,
                    healthy: input.healthy,
                    ephemeral: input.ephemeral,
                    cluster_name: NamingUtils::default_cluster(
                        input.cluster_name.unwrap_or_default(),
                    ),
                    service_name,
                    group_name: group_name.to_owned(),
                    group_service: Default::default(),
                    metadata: input.metadata,
                    last_modified_millis: last_modified_millis.to_owned(),
                    namespace_id: namesapce_id.clone(),
                    app_name: "".to_owned(),
                    from_grpc: true,
                    from_cluster: false,
                    client_id: client_id.clone(),
                };
                instance.generate_key();
                list.push(instance);
            }
            Ok(list)
        } else {
            Err(anyhow::format_err!("instance is empty"))
        }
    }
}

#[async_trait]
impl PayloadHandler for BatchInstanceRequestHandler {
    async fn handle(
        &self,
        request_payload: crate::grpc::nacos_proto::Payload,
        request_meta: crate::grpc::RequestMeta,
    ) -> anyhow::Result<Payload> {
        let body_vec = request_payload.body.unwrap_or_default().value;
        let request: BatchInstanceRequest = serde_json::from_slice(&body_vec)?;
        let request_id = request.request_id.clone();
        let mut is_de_register = false;
        if let Some(t) = &request.r#type {
            if t == DE_REGISTER_INSTANCE {
                is_de_register = true;
            }
        }
        let instances = Self::convert_to_instances(request, request_meta.connection_id)?;
        let mut response = InstanceResponse {
            request_id,
            ..Default::default()
        };
        for instance in instances {
            let cmd = if is_de_register {
                NamingCmd::Delete(instance)
            } else {
                let update_tag = InstanceUpdateTag {
                    weight: instance.weight != 1.0f32,
                    metadata: true,
                    enabled: !instance.enabled,
                    ephemeral: false,
                    from_update: false,
                };
                NamingCmd::Update(instance, Some(update_tag))
            };
            match self.naming_addr.send(cmd).await {
                Ok(_res) => {
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
        }
        Ok(PayloadUtils::build_payload(
            "BatchInstanceResponse",
            serde_json::to_string(&response)?,
        ))
    }
}
