use std::sync::Arc;

use actix::prelude::*;
//use tokio_stream::StreamExt;

use crate::grpc::{PayloadUtils, PayloadHandler, RequestMeta};
use crate::grpc::nacos_proto::{request_server, Payload};

use super::bistream_conn::BiStreamConn;
use super::bistream_manage::{BiStreamManage, BiStreamManageCmd};
use super::handler::InvokerHandler;
use super::nacos_proto::bi_request_stream_server::BiRequestStream;


pub struct RequestServerImpl{
    bistream_manage_addr:Addr<BiStreamManage>,
    invoker:InvokerHandler,
}

impl RequestServerImpl {
    pub fn new(bistream_manage_addr:Addr<BiStreamManage>,invoker:InvokerHandler) -> Self {
        Self { bistream_manage_addr,invoker}
    }
}

#[tonic::async_trait]
impl request_server::Request for RequestServerImpl {
    async fn request(
        &self,
        request: tonic::Request<Payload>,
    ) -> Result<tonic::Response<Payload>,tonic::Status> {
        let remote_addr = request.remote_addr().unwrap();
        let mut request_meta = RequestMeta::default();
        request_meta.client_ip = remote_addr.ip().to_string();
        request_meta.connection_id = Arc::new(remote_addr.to_string());
        let payload = request.into_inner();
        self.bistream_manage_addr.do_send(BiStreamManageCmd::ActiveClinet(request_meta.connection_id.clone()));
        println!("request_server request:{},client_id:{}",PayloadUtils::get_payload_string(&payload),&request_meta.connection_id);
        match self.invoker.handle(payload,request_meta).await {
            Ok(res) => {
                println!("request_server response:{}",PayloadUtils::get_payload_string(&res));
                Ok(tonic::Response::new(res))
            },
            Err(e) => {
                //Err(tonic::Status::aborted(e.to_string()))
                log::error!("request_server handler error:{:?}",e);
                Ok(tonic::Response::new(PayloadUtils::build_error_payload(500u16,e.to_string())))
            },
        }
    }
}

pub struct BiRequestStreamServerImpl{
    bistream_manage_addr:Addr<BiStreamManage>,
}

impl BiRequestStreamServerImpl {
    pub fn new(bistream_manage_addr:Addr<BiStreamManage>) -> Self {
        Self { bistream_manage_addr }
    }
}

#[tonic::async_trait]
impl BiRequestStream for BiRequestStreamServerImpl {
    type requestBiStreamStream=tokio_stream::wrappers::ReceiverStream<Result<Payload,tonic::Status>>;

    async fn request_bi_stream(
            &self,
            request: tonic::Request<tonic::Streaming<Payload>>,
        ) -> Result<tonic::Response<Self::requestBiStreamStream>, tonic::Status> {
        let client_id = Arc::new(request.remote_addr().unwrap().to_string());
        let req = request.into_inner();
        let (tx,rx) = tokio::sync::mpsc::channel(10);
        let r_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        let conn = BiStreamConn::new(tx, client_id.clone(), req, self.bistream_manage_addr.clone());
        self.bistream_manage_addr.do_send(BiStreamManageCmd::AddConn(client_id, conn));
        Ok(tonic::Response::new(r_stream))
    }
}