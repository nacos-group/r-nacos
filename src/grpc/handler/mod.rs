use std::sync::Arc;

use crate::{common::appdata::AppShareData};

use self::{
    config_change_batch_listen::ConfigChangeBatchListenRequestHandler,
    config_publish::ConfigPublishRequestHandler, config_query::ConfigQueryRequestHandler,
    config_remove::ConfigRemoveRequestHandler, naming_batch_instance::BatchInstanceRequestHandler,
    naming_instance::InstanceRequestHandler, naming_service_list::ServiceListRequestHandler, naming_service_query::ServiceQueryRequestHandler,
    naming_subscribe_service::SubscribeServiceRequestHandler, raft_route::RaftRouteRequestHandler, naming_route::NamingRouteRequestHandler,
};

use super::{
    api_model::{BaseResponse, ServerCheckResponse, SUCCESS_CODE},
    nacos_proto::Payload,
    PayloadHandler, PayloadUtils, RequestMeta,
};
use crate::grpc::handler::raft_append::RaftAppendRequestHandler;
use crate::grpc::handler::raft_snapshot::RaftSnapshotRequestHandler;
use crate::grpc::handler::raft_vote::RaftVoteRequestHandler;
use async_trait::async_trait;

pub mod config_change_batch_listen;
pub mod config_publish;
pub mod config_query;
pub mod config_remove;

pub mod converter;
pub mod naming_batch_instance;
pub mod naming_instance;
pub mod naming_service_list;
pub mod naming_service_query;
pub mod naming_subscribe_service;
pub mod naming_route;
pub mod raft_append;
pub mod raft_route;
mod raft_snapshot;
mod raft_vote;

pub(crate) const SERVER_CHECK_REQUEST: &str = "ServerCheckRequest";
pub(crate) const RAFT_APPEND_REQUEST: &str = "RaftAppendRequest";
pub(crate) const RAFT_SNAPSHOT_REQUEST: &str = "RaftSnapshotRequest";
pub(crate) const RAFT_VOTE_REQUEST: &str = "RaftVoteRequest";
pub(crate) const RAFT_ROUTE_REQUEST: &str = "RaftRouteRequest";
pub(crate) const NAMING_ROUTE_REQUEST: &str = "NamingRouteRequest";

#[derive(Default)]
pub struct InvokerHandler {
    handlers: Vec<(String, Box<dyn PayloadHandler + Send + Sync + 'static>)>,
}
pub struct HealthCheckRequestHandler {}

impl InvokerHandler {
    pub fn new() -> Self {
        let mut this = Self {
            handlers: Default::default(),
        };
        this.add_handler("HealthCheckRequest", Box::new(HealthCheckRequestHandler {}));
        this
    }

    pub fn add_handler(
        &mut self,
        url: &str,
        handler: Box<dyn PayloadHandler + Send + Sync + 'static>,
    ) {
        self.handlers.push((url.to_owned(), handler));
    }

    pub fn match_handler<'a>(
        &'a self,
        url: &str,
    ) -> Option<&'a (dyn PayloadHandler + Send + Sync + 'static)> {
        for (t, h) in &self.handlers {
            if t == url {
                return Some(h.as_ref());
            }
        }
        None
    }

    pub fn ignore_active_err(&self, t: &str) -> bool {
        SERVER_CHECK_REQUEST.eq(t)
            || RAFT_APPEND_REQUEST.eq(t)
            || RAFT_SNAPSHOT_REQUEST.eq(t)
            || RAFT_VOTE_REQUEST.eq(t)
            || RAFT_ROUTE_REQUEST.eq(t)
            || NAMING_ROUTE_REQUEST.eq(t)
    }

    pub fn add_raft_handler(&mut self, app_data: &Arc<AppShareData>) {
        self.add_handler(
            RAFT_APPEND_REQUEST,
            Box::new(RaftAppendRequestHandler::new(app_data.clone())),
        );
        self.add_handler(
            RAFT_SNAPSHOT_REQUEST,
            Box::new(RaftSnapshotRequestHandler::new(app_data.clone())),
        );
        self.add_handler(
            RAFT_VOTE_REQUEST,
            Box::new(RaftVoteRequestHandler::new(app_data.clone())),
        );
        self.add_handler(
            RAFT_ROUTE_REQUEST,
            Box::new(RaftRouteRequestHandler::new(app_data.clone())),
        );
        self.add_handler(
            NAMING_ROUTE_REQUEST,
            Box::new(NamingRouteRequestHandler::new(app_data.clone())),
        );
    }

    pub fn add_config_handler(&mut self, app_data: &Arc<AppShareData>) {
        self.add_handler(
            "ConfigQueryRequest",
            Box::new(ConfigQueryRequestHandler::new(app_data.clone())),
        );
        self.add_handler(
            "ConfigPublishRequest",
            Box::new(ConfigPublishRequestHandler::new(app_data.clone())),
        );
        self.add_handler(
            "ConfigRemoveRequest",
            Box::new(ConfigRemoveRequestHandler::new(app_data.clone())),
        );
        self.add_handler(
            "ConfigBatchListenRequest",
            Box::new(ConfigChangeBatchListenRequestHandler::new(app_data.clone())),
        );
    }

    pub fn add_naming_handler(&mut self, app_data: &Arc<AppShareData>) {
        self.add_handler(
            "InstanceRequest",
            Box::new(InstanceRequestHandler::new(app_data.clone())),
        );
        self.add_handler(
            "BatchInstanceRequest",
            Box::new(BatchInstanceRequestHandler::new(app_data.clone())),
        );
        self.add_handler(
            "SubscribeServiceRequest",
            Box::new(SubscribeServiceRequestHandler::new(app_data.clone())),
        );
        self.add_handler(
            "ServiceQueryRequest",
            Box::new(ServiceQueryRequestHandler::new(app_data.clone())),
        );
        self.add_handler(
            "ServiceListRequest",
            Box::new(ServiceListRequestHandler::new(app_data.clone())),
        );
    }
}

#[async_trait]
impl PayloadHandler for InvokerHandler {
    async fn handle(
        &self,
        request_payload: super::nacos_proto::Payload,
        request_meta: RequestMeta,
    ) -> anyhow::Result<Payload> {
        if let Some(url) = PayloadUtils::get_payload_type(&request_payload) {
            if SERVER_CHECK_REQUEST.eq(url) {
                let response = ServerCheckResponse {
                    result_code: SUCCESS_CODE,
                    connection_id: Some(request_meta.connection_id.as_ref().to_owned()),
                    ..Default::default()
                };
                return Ok(PayloadUtils::build_payload(
                    "ServerCheckResponse",
                    serde_json::to_string(&response)?,
                ));
            }
            //println!("InvokerHandler type:{}",url);
            if let Some(handler) = self.match_handler(url) {
                return handler.handle(request_payload, request_meta).await;
            }
            log::warn!("InvokerHandler not fund handler,type:{}", url);
            return Ok(PayloadUtils::build_error_payload(
                302u16,
                format!("{} RequestHandler Not Found", url),
            ));
        }
        Ok(PayloadUtils::build_error_payload(
            302u16,
            "empty type url".to_owned(),
        ))
    }
}

#[async_trait]
impl PayloadHandler for HealthCheckRequestHandler {
    async fn handle(
        &self,
        _request_payload: super::nacos_proto::Payload,
        _request_meta: RequestMeta,
    ) -> anyhow::Result<Payload> {
        //println!("HealthCheckRequest");
        let response = BaseResponse::build_success_response();
        return Ok(PayloadUtils::build_payload(
            "HealthCheckResponse",
            serde_json::to_string(&response)?,
        ));
    }
}
