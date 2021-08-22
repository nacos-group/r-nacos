
use std::io::stdin;
use std::error::Error;
use std::env;
use std::net::SocketAddr;
use std::sync::{Arc};
use std::borrow::Cow;
use actix::prelude::*;
use tokio::net::{UdpSocket,udp::{RecvHalf,SendHalf}};
use tokio::signal;
use tokio::sync::Mutex;

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
        let mut s=send.lock().await;
        s.send_to(data,&addr).await.unwrap();
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
    fn new(data:Vec<u8>,addr:SocketAddr) -> Self {
        Self{
            data,
            target_addr:addr,
        }
    }
}

impl Handler<UdpSenderCmd> for UdpSender {
    type Result = ResponseActFuture<Self,Result<(),std::io::Error>>;

    fn handle(&mut self,msg:UdpSenderCmd,ctx: &mut Context<Self>) -> Self::Result {
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

pub struct Notify;

impl Actor for Notify {
    type Context = Context<Self>;
}

#[derive(Debug,Message)]
#[rtype(result = "Result<(),std::io::Error>")]
pub struct NotifyMsg(pub Vec<u8>);

impl Handler<NotifyMsg> for Notify {
    type Result = Result<(),std::io::Error>;

    fn handle(&mut self,msg:NotifyMsg,ctx: &mut Context<Self>) -> Self::Result {
        Ok(())
    }
}

pub struct UdpReciver{
    recv:RecvHalf,
    buf:Vec<u8>,
    recv_call:Option<Box<dyn Fn(Vec<u8>) +Send+Sync>>,
    //actor: Option<Addr<T>>,
}

const MAX_DATAGRAM_SIZE: usize = 65_507;

impl UdpReciver {
    pub fn new(recv:RecvHalf,recv_call:Option<Box<dyn Fn(Vec<u8>)+Send+Sync>>) -> Self{
        Self{
            recv:recv,
            buf:vec![0u8;MAX_DATAGRAM_SIZE],
            recv_call,
            //actor:None,
        }
    }

    pub async fn recv(&mut self) {
        let a = async{
        };
        loop{
            match self.recv.recv_from(&mut self.buf).await{
                Ok((len,addr)) => {
                    /*
                    if let Some(addr) = self.actor{
                        let mut data:Vec<u8> = Vec::with_capacity(len);
                        data.clone_from_slice(&self.buf[..len]);
                        addr.send(data).await;
                    }
                    */
                    let s=String::from_utf8_lossy(&self.buf[..len]);
                    println!("rece from:{} | len:{} | str:{}",&addr,len,s);
                },
                _ => {}
            }
        }
    }
}

fn get_stdin_data() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut s = String::new();
    stdin().read_line(&mut s)?;
    Ok(s.into_bytes())
}

fn send(sender:UdpSender,remote_addr:SocketAddr){
    let mut sys = actix::System::new("udp_send").block_on(async move {

    let sender_addr = sender.start();
    loop{
        let data = get_stdin_data().unwrap();
        let msg = UdpSenderCmd::new(data,remote_addr.clone());
        sender_addr.send(msg).await;
    }
    });
}

//#[tokio::main]
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("notify udp");
    let remote_addr: SocketAddr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".into())
        .parse()?;
    println!("addr:{:?}", &remote_addr);

    // We use port 0 to let the operating system allocate an available port for us.
    let local_addr: SocketAddr = env::args()
        .nth(2)
        .unwrap_or_else(|| "0.0.0.0:0".into())
        .parse()?;
    println!("local_addr:{:?}", &local_addr);

    let socket = UdpSocket::bind(local_addr).await?;
    //socket.connect(remote_addr.clone()).await?;
    let (r,w) = socket.split();
    tokio::spawn(async move {
        println!("loop_rec start");
        UdpReciver::new(r,None).recv().await;
        println!("loop_rec end");
    });

    let sender = UdpSender::new(w);
    std::thread::spawn(move || send(sender,remote_addr));
    let ctrl_c = signal::ctrl_c().await;

    Ok(())
}