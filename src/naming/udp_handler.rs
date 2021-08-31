
use std::error::Error;
use std::net::SocketAddr;
use std::sync::{Arc};
use actix::prelude::*;
use tokio::net::{UdpSocket,udp::{RecvHalf,SendHalf}};
use tokio::sync::Mutex;

use super::core::{NamingActor,NamingCmd};

#[derive(Debug)]
pub struct UdpSender{
    send:Arc<Mutex<SendHalf>>,
}

impl UdpSender{
    pub fn new(send:SendHalf) -> Self{
        Self{
            send:Arc::new(Mutex::new(send)),
        }
    }

    async fn send(send:Arc<Mutex<SendHalf>>,addr:SocketAddr,data:&[u8]) {
        println!("UdpSender send:{:?},{}",&addr,data.len());
        let mut s=send.lock().await;
        s.send_to(data,&addr).await.unwrap();
        println!("UdpSender send end");
    }
}

impl Actor for UdpSender{
    type Context = Context<Self>;
}

#[derive(Debug,Message)]
#[rtype(result = "Result<(),std::io::Error>")]
pub struct UdpSenderCmd{
    pub data:Vec<u8>,
    pub target_addr:SocketAddr,
}

impl UdpSenderCmd{
    pub fn new(data:Vec<u8>,addr:SocketAddr) -> Self {
        Self{
            data,
            target_addr:addr,
        }
    }
}

impl Handler<UdpSenderCmd> for UdpSender {
    type Result = ResponseActFuture<Self,Result<(),std::io::Error>>;

    fn handle(&mut self,msg:UdpSenderCmd,ctx: &mut Context<Self>) -> Self::Result {
        println!("UdpSender msg:{:?},{}",&msg.target_addr,&msg.data.len());
        let send = self.send.clone();
        
        Box::pin(
            async move{
                Self::send(send, msg.target_addr,&msg.data.as_slice()).await;
                Ok(())
            }.into_actor(self).map(|r,b,c| 
                r
            )
        )
    }
}


#[derive(Debug)]
pub struct UdpReciver{
    recv:RecvHalf,
    buf:Vec<u8>,
    naming_addr:Addr<NamingActor>,
}

const MAX_DATAGRAM_SIZE: usize = 65_507;

impl UdpReciver {
    pub fn new(recv:RecvHalf,naming_addr:Addr<NamingActor>) -> Self{
        Self{
            recv:recv,
            buf:vec![0u8;MAX_DATAGRAM_SIZE],
            naming_addr,
        }
    }

    pub async fn recv(&mut self) {
        println!("UdpReciver start");
        loop{
            match self.recv.recv_from(&mut self.buf).await{
                Ok((len,addr)) => {
                    println!("UdpReciver addr:{}",&addr);
                    let mut data:Vec<u8> = vec![0u8;len];
                    data.clone_from_slice(&self.buf[..len]);
                    println!("UdpReciver data len:{}",&data.len());
                    let msg = NamingCmd::ClientReceiveMsg((addr,data));
                    self.naming_addr.do_send(msg);
                },
                _ => {}
            }
        }
    }
}
