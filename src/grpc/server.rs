use std::sync::Arc;

use actix::prelude::*;
//use tokio_stream::StreamExt;

use crate::grpc::{PayloadUtils, PayloadHandler};
use crate::grpc::nacos_proto::{request_server, Payload};

use super::bistream_conn::BiStreamConn;
use super::bistream_manage::{BiStreamManage, BiStreamManageCmd};
use super::handler::InvokerHandler;
use super::nacos_proto::bi_request_stream_server::BiRequestStream;


pub struct RequestServerImpl{
    invoker:InvokerHandler,
}

impl RequestServerImpl {
    pub fn new() -> Self {
        Self { invoker:InvokerHandler::new() }
    }
}

#[tonic::async_trait]
impl request_server::Request for RequestServerImpl {
    async fn request(
        &self,
        request: tonic::Request<Payload>,
    ) -> Result<tonic::Response<Payload>,tonic::Status> {
        let remote_addr = request.remote_addr().unwrap();
        let connection_id = remote_addr.to_string();
        let mut payload = request.into_inner();
        println!("request_server request:{}",PayloadUtils::get_payload_string(&payload));
        if let Some(meta) = payload.metadata.as_mut() {
            meta.headers.insert("connection_id".to_owned(), connection_id);
        }
        let res = self.invoker.handle(payload);
        Ok(tonic::Response::new(res))
    }
}

pub struct BiRequestStreamServerImpl{
    stream_manage:Addr<BiStreamManage>,
}

impl BiRequestStreamServerImpl {
    pub fn new(stream_manage:Addr<BiStreamManage>) -> Self {
        Self { stream_manage }
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
        let conn = BiStreamConn::new(tx, client_id.clone(), req, self.stream_manage.clone());
        self.stream_manage.do_send(BiStreamManageCmd::AddConn(client_id, conn));
        Ok(tonic::Response::new(r_stream))
    }
}