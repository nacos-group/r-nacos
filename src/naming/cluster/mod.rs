//distor cluster

use self::{
    model::{
        NamingRouteRequest, NamingRouterResponse, ProcessRange, SnapshotDataInfo,
        SnapshotForReceive, SyncBatchDataInfo, SyncBatchForReceive,
    },
    node_manage::{NodeManageRequest, NodeManageResponse},
};
use crate::common::constant::GRPC_HEAD_KEY_CLUSTER_ID;
use crate::mcp::model::actor_model::{McpManagerReq, McpManagerResult};
use crate::mcp::model::sse_model::SseStreamManageAsyncCmd;
use crate::mcp::sse_manage::SseConnUtils;
use crate::metrics::model::{MetricsRequest, MetricsResponse};
use crate::naming::cluster::model::SnapshotForSend;
use crate::naming::model::{DistroData, Instance};
use crate::{
    common::appdata::AppShareData,
    naming::core::{NamingCmd, NamingResult},
    openapi,
};
use std::{
    collections::{HashMap, HashSet},
    convert::TryFrom,
    sync::Arc,
};

pub mod instance_delay_notify;
pub mod model;
pub mod node_manage;
pub mod route;
pub mod sync_sender;

fn get_cluster_id(extend_info: HashMap<String, String>) -> anyhow::Result<u64> {
    if let Some(id_str) = extend_info.get(GRPC_HEAD_KEY_CLUSTER_ID) {
        match id_str.parse() {
            Ok(id) => Ok(id),
            Err(_err) => Err(anyhow::anyhow!("cluster_id can't parse to u64,{}", id_str)),
        }
    } else {
        Err(anyhow::anyhow!("extend_info not found cluster_id"))
    }
}

pub async fn handle_naming_route(
    app: &Arc<AppShareData>,
    req: NamingRouteRequest,
    extend_info: HashMap<String, String>,
) -> anyhow::Result<NamingRouterResponse> {
    #[cfg(feature = "debug")]
    log::info!("handle_naming_route req:{:?}", &req);
    match req {
        NamingRouteRequest::Ping(cluster_id) => {
            //更新node_id节点活跃状态
            app.naming_node_manage.active_node(cluster_id);
        }
        NamingRouteRequest::UpdateInstance { instance, tag } => {
            let cmd = NamingCmd::Update(instance, tag);
            let _: NamingResult = app.naming_addr.send(cmd).await??;
        }
        NamingRouteRequest::RemoveInstance { instance } => {
            let cmd = NamingCmd::Delete(instance);
            let _: NamingResult = app.naming_addr.send(cmd).await??;
        }
        NamingRouteRequest::SyncUpdateService { service } => {
            let cluster_id = get_cluster_id(extend_info)?;
            app.naming_addr
                .do_send(NamingCmd::UpdateServiceFromCluster(service));
            app.naming_node_manage.active_node(cluster_id);
        }
        NamingRouteRequest::SyncUpdateInstance { mut instance } => {
            let cluster_id = get_cluster_id(extend_info)?;
            reset_cluster_info(cluster_id, &mut instance);
            if instance.from_cluster == cluster_id {
                app.naming_inner_node_manage
                    .do_send(NodeManageRequest::AddClientId(
                        cluster_id,
                        instance.client_id.clone(),
                    ));
            }
            let cmd = NamingCmd::Update(instance, None);
            let _: NamingResult = app.naming_addr.send(cmd).await??;
        }
        NamingRouteRequest::SyncRemoveInstance { mut instance } => {
            let cluster_id = get_cluster_id(extend_info)?;
            app.naming_node_manage.active_node(cluster_id);
            instance.from_cluster = cluster_id;
            //reset_cluster_info(cluster_id, &mut instance);
            let cmd = NamingCmd::Delete(instance);
            let _: NamingResult = app.naming_addr.send(cmd).await??;
        }
        NamingRouteRequest::SyncBatchInstances(data) => {
            let cluster_id = get_cluster_id(extend_info)?;
            let snapshot = SyncBatchDataInfo::from_bytes(&data)?;
            let mut batch_receive = SyncBatchForReceive::try_from(snapshot)?;
            let mut client_sets = HashSet::new();
            for instance in &mut batch_receive.update_instances {
                reset_cluster_info(cluster_id, instance);
                if instance.from_cluster == cluster_id {
                    client_sets.insert(instance.client_id.clone());
                }
            }
            /*
            for instance in &mut batch_receive.remove_instances {
                reset_cluster_info(cluster_id, instance);
            }
             */
            if !client_sets.is_empty() {
                app.naming_inner_node_manage
                    .do_send(NodeManageRequest::AddClientIds(cluster_id, client_sets));
            }
            if !batch_receive.remove_instances.is_empty() {
                app.naming_addr
                    .do_send(NamingCmd::DeleteBatch(batch_receive.remove_instances));
            }
            if !batch_receive.update_instances.is_empty() {
                app.naming_addr
                    .do_send(NamingCmd::UpdateBatch(batch_receive.update_instances));
            }
        }
        NamingRouteRequest::RemoveClientId { client_id } => {
            app.naming_inner_node_manage
                .do_send(NodeManageRequest::RemoveClientId(client_id));
        }
        NamingRouteRequest::QuerySnapshot { index, len } => {
            //请求 snapshot data
            let cluster_id = get_cluster_id(extend_info)?;
            log::info!("query snapshot from {}", &cluster_id);
            let cmd = NodeManageRequest::QueryOwnerRange(ProcessRange::new(index, len));
            let resp: NodeManageResponse = app.naming_inner_node_manage.send(cmd).await??;
            if let NodeManageResponse::OwnerRange(ranges) = resp {
                let cmd = NamingCmd::QuerySnapshot(ranges);
                let result: NamingResult = app.naming_addr.send(cmd).await??;
                if let NamingResult::Snapshot(snapshot) = result {
                    //发送 snapshot data
                    log::info!("send snapshot to {}", &cluster_id);
                    app.naming_inner_node_manage
                        .do_send(NodeManageRequest::SendSnapshot(cluster_id, snapshot));
                }
            }
            app.naming_node_manage.active_node(cluster_id);
        }
        NamingRouteRequest::Snapshot(data) => {
            let cluster_id = get_cluster_id(extend_info)?;
            //接收snapshot data
            let snapshot = SnapshotDataInfo::from_bytes(&data)?;
            log::info!(
                "receive snapshot from {},instance size:{}",
                &cluster_id,
                snapshot.instances.len()
            );
            let mut snapshot_receive = SnapshotForReceive::try_from(snapshot)?;
            let mut client_sets = HashSet::new();
            for instance in &mut snapshot_receive.instances {
                reset_cluster_info(cluster_id, instance);
                if instance.from_cluster == cluster_id {
                    client_sets.insert(instance.client_id.clone());
                }
            }
            if !client_sets.is_empty() {
                app.naming_inner_node_manage
                    .do_send(NodeManageRequest::AddClientIds(cluster_id, client_sets));
            }
            //增量数据
            app.naming_addr
                .do_send(NamingCmd::ReceiveSnapshot(snapshot_receive));
        }
        NamingRouteRequest::MetricsTimelineQuery(param) => {
            let resp = app
                .metrics_manager
                .send(MetricsRequest::TimelineQuery(param))
                .await??;
            if let MetricsResponse::TimelineResponse(mut resp) = resp {
                resp.from_node_id = app.sys_config.raft_node_id;
                return Ok(NamingRouterResponse::MetricsTimeLineResponse(resp));
            }
        }
        NamingRouteRequest::SyncDistroClientInstances(client_instance) => {
            let cluster_id = get_cluster_id(extend_info)?;
            let client_ids: HashSet<Arc<String>> = client_instance.keys().cloned().collect();
            //清理不存在的client_id数据
            app.naming_inner_node_manage
                .send(NodeManageRequest::RemoveDiffClientIds(
                    cluster_id, client_ids,
                ))
                .await??;
            let res = app
                .naming_addr
                .send(NamingCmd::DiffGrpcDistroData {
                    data: DistroData::ClientInstances(client_instance),
                    cluster_id,
                })
                .await??;
            if let NamingResult::DiffDistroData(DistroData::DiffClientInstances(diff_data)) = res {
                log::info!(
                    "sync distro client|DiffDistroData,{},count:{}",
                    &cluster_id,
                    diff_data.len()
                );
                if !diff_data.is_empty() {
                    let cmd = NodeManageRequest::QueryDiffClientInstances(cluster_id, diff_data);
                    app.naming_inner_node_manage.send(cmd).await??;
                }
            }
        }
        NamingRouteRequest::QueryDistroInstanceSnapshot(instances) => {
            if instances.is_empty() {
                return Ok(NamingRouterResponse::None);
            }
            let cluster_id = get_cluster_id(extend_info)?;
            let res = app
                .naming_addr
                .send(NamingCmd::QueryDistroInstanceSnapshot(instances))
                .await??;
            if let NamingResult::DistroInstancesSnapshot(instances) = res {
                if !instances.is_empty() {
                    let snapshot = SnapshotForSend {
                        route_index: 0,
                        node_count: 0,
                        mode: 0,
                        services: vec![],
                        instances,
                    };
                    //发送 snapshot data给请求方
                    let cmd = NodeManageRequest::SendSnapshot(cluster_id, snapshot);
                    app.naming_inner_node_manage.send(cmd).await??;
                }
            }
        }
        NamingRouteRequest::QueryServiceSubscriberPage(param) => {
            let resp = app
                .naming_addr
                .send(NamingCmd::QueryServiceSubscribersPageV2(param))
                .await??;
            if let NamingResult::ServiceSubscribersPage((total, list)) = resp {
                return Ok(NamingRouterResponse::ServiceSubscribersPage((total, list)));
            }
        }
        NamingRouteRequest::McpMessages {
            server_key,
            session_id,
            request,
            headers,
        } => {
            let mcp_server = if let Ok(Ok(McpManagerResult::ServerInfo(Some(server)))) = app
                .mcp_manager
                .send(McpManagerReq::GetServerByKey(server_key.clone()))
                .await
            {
                server
            } else {
                return Err(anyhow::anyhow!("McpServer not found"));
            };
            let mut ref_headers = HashMap::new();
            for (key, value) in headers.iter() {
                ref_headers.insert(key.as_str(), value.as_bytes());
            }
            match openapi::mcp::api::handle_request(
                app,
                request,
                &mcp_server,
                session_id.as_ref(),
                ref_headers,
            )
            .await
            {
                Ok(result) => {
                    let message = SseConnUtils::create_sse_message(&result);
                    app.sse_stream_manager
                        .send(SseStreamManageAsyncCmd::SendMessage(
                            session_id.clone(),
                            message,
                        ))
                        .await
                        .ok();
                }
                Err(_e) => {}
            }
        }
    };
    Ok(NamingRouterResponse::None)
}

fn reset_cluster_info(cluster_id: u64, instance: &mut Instance) {
    /*
    if instance.client_id.is_empty() && cluster_id > 0 {
        instance.client_id = Arc::new(format!("{}_G", &cluster_id));
    }
    */
    if instance.from_cluster == 0 {
        instance.from_cluster = cluster_id;
    }
}
