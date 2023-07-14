use std::sync::Arc;
use std::time::SystemTime;

use actix::prelude::*;
//use tokio_stream::StreamExt;

use crate::grpc::bistream_manage::BiStreamManageResult;
use crate::grpc::nacos_proto::{request_server, Payload};
use crate::grpc::{PayloadHandler, PayloadUtils, RequestMeta};

use super::bistream_conn::BiStreamConn;
use super::bistream_manage::{BiStreamManage, BiStreamManageCmd};
use super::handler::InvokerHandler;
use super::nacos_proto::bi_request_stream_server::BiRequestStream;

pub struct RequestServerImpl {
    bistream_manage_addr: Addr<BiStreamManage>,
    invoker: InvokerHandler,
}

impl RequestServerImpl {
    pub fn new(bistream_manage_addr: Addr<BiStreamManage>, invoker: InvokerHandler) -> Self {
        Self {
            bistream_manage_addr,
            invoker,
        }
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
        let request_meta = RequestMeta {
            client_ip: remote_addr.ip().to_string(),
            connection_id: Arc::new(remote_addr.to_string()),
            ..Default::default()
        };
        let payload = request.into_inner();
        //debug
        //log::info!("client request: {}",PayloadUtils::get_payload_string(&payload));
        let request_type = PayloadUtils::get_payload_type(&payload).unwrap();
        let request_log_info = format!(
            "|grpc|client_request|{}|{}",
            &request_meta.connection_id, &request_type
        );
        let ignore_active_err = self.invoker.ignore_active_err(request_type);
        //self.bistream_manage_addr.do_send(BiStreamManageCmd::ActiveClinet(request_meta.connection_id.clone()));
        let active_result = self
            .bistream_manage_addr
            .send(BiStreamManageCmd::ActiveClinet(
                request_meta.connection_id.clone(),
            ))
            .await;
        match active_result {
            Ok(result) => {
                let result: anyhow::Result<BiStreamManageResult> = result;
                match result {
                    Ok(_) => {}
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
                    return Ok(tonic::Response::new(PayloadUtils::build_error_payload(
                        301, err_msg,
                    )));
                }
            }
        };
        let handle_result = self.invoker.handle(payload, request_meta).await;
        let duration = SystemTime::now()
            .duration_since(start)
            .unwrap_or_default()
            .as_secs_f64();
        match handle_result {
            Ok(res) => {
                //log::info!("{}|ok|{}",PayloadUtils::get_payload_header(&res));
                //debug
                //log::info!("client response: {}",PayloadUtils::get_payload_string(&res));
                log::info!("{}|ok|{}", request_log_info, duration);
                Ok(tonic::Response::new(res))
            }
            Err(e) => {
                //Err(tonic::Status::aborted(e.to_string()))
                //log::error!("request_server handler error:{:?}",e);
                log::error!("{}|err|{}|{:?}", request_log_info, duration, e);
                Ok(tonic::Response::new(PayloadUtils::build_error_payload(
                    500u16,
                    e.to_string(),
                )))
            }
        }
    }
}

pub struct BiRequestStreamServerImpl {
    bistream_manage_addr: Addr<BiStreamManage>,
}

impl BiRequestStreamServerImpl {
    pub fn new(bistream_manage_addr: Addr<BiStreamManage>) -> Self {
        Self {
            bistream_manage_addr,
        }
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
        let client_id = Arc::new(request.remote_addr().unwrap().to_string());
        let req = request.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let r_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        let conn = BiStreamConn::new(
            tx,
            client_id.clone(),
            req,
            self.bistream_manage_addr.clone(),
        );
        self.bistream_manage_addr
            .do_send(BiStreamManageCmd::AddConn(client_id, conn));
        Ok(tonic::Response::new(r_stream))
    }
}
