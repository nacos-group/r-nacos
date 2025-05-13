use std::sync::Arc;
use std::time::SystemTime;

use crate::common::appdata::AppShareData;
use crate::common::constant::{
    ACCESS_TOKEN_HEADER, AUTHORIZATION_HEADER, EMPTY_ARC_STRING, EMPTY_CLIENT_VERSION,
};
use crate::common::model::TokenSession;
use actix::prelude::*;
//use tokio_stream::StreamExt;

use crate::grpc::bistream_manage::BiStreamManageResult;
use crate::grpc::nacos_proto::{request_server, Payload};
use crate::grpc::{PayloadHandler, PayloadUtils, RequestMeta};
use crate::metrics::metrics_key::MetricsKey;
use crate::metrics::model::{MetricsItem, MetricsRecord, MetricsRequest};
use crate::raft::cache::model::{CacheKey, CacheType, CacheValue};
use crate::raft::cache::{CacheManager, CacheManagerReq, CacheManagerResult};

use super::bistream_conn::BiStreamConn;
use super::bistream_manage::BiStreamManageCmd;
use super::handler::{InvokerHandler, CLUSTER_TOKEN};
use super::nacos_proto::bi_request_stream_server::BiRequestStream;

pub struct RequestServerImpl {
    app: Arc<AppShareData>,
    invoker: InvokerHandler,
}

impl RequestServerImpl {
    pub fn new(app: Arc<AppShareData>, invoker: InvokerHandler) -> Self {
        Self { app, invoker }
    }
    async fn fill_token_session(
        &self,
        payload: &Payload,
        request_meta: &mut RequestMeta,
    ) -> anyhow::Result<()> {
        let token = if let Some(meta) = &payload.metadata {
            if let Some(v) = meta.headers.get(ACCESS_TOKEN_HEADER) {
                Arc::new(v.to_owned())
            } else if let Some(v) = meta.headers.get(AUTHORIZATION_HEADER) {
                Arc::new(v.to_owned())
            } else {
                EMPTY_ARC_STRING.clone()
            }
        } else {
            EMPTY_ARC_STRING.clone()
        };
        if self.app.sys_config.openapi_enable_auth && !token.is_empty() {
            if let Ok(Some(session)) = get_user_session(
                &self.app.cache_manager,
                CacheManagerReq::Get(CacheKey::new(CacheType::ApiTokenSession, token.clone())),
            )
            .await
            {
                request_meta.token_session = Some(session);
            }
        } else if !self.app.sys_config.cluster_token.is_empty() {
            if let Some(Some(token)) = payload
                .metadata
                .as_ref()
                .map(|e| e.headers.get(CLUSTER_TOKEN))
            {
                request_meta.cluster_token_is_valid =
                    token == self.app.sys_config.cluster_token.as_ref();
            }
        }
        Ok(())
    }

    fn record_req_metrics(&self, duration: f64, _success: bool) {
        self.app
            .metrics_manager
            .do_send(MetricsRequest::BatchRecord(vec![
                MetricsItem::new(
                    MetricsKey::GrpcRequestHandleRtHistogram,
                    MetricsRecord::HistogramRecord(duration as f32 * 1000f32),
                ),
                MetricsItem::new(
                    MetricsKey::GrpcRequestTotalCount,
                    MetricsRecord::CounterInc(1),
                ),
            ]));
    }
}

#[tonic::async_trait]
impl request_server::Request for RequestServerImpl {
    async fn request(
        &self,
        request: tonic::Request<Payload>,
    ) -> Result<tonic::Response<Payload>, tonic::Status> {
        let start = SystemTime::now();
        let remote_addr = request.remote_addr().unwrap();
        let payload = request.into_inner();
        let mut request_meta = RequestMeta {
            client_ip: remote_addr.ip().to_string(),
            client_version: EMPTY_CLIENT_VERSION.clone(),
            connection_id: Arc::new(format!(
                "{}_{}",
                self.app.sys_config.raft_node_id, &remote_addr
            )),
            ..Default::default()
        };
        let request_type = PayloadUtils::get_payload_type(&payload).unwrap();
        let request_log_info = format!(
            "|grpc|client_request|{}|{}",
            &request_meta.connection_id, &request_type
        );
        let ignore_active_err = self.invoker.ignore_active_err(request_type);
        //self.bistream_manage_addr.do_send(BiStreamManageCmd::ActiveClinet(request_meta.connection_id.clone()));
        let active_result = self
            .app
            .bi_stream_manage
            .send(BiStreamManageCmd::ActiveClinet(
                request_meta.connection_id.clone(),
            ))
            .await;
        match active_result {
            Ok(result) => {
                let result: anyhow::Result<BiStreamManageResult> = result;
                match result {
                    Ok(conn_result) => {
                        if let BiStreamManageResult::ClientInfo(client_version) = conn_result {
                            request_meta.client_version = client_version;
                        }
                    }
                    Err(err) => {
                        if !ignore_active_err {
                            let err_msg = err.to_string();
                            //log::error!("{}",&err_msg);
                            //return Err(tonic::Status::unknown(err_msg));
                            let duration = SystemTime::now()
                                .duration_since(start)
                                .unwrap_or_default()
                                .as_secs_f64();
                            log::error!("{}|err|{}|{}", request_log_info, duration, &err_msg);
                            self.record_req_metrics(duration, false);
                            return Ok(tonic::Response::new(PayloadUtils::build_error_payload(
                                301, err_msg,
                            )));
                        }
                    }
                }
            }
            Err(err) => {
                if !ignore_active_err {
                    let err_msg = err.to_string();
                    //log::error!("{}",err_msg);
                    //return Err(tonic::Status::unknown(err_msg));
                    let duration = SystemTime::now()
                        .duration_since(start)
                        .unwrap_or_default()
                        .as_secs_f64();
                    log::error!("{}|err|{}|{}", request_log_info, duration, &err_msg);
                    self.record_req_metrics(duration, false);
                    return Ok(tonic::Response::new(PayloadUtils::build_error_payload(
                        301, err_msg,
                    )));
                }
            }
        };
        //debug
        #[cfg(feature = "debug")]
        log::info!(
            "client request: {} | version: {}",
            PayloadUtils::get_payload_string(&payload),
            &request_meta.client_version
        );
        self.fill_token_session(&payload, &mut request_meta)
            .await
            .ok();
        let args = self.invoker.get_log_args(&payload, &request_meta);
        let handle_result = self.invoker.handle(payload, request_meta).await;
        let duration = SystemTime::now()
            .duration_since(start)
            .unwrap_or_default()
            .as_secs_f64();
        match handle_result {
            Ok(res) => {
                //log::info!("{}|ok|{}",PayloadUtils::get_payload_header(&res.payload));
                //debug
                //log::info!("client response: {}",PayloadUtils::get_payload_string(&res.payload));
                if !res.success {
                    let msg = if let Some(m) = &res.message {
                        m.as_str()
                    } else {
                        ""
                    };
                    log::error!("{}|err|{}|{}|{}", request_log_info, duration, &args, msg);
                    self.record_req_metrics(duration, false);
                } else if duration < 1f64 {
                    if args.enable_log() {
                        log::info!("{}|ok|{}|{}", request_log_info, duration, &args);
                    }
                    self.record_req_metrics(duration, true);
                } else {
                    if args.enable_log() {
                        //slow request handle
                        log::warn!("{}|ok|{}|{}", request_log_info, duration, &args);
                    }
                    self.record_req_metrics(duration, true);
                }
                Ok(tonic::Response::new(res.payload))
            }
            Err(e) => {
                //Err(tonic::Status::aborted(e.to_string()))
                //log::error!("request_server handler error:{:?}",e);
                log::error!("{}|err|{}|{}|{}", request_log_info, duration, &args, e);
                self.record_req_metrics(duration, false);
                Ok(tonic::Response::new(PayloadUtils::build_error_payload(
                    500u16,
                    e.to_string(),
                )))
            }
        }
    }
}

pub struct BiRequestStreamServerImpl {
    app: Arc<AppShareData>,
}

impl BiRequestStreamServerImpl {
    pub fn new(app: Arc<AppShareData>) -> Self {
        Self { app }
    }
}

#[tonic::async_trait]
impl BiRequestStream for BiRequestStreamServerImpl {
    type requestBiStreamStream =
        tokio_stream::wrappers::ReceiverStream<Result<Payload, tonic::Status>>;

    async fn request_bi_stream(
        &self,
        request: tonic::Request<tonic::Streaming<Payload>>,
    ) -> Result<tonic::Response<Self::requestBiStreamStream>, tonic::Status> {
        let client_id = Arc::new(format!(
            "{}_{}",
            self.app.sys_config.raft_node_id,
            &request.remote_addr().unwrap()
        ));
        let req = request.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let r_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        let conn = BiStreamConn::new(
            tx,
            client_id.clone(),
            req,
            self.app.bi_stream_manage.clone(),
        );
        self.app
            .bi_stream_manage
            .do_send(BiStreamManageCmd::AddConn(client_id, conn));
        Ok(tonic::Response::new(r_stream))
    }
}

async fn get_user_session(
    cache_manager: &Addr<CacheManager>,
    req: CacheManagerReq,
) -> anyhow::Result<Option<Arc<TokenSession>>> {
    match cache_manager.send(req).await?? {
        CacheManagerResult::Value(CacheValue::ApiTokenSession(session)) => Ok(Some(session)),
        _ => Ok(None),
    }
}
