use std::cell::Cell;
use std::collections::HashMap;
use std::sync::Arc;

use actix::prelude::*;
use tokio_stream::StreamExt;

use super::api_model::ConnectResetRequest;
use super::bistream_manage::{BiStreamManage, BiStreamManageCmd};
use super::PayloadUtils;
use super::{api_model::ClientDetectionRequest, nacos_proto::Payload};

type SenderType = tokio::sync::mpsc::Sender<Result<Payload, tonic::Status>>;
type ReceiverStreamType = tonic::Streaming<Payload>;

#[derive(Debug)]
pub enum NamespaceType {
    Empty,
    Public,
    Other(Arc<String>),
    Unknown,
}

impl NamespaceType {
    pub fn from_option(namespace: Option<String>) -> Self {
        if let Some(namespace) = namespace {
            NamespaceType::get_namespace_type(&namespace)
        } else {
            NamespaceType::Unknown
        }
    }
    pub fn get_namespace_type(namespace: &str) -> Self {
        if namespace.is_empty() {
            NamespaceType::Empty
        } else if namespace == "public" {
            NamespaceType::Public
        } else {
            NamespaceType::Other(Arc::new(namespace.to_owned()))
        }
    }

    pub fn is_default(&self) -> bool {
        match self {
            NamespaceType::Empty => true,
            NamespaceType::Public => true,
            _ => false,
        }
    }

    pub fn to_str(&self) -> &str {
        match self {
            NamespaceType::Empty => "",
            NamespaceType::Public => "public",
            NamespaceType::Other(namespace) => namespace.as_str(),
            NamespaceType::Unknown => "",
        }
    }
}

pub struct BiStreamConn {
    sender: SenderType,
    client_id: Arc<String>,
    receiver_stream: Cell<Option<ReceiverStreamType>>,
    manage: Addr<BiStreamManage>,
}

impl BiStreamConn {
    pub fn new(
        sender: SenderType,
        client_id: Arc<String>,
        receiver_stream: ReceiverStreamType,
        manage: Addr<BiStreamManage>,
    ) -> Self {
        Self {
            sender,
            client_id,
            receiver_stream: Cell::new(Some(receiver_stream)),
            manage,
        }
    }

    pub fn receive(&mut self, ctx: &mut Context<Self>) {
        //println!("BiStreamConn start receive");
        if let Some(mut receiver_stream) = self.receiver_stream.replace(None) {
            let manage = self.manage.clone();
            let client_id = self.client_id.clone();
            async move {
                //if let Some(Ok(_payload)) = receiver_stream.next().await {
                //println!("BiStreamConn receive frist msg:{}",PayloadUtils::get_payload_string(&payload));
                //}
                while let Some(Ok(payload)) = receiver_stream.next().await {
                    //println!("BiStreamConn receive msg:{}",PayloadUtils::get_payload_string(&payload));
                    manage.do_send(BiStreamManageCmd::Response(client_id.clone(), payload));
                }
                manage.do_send(BiStreamManageCmd::ConnClose(client_id));
            }
            .into_actor(self)
            .map(|_, _, ctx| {
                //debug
                //println!("stop at receive!");
                ctx.stop();
            })
            .spawn(ctx);
        }
    }

    fn send_payload(&mut self, ctx: &mut Context<Self>, payload: Payload) {
        let sender = self.sender.clone();
        //debug
        //log::info!("send_payload {}",PayloadUtils::get_payload_string(&payload));
        async move { sender.send(Ok(payload)).await }
            .into_actor(self)
            .map(|_, _, _| {})
            .spawn(ctx);
    }

    fn close_stream_and_stop(&mut self, ctx: &mut Context<Self>) {
        let sender = self.sender.clone();
        async move {
            //debug
            //println!("close_stream_and_stop! 01");
            sender.send(Err(tonic::Status::cancelled("close"))).await
        }
        .into_actor(self)
        .map(|_, _, ctx| {
            //debug
            //println!("stop at close_stream_and_stop!");
            ctx.stop();
        })
        .wait(ctx);
    }
}

impl Actor for BiStreamConn {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        //log::info!("BiStreamConn started");
        self.receive(ctx);
    }
}

/*
impl Supervised for BiStreamConn {
    fn restarting(&mut self, _ctx: &mut <Self as Actor>::Context) {
        log::warn!("BiStreamConn restart ...");
    }
}
*/

#[derive(Debug, Message)]
#[rtype(result = "Result<BiStreamSenderResult,std::io::Error>")]
pub enum BiStreamSenderCmd {
    Detection(String),
    Reset(String, Option<String>, Option<String>),
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
                let request = ClientDetectionRequest {
                    module: Some("internal".to_owned()),
                    request_id: Some(request_id),
                    headers: Some(HashMap::new()),
                };
                let payload = PayloadUtils::build_payload(
                    "ClientDetectionRequest",
                    serde_json::to_string(&request).unwrap(),
                );
                self.send_payload(ctx, payload);
            }
            BiStreamSenderCmd::Reset(request_id, ip, port) => {
                let request = ConnectResetRequest {
                    module: Some("internal".to_owned()),
                    request_id: Some(request_id),
                    server_ip: ip,
                    server_port: port,
                    headers: Some(HashMap::new()),
                };
                let payload = PayloadUtils::build_payload(
                    "ConnectResetRequest",
                    serde_json::to_string(&request).unwrap(),
                );
                self.send_payload(ctx, payload);
            }
            BiStreamSenderCmd::Send(payload) => {
                self.send_payload(ctx, payload.as_ref().to_owned());
            }
            BiStreamSenderCmd::Close => {
                self.close_stream_and_stop(ctx);
            }
        }
        Ok(BiStreamSenderResult::None)
    }
}
