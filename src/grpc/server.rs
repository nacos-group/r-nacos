use tokio_stream::StreamExt;

use crate::grpc::{PayloadUtils, PayloadHandler};
use crate::grpc::nacos_proto::{request_server, Payload};

use super::handler::InvokerHandler;
use super::nacos_proto::bi_request_stream_server::BiRequestStream;


#[derive(Default)]
pub struct RequestServerImpl{
    invoker:InvokerHandler,
}

#[tonic::async_trait]
impl request_server::Request for RequestServerImpl {
    async fn request(
        &self,
        request: tonic::Request<Payload>,
    ) -> Result<tonic::Response<Payload>,tonic::Status> {
        let v2 = request.into_inner();
        println!("request_server request:{}",PayloadUtils::get_payload_string(&v2));
        let res = self.invoker.handle(v2);
        Ok(tonic::Response::new(res))
    }
}

#[derive(Debug,Default)]
pub struct BiRequestStreamServerImpl;

#[tonic::async_trait]
impl BiRequestStream for BiRequestStreamServerImpl {
    type requestBiStreamStream=tokio_stream::wrappers::ReceiverStream<Result<Payload,tonic::Status>>;

    async fn request_bi_stream(
            &self,
            request: tonic::Request<tonic::Streaming<Payload>>,
        ) -> Result<tonic::Response<Self::requestBiStreamStream>, tonic::Status> {
        let mut req = request.into_inner();
        let (tx,rx) = tokio::sync::mpsc::channel(10);
        let r_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        tokio::spawn(async move {
            while let Some(v) = req.next().await {
                if let Ok(payload)=&v {
                    println!("request_bi_stream request:{}",PayloadUtils::get_payload_string(payload));
                }
                tx.send(v).await.unwrap();
            }
            tx.send(Err(tonic::Status::aborted("close"))).await.unwrap();
        });
        Ok(tonic::Response::new(r_stream))
    }
}