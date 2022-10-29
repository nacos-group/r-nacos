use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::now_millis;

use super::{bistream_conn::{BiStreamConn, BiStreamSenderCmd}, PayloadUtils, nacos_proto::Payload};
use actix::prelude::*;
use inner_mem_cache::TimeoutSet;


struct ConnCacheItem {
    last_active_time:u64,
    conn:Addr<BiStreamConn>,
}

impl ConnCacheItem {
    fn new(last_active_time:u64,conn:Addr<BiStreamConn>) -> Self {
        Self { last_active_time, conn: conn }
    }
}

#[derive(Default)]
pub struct BiStreamManage{
    conn_cache:HashMap<Arc<String>,ConnCacheItem>,
    active_time_set: TimeoutSet<Arc<String>>,
    response_time_set : TimeoutSet<Arc<String>>,
    detection_time_out: u64,
    response_time_out: u64,
    request_id:u64,
}

impl BiStreamManage {

    pub fn new() -> Self {
        let mut this= BiStreamManage::default();
        this.detection_time_out = 5000;
        this.response_time_out = 5000;
        this
    }

    pub fn add_conn(&mut self,client_id:Arc<String>,sender:Addr<BiStreamConn>) {
        let now = now_millis();
        let item = ConnCacheItem::new(now,sender);
        self.conn_cache.insert(client_id.clone(), item);
        self.active_time_set.add(now+self.detection_time_out, client_id);
    }

    fn check_active_time_set(&mut self,now:u64){
        let keys=self.active_time_set.timeout(now);
        for key in keys{
            if let Some(item)=self.conn_cache.get(&key) {
                item.conn.do_send(BiStreamSenderCmd::Detection(self.request_id.to_string()));
                self.request_id+=1;
                self.response_time_set.add(now+self.response_time_out, key);
            }
        }
    }

    fn check_response_time_set(&mut self,now:u64){
        let keys=self.response_time_set.timeout(now);
        let mut del_keys = vec![];
        for key in keys {
            if let Some(item)=self.conn_cache.get(&key) {
                if item.last_active_time + self.detection_time_out + self.response_time_out <= now {
                    del_keys.push(key);
                }
            }
        }
        for key in del_keys {
            if let Some(item) = self.conn_cache.remove(&key){
                item.conn.do_send(BiStreamSenderCmd::Close);
            }
        }
    }


    pub fn time_out_heartbeat(&self,ctx:&mut actix::Context<Self>) {
        ctx.run_later(Duration::new(1,0), |act,ctx|{
            let now = now_millis();
            act.check_response_time_set(now);
            act.check_active_time_set(now);
            act.time_out_heartbeat(ctx);
        });
    }

}

impl Actor for BiStreamManage {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.time_out_heartbeat(ctx);
        log::info!("BiStreamManage started");
    }
}

#[derive(Debug, Message)]
#[rtype(result = "Result<BiStreamManageResult,std::io::Error>")]
pub enum BiStreamManageCmd {
    Response(Arc<String>,Payload),
    ConnClose(Arc<String>),
    AddConn(Arc<String>,Addr<BiStreamConn>),
}

pub enum BiStreamManageResult {
    None,
}

impl Handler<BiStreamManageCmd> for BiStreamManage {
    type Result = Result<BiStreamManageResult, std::io::Error>;

    fn handle(&mut self, msg: BiStreamManageCmd, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            BiStreamManageCmd::Response(client_id,payload)=> {
                if let Some(t)=PayloadUtils::get_payload_type(&payload) {
                    if "ClientDetectionResponse"== t {
                        if let Some(item)=self.conn_cache.get_mut(&client_id) {
                            item.last_active_time = now_millis();
                        }
                    }
                }
            },
            BiStreamManageCmd::ConnClose(client_id) => {
                self.conn_cache.remove(&client_id);
            },
            BiStreamManageCmd::AddConn(client_id, conn) => {
                self.add_conn(client_id, conn);
            },
        }
        Ok(BiStreamManageResult::None)
    }
}


