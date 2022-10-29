use std::cell::Cell;
use std::sync::Arc;

use actix::prelude::*;
use tokio_stream::StreamExt;

use super::PayloadUtils;
use super::bistream_manage::{BiStreamManage, BiStreamManageCmd};
use super::{api_model::ClientDetectionRequest, nacos_proto::Payload};

type SenderType = tokio::sync::mpsc::Sender<Result<Payload, tonic::Status>>;
type ReceiverStreamType = tokio_stream::wrappers::ReceiverStream<Result<Payload, tonic::Status>>;

pub struct BiStreamConn {
    sender: SenderType,
    client_id:Arc<String>,
    receiver_stream: Cell<Option<ReceiverStreamType>>,
    manage : Addr<BiStreamManage>,
}

impl BiStreamConn {
    pub fn new(sender: SenderType,client_id:Arc<String> , receiver_stream: ReceiverStreamType,manage : Addr<BiStreamManage>) -> Self {
        Self {
            sender,
            client_id,
            receiver_stream: Cell::new(Some(receiver_stream)),
            manage,
        }
    }

    pub fn receive(&mut self, ctx: &mut Context<Self>) {
        if let Some(mut receiver_stream) = self.receiver_stream.replace(None) {
            let manage = self.manage.clone();
            let client_id = self.client_id.clone();
            async move { 
                while let Some(item) = receiver_stream.next().await {
                    if let Ok(payload) = item {
                        manage.do_send(BiStreamManageCmd::Response(client_id.clone(), payload));
                    }
                    else{
                        break;
                    }
                } 
                manage.do_send(BiStreamManageCmd::ConnClose(client_id));
            }
            .into_actor(self)
            .map(|_, _, ctx| {
                ctx.stop();
            })
            .spawn(ctx);
        }
    }
}

impl Actor for BiStreamConn {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {}
}

#[derive(Debug, Message)]
#[rtype(result = "Result<BiStreamSenderResult,std::io::Error>")]
pub enum BiStreamSenderCmd {
    Detection(String),
    Close,
}

pub enum BiStreamSenderResult {
    None,
}

impl Handler<BiStreamSenderCmd> for BiStreamConn {
    type Result = Result<BiStreamSenderResult, std::io::Error>;

    fn handle(&mut self, msg: BiStreamSenderCmd, ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            BiStreamSenderCmd::Detection(request_id) => {
                let mut request = ClientDetectionRequest::default();
                request.module = Some("internal".to_owned());
                request.request_id = Some(request_id);
                let payload = PayloadUtils::build_payload(
                    "ClientDetectionRequest",
                    serde_json::to_string(&request).unwrap(),
                );
                let sender = self.sender.clone();
                async move {
                    sender.send(Ok(payload)).await.unwrap();
                }
                .into_actor(self)
                .map(|_, _, _| {})
                .spawn(ctx);
            }
            BiStreamSenderCmd::Close=> {
                ctx.stop();
            },
        }
        Ok(BiStreamSenderResult::None)
    }
}
