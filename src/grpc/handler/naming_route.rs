#![allow(unused_imports)]

use crate::common::constant::{GRPC_HEAD_KEY_CLUSTER_ID, GRPC_HEAD_KEY_TRACE_ID};
use crate::common::log_utils::LogArgs;
use crate::grpc::{HandleLogArgs, HandlerResult};
use crate::naming::cluster::model::{
    SnapshotDataInfo, SnapshotForReceive, SyncBatchDataInfo, SyncBatchForReceive,
};
use crate::naming::model::{Instance, InstanceKey};
use crate::{
    common::appdata::AppShareData,
    grpc::{nacos_proto::Payload, PayloadHandler, PayloadUtils, RequestMeta},
    naming::cluster::{handle_naming_route, model::NamingRouteRequest},
};
use async_trait::async_trait;
use std::convert::TryFrom;
use std::sync::Arc;

pub struct NamingRouteRequestHandler {
    app_data: Arc<AppShareData>,
}

impl NamingRouteRequestHandler {
    pub fn new(app_data: Arc<AppShareData>) -> Self {
        Self { app_data }
    }
}

///用于debug模式构建参数日志
#[cfg(feature = "debug")]
impl NamingRouteRequestHandler {
    fn instance_arg<'a, 'b>(args: &'a mut LogArgs<'b>, instance: &'b Instance) {
        args.add_str(instance.namespace_id.as_str())
            .add_key_split()
            .add_str(instance.group_name.as_str())
            .add_key_split()
            .add_str(instance.service_name.as_str())
            .add_item_split()
            .add_str(instance.ip.as_str())
            .add_key_split()
            .add_string(instance.port.to_string())
            .add_item_split()
            .add_str("healthy")
            .add_key_split()
            .add_string(instance.healthy.to_string())
            .add_item_split()
            .add_str("from_cluster")
            .add_key_split()
            .add_string(instance.from_cluster.to_string())
            .add_item_split()
            .add_str("client_id")
            .add_key_split()
            .add_str(instance.client_id.as_str());
    }
    fn get_instance_arg(instance: &Instance) -> String {
        let mut args = LogArgs::new();
        Self::instance_arg(&mut args, instance);
        args.to_string()
    }

    fn do_get_instance_key_arg(args: &mut LogArgs, item: &InstanceKey) {
        args.add_string(item.namespace_id.as_str().to_string())
            .add_key_split()
            .add_string(item.group_name.as_str().to_string())
            .add_key_split()
            .add_string(item.service_name.as_str().to_string())
            .add_item_split()
            .add_string(item.ip.as_str().to_string())
            .add_key_split()
            .add_string(item.port.to_string());
    }

    fn get_instance_key_arg(item: &InstanceKey) -> String {
        let mut args = LogArgs::new();
        Self::do_get_instance_key_arg(&mut args, item);
        args.to_string()
    }

    fn snapshot_args(data: &[u8]) -> anyhow::Result<String> {
        let mut args = LogArgs::new();
        let snapshot = SnapshotDataInfo::from_bytes(data)?;
        let snapshot_receive = SnapshotForReceive::try_from(snapshot)?;
        args.add_str("route_index")
            .add_key_split()
            .add_string(snapshot_receive.route_index.to_string())
            .add_item_split();
        args.add_str("node_count")
            .add_key_split()
            .add_string(snapshot_receive.node_count.to_string())
            .add_item_split();
        for item in &snapshot_receive.instances {
            Self::instance_arg(&mut args, item);
            args.add_item_split();
        }
        Ok(args.to_string())
    }

    fn get_req_args(request: &NamingRouteRequest) -> anyhow::Result<String> {
        let mut args = LogArgs::new();
        args.add_str("NamingRoute")
            .add_key_split()
            .add_str(request.get_sub_name())
            .add_item_split();
        match request {
            NamingRouteRequest::Ping(_) => {}
            NamingRouteRequest::UpdateInstance { instance, tag } => {
                args.add_string(Self::get_instance_arg(instance));
                if let Some(tag) = tag {
                    args.add_item_split()
                        .add_str("tag")
                        .add_key_split()
                        .add_string(format!("{:?}", tag));
                } else {
                    args.add_item_split()
                        .add_str("tag")
                        .add_key_split()
                        .add_str("None");
                }
            }
            NamingRouteRequest::RemoveInstance { instance } => {
                args.add_string(Self::get_instance_arg(instance));
            }
            NamingRouteRequest::SyncUpdateInstance { instance } => {
                args.add_item_split()
                    .add_string(Self::get_instance_arg(instance));
            }
            NamingRouteRequest::SyncRemoveInstance { instance } => {
                args.add_item_split()
                    .add_string(Self::get_instance_arg(instance));
            }
            NamingRouteRequest::SyncUpdateService { .. } => {}
            NamingRouteRequest::SyncBatchInstances(data) => {
                let snapshot = SyncBatchDataInfo::from_bytes(&data)?;
                let batch_receive = SyncBatchForReceive::try_from(snapshot)?;
                let mut tmp_args = LogArgs::new();
                tmp_args.add_str("UPDATE").add_item_split();
                for instance in batch_receive.update_instances {
                    tmp_args.add_string(Self::get_instance_arg(&instance));
                    tmp_args.add_item_split();
                }
                tmp_args.add_str("REMOVE").add_item_split();
                for instance in batch_receive.remove_instances {
                    tmp_args.add_string(Self::get_instance_arg(&instance));
                    tmp_args.add_item_split();
                }
                args.merge_args(tmp_args);
            }
            NamingRouteRequest::RemoveClientId { client_id } => {
                args.add_string(client_id.as_str().to_string());
            }
            NamingRouteRequest::QuerySnapshot { index, len } => {
                args.add_str("index")
                    .add_key_split()
                    .add_string(index.to_string())
                    .add_item_split()
                    .add_str("len")
                    .add_key_split()
                    .add_string(len.to_string());
            }
            NamingRouteRequest::Snapshot(data) => {
                if let Ok(arg) = Self::snapshot_args(data) {
                    args.add_string(arg);
                }
            }
            NamingRouteRequest::MetricsTimelineQuery(_) => {}
            NamingRouteRequest::SyncDistroClientInstances(data) => {
                let mut tmp_args = LogArgs::new();
                for (key, items) in data.iter() {
                    tmp_args
                        .add_str("GROUP")
                        .add_key_split()
                        .add_str(key.as_str())
                        .add_item_split();
                    for item in items {
                        tmp_args.add_string(Self::get_instance_key_arg(item));
                        tmp_args.add_item_split();
                    }
                }
                args.merge_args(tmp_args);
            }
            NamingRouteRequest::QueryDistroInstanceSnapshot(instance_key) => {
                let mut tmp_args = LogArgs::new();
                for item in instance_key {
                    Self::do_get_instance_key_arg(&mut tmp_args, item);
                    tmp_args.add_item_split();
                }
                args.merge_args(tmp_args);
            }
            NamingRouteRequest::QueryServiceSubscriberPage(param) => {}
            NamingRouteRequest::McpMessages { .. } => {}
        }
        Ok(args.to_string())
    }
}

#[async_trait]
impl PayloadHandler for NamingRouteRequestHandler {
    fn get_log_args(
        &self,
        request_payload: &Payload,
        _request_meta: &RequestMeta,
    ) -> HandleLogArgs {
        let mut args = LogArgs::new();
        args.add_str(GRPC_HEAD_KEY_TRACE_ID).add_key_split();
        if let Some(Some(v)) = request_payload
            .metadata
            .as_ref()
            .map(|e| e.headers.get(GRPC_HEAD_KEY_TRACE_ID))
        {
            args.add_str(v);
        }
        args.add_item_split()
            .add_str(GRPC_HEAD_KEY_CLUSTER_ID)
            .add_key_split();
        if let Some(Some(v)) = request_payload
            .metadata
            .as_ref()
            .map(|e| e.headers.get(GRPC_HEAD_KEY_CLUSTER_ID))
        {
            args.add_str(v);
        }
        args.add_item_split();
        #[cfg(feature = "debug")]
        if let Some(v) = request_payload.body.as_ref() {
            let request: Result<NamingRouteRequest, _> = serde_json::from_slice(&v.value);
            if let Ok(request) = &request {
                if let NamingRouteRequest::Ping(_) = request {
                    return HandleLogArgs::Ignore;
                }
                if let Ok(v) = Self::get_req_args(request) {
                    args.add_string(v);
                }
            }
        }
        #[cfg(not(feature = "debug"))]
        if let Some(Some(v)) = request_payload.metadata.as_ref().map(|e| {
            e.headers
                .get(crate::common::constant::GRPC_HEAD_KEY_SUB_NAME)
        }) {
            if v == "Ping" {
                return HandleLogArgs::Ignore;
            }
            args.add_str("NamingRoute")
                .add_key_split()
                .add_str(v)
                .add_item_split();
        }
        HandleLogArgs::Arg(format!("{}", args))
    }

    async fn handle(
        &self,
        request_payload: Payload,
        _request_meta: RequestMeta,
    ) -> anyhow::Result<HandlerResult> {
        let body_vec = request_payload.body.unwrap_or_default().value;
        let request: NamingRouteRequest = serde_json::from_slice(&body_vec)?;
        let res = handle_naming_route(
            &self.app_data,
            request,
            request_payload
                .metadata
                .map(|e| e.headers)
                .unwrap_or_default(),
        )
        .await?;
        let value = serde_json::to_string(&res)?;
        let payload = PayloadUtils::build_payload("NamingRouteResponse", value);
        Ok(HandlerResult::success(payload))
    }
}
