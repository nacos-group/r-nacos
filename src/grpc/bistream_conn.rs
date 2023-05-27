use std::cell::Cell;
use std::sync::Arc;

use actix::prelude::*;
use tokio_stream::StreamExt;

use super::PayloadUtils;
use super::bistream_manage::{BiStreamManage, BiStreamManageCmd};
use super::{api_model::ClientDetectionRequest, nacos_proto::Payload};

type SenderType = tokio::sync::mpsc::Sender<Result<Payload, tonic::Status>>;
type ReceiverStreamType = tonic::Streaming<Payload>;

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
                if let Some(item) = receiver_stream.next().await {
                    if let Ok(payload) = item {
                        println!("BiStreamConn receive frist msg:{}",PayloadUtils::get_payload_string(&payload));
                    }
                }
                while let Some(item) = receiver_stream.next().await {
                    if let Ok(payload) = item {
                        println!("BiStreamConn receive msg:{}",PayloadUtils::get_payload_string(&payload));
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

    fn send_payload(&mut self, ctx: &mut Context<Self>,payload:Payload){
        let sender = self.sender.clone();
        async move {
            sender.send(Ok(payload)).await
        }
        .into_actor(self)
        .map(|_, _, _| {})
        .spawn(ctx);
    }

}

impl Actor for BiStreamConn {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        //log::info!("BiStreamConn started");
    }
}


impl Supervised for BiStreamConn {
    fn restarting(&mut self, _ctx: &mut <Self as Actor>::Context) {
        log::warn!("BiStreamConn restart ...");
    }
}

#[derive(Debug, Message)]
#[rtype(result = "Result<BiStreamSenderResult,std::io::Error>")]
pub enum BiStreamSenderCmd {
    Detection(String),
    Send(Arc<Payload>),
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
                self.send_payload(ctx,payload);
            },
            BiStreamSenderCmd::Send(payload) => {
                self.send_payload(ctx,payload.as_ref().to_owned());
            },
            BiStreamSenderCmd::Close=> {
                ctx.stop();
            },
        }
        Ok(BiStreamSenderResult::None)
    }
}
