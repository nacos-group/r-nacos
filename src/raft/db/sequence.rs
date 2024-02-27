use std::collections::HashMap;
use std::sync::Arc;
use actix::prelude::*;
use serde::{Deserialize, Serialize};

pub struct SequenceManager{
    pub(crate) seq_info: HashMap<Arc<String>,u64>,
}

impl SequenceManager {

    fn get_last_id(&self,key:&Arc<String>) -> u64 {
        if let Some(v) = self.seq_info.get(key) {
            *v
        }
        else{
            0
        }
    }

    fn next_id(&mut self,key:Arc<String>,step:u64) -> u64 {
        let last_id = self.get_last_id(&key);
        let next_id = last_id + step;
        self.seq_info.insert(key,next_id);
        next_id
    }
}

impl Actor for SequenceManager {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("SequenceManager start")
    }
}

#[derive(Message, Clone, Debug, Serialize, Deserialize)]
#[rtype(result = "anyhow::Result<SequenceResult>")]
pub enum SequenceRequest{
    GetLastId(Arc<String>),
    GetNextId(Arc<String>,u64),
    SetLastId(Arc<String>,u64),
}

pub enum SequenceResult {
    LastId(u64),
    NextId(u64),
    None
}

impl Handler<SequenceRequest> for SequenceManager {
    type Result = anyhow::Result<SequenceResult>;

    fn handle(&mut self, msg: SequenceRequest, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            SequenceRequest::GetLastId(key) => {
                let v = self.get_last_id(&key);
                Ok(SequenceResult::LastId(v))
            }
            SequenceRequest::GetNextId(key, step) => {
                let v = self.next_id(key,step);
                Ok(SequenceResult::NextId(v))
            }
            SequenceRequest::SetLastId(key,last_id) => {
                self.seq_info.insert(key,last_id);
                Ok(SequenceResult::None)
            }
        }
    }
}

pub struct SequenceItem {
    cache_size: u64,
    batch_size: u64,
    last_id: u64,
    pub(crate) seq_addr: Addr<SequenceManager>,
    pub(crate) seq_key: Arc<String>,
    init: bool,
}

impl SequenceItem {
    pub fn new(seq_addr: Addr<SequenceManager>,seq_key: Arc<String>,batch_size:u64) -> Self {
        Self {
            seq_addr,
            seq_key,
            batch_size,
            cache_size:0,
            last_id:0,
            init: false,
        }
    }

    pub async fn init(&mut self) -> anyhow::Result<()> {
        let res = self.seq_addr.send(SequenceRequest::GetLastId(self.seq_key.clone())).await??;
        match res {
            SequenceResult::LastId(v) => {
                self.last_id=v;
                self.init=true;
            }
            _ => {}
        };
        Ok(())
    }

    pub fn set_table_last_id(&mut self, id: u64) -> anyhow::Result<()> {
        if (self.last_id + self.batch_size) < id {
            self.last_id = id;
            self.batch_size=0;
            self.init=true;
            self.seq_addr.do_send(SequenceRequest::SetLastId(self.seq_key.clone(),id));
        }
        Ok(())
    }

    pub fn next_id(&mut self) -> anyhow::Result<u64> {
        if !self.init {
            return Err(anyhow::anyhow!("The SequenceItem is not init,{}",&self.seq_key))
        }
        if self.cache_size == 0 {
            let cache_last_id = self.last_id + self.batch_size;
            self.seq_addr.do_send(SequenceRequest::SetLastId(self.seq_key.clone(),cache_last_id));
            self.cache_size = self.batch_size;
        }
        self.cache_size -= 1;
        self.last_id += 1;
        Ok(self.last_id)
    }
}
