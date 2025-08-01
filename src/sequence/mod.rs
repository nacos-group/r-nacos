pub mod core;
pub mod model;

use crate::raft::cluster::route::RaftRequestRoute;
use crate::raft::store::{ClientRequest, ClientResponse};
use crate::sequence::model::{SeqGroup, SeqRange, SequenceRaftReq, SequenceRaftResult};
use actix::prelude::*;
use bean_factory::{bean, BeanFactory, FactoryData, Inject};
use serde::{Deserialize, Serialize};
/// 获取顺序递增的id功能
use std::collections::HashMap;
use std::sync::Arc;

///
/// 序号管理器
#[derive(Clone)]
#[bean(inject)]
pub struct SequenceManager {
    pub(crate) seq_map: HashMap<Arc<String>, SeqGroup>,
    raft_router: Option<Arc<RaftRequestRoute>>,
    seq_step: u64,
}

impl SequenceManager {
    pub fn new() -> Self {
        SequenceManager {
            seq_map: HashMap::new(),
            raft_router: None,
            seq_step: 100,
        }
    }

    fn do_next_id(&mut self, key: &Arc<String>) -> (Option<u64>, bool) {
        if let Some(group) = self.seq_map.get_mut(key) {
            //log::info!("SequenceManager|group info:{:?}", group);
            let v = group.next_id();
            let need_apply = group.need_apply();
            (v, need_apply)
        } else {
            let seq_group = SeqGroup::new(self.seq_step);
            self.seq_map.insert(key.clone(), seq_group);
            (None, true)
        }
    }

    async fn async_handle(
        middle_state: SequenceMiddleState,
        raft_router: Option<Arc<RaftRequestRoute>>,
        step: u64,
    ) -> anyhow::Result<SequenceBeforeResult> {
        match middle_state {
            SequenceMiddleState::NextId(key, next_id, need_apply) => {
                if let Some(v) = next_id {
                    return Ok(SequenceBeforeResult::NextId(key, v, need_apply));
                }
                if let Some(raft_router) = raft_router {
                    let (start, len) =
                        Self::get_next_range(&raft_router, key.clone(), step).await?;
                    //let mut simple_sequence = SimpleSequence::new(start, len);
                    //let v = simple_sequence.next_id();
                    Ok(SequenceBeforeResult::UseFromRange { key, start, len })
                } else {
                    Err(anyhow::anyhow!("SequenceManager|raft_router is none"))
                }
            }
            SequenceMiddleState::FillRange(key, step) => {
                if let Some(raft_router) = raft_router {
                    let (start, len) =
                        Self::get_next_range(&raft_router, key.clone(), step).await?;
                    //let mut simple_sequence = SimpleSequence::new(start, len);
                    //let v = simple_sequence.next_id();
                    Ok(SequenceBeforeResult::FillRange { key, start, len })
                } else {
                    Err(anyhow::anyhow!("SequenceManager|raft_router is none"))
                }
            }
            SequenceMiddleState::GetDirectRange(key, step) => {
                if let Some(raft_router) = raft_router {
                    let (start, len) = Self::get_next_range(&raft_router, key, step).await?;
                    Ok(SequenceBeforeResult::DirectRange { start, len })
                } else {
                    Err(anyhow::anyhow!("SequenceManager|raft_router is none"))
                }
            }
            SequenceMiddleState::FillIgnore => Ok(SequenceBeforeResult::FillIgnore),
        }
    }

    async fn get_next_range(
        raft_router: &Arc<RaftRequestRoute>,
        key: Arc<String>,
        step: u64,
    ) -> anyhow::Result<(u64, u64)> {
        let raft_req = SequenceRaftReq::NextRange(key, step);
        let client_resp = raft_router
            .request(ClientRequest::SequenceReq { req: raft_req })
            .await?;
        if let ClientResponse::SequenceResp { resp } = client_resp {
            if let SequenceRaftResult::NextRange { start, len } = resp {
                return Ok((start, len));
            }
        }
        Err(anyhow::anyhow!(
            "SequenceManager|raft request SequenceRaftReq::NextRange error"
        ))
    }

    fn handle_result(
        &mut self,
        before_result: anyhow::Result<SequenceBeforeResult>,
        ctx: &mut Context<Self>,
    ) -> anyhow::Result<SequenceResult> {
        match before_result? {
            SequenceBeforeResult::NextId(key, id, need_apply) => {
                if need_apply {
                    // 异步填充
                    ctx.address().do_send(SequenceRequest::FillRange(key));
                }
                Ok(SequenceResult::NextId(id))
            }
            SequenceBeforeResult::UseFromRange { key, start, len } => {
                if let Some(v) = self.seq_map.get_mut(&key) {
                    v.apply_range(start, len);
                }
                if let (Some(id), _) = self.do_next_id(&key) {
                    Ok(SequenceResult::NextId(id))
                } else {
                    Ok(SequenceResult::None)
                }
            }
            SequenceBeforeResult::FillRange { key, start, len } => {
                if let Some(v) = self.seq_map.get_mut(&key) {
                    v.apply_range(start, len);
                    v.clear_apply_mark();
                }
                Ok(SequenceResult::None)
            }
            SequenceBeforeResult::FillIgnore => Ok(SequenceResult::None),
            SequenceBeforeResult::DirectRange { start, len } => {
                Ok(SequenceResult::Range(SeqRange::new(start, len)))
            }
        }
    }
}

impl Actor for SequenceManager {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("SequenceManager start")
    }
}

impl Inject for SequenceManager {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        factory_data: FactoryData,
        _factory: BeanFactory,
        _ctx: &mut Self::Context,
    ) {
        self.raft_router = factory_data.get_bean();
    }
}

#[derive(Message, Clone, Debug, Serialize, Deserialize)]
#[rtype(result = "anyhow::Result<SequenceResult>")]
pub enum SequenceRequest {
    GetNextId(Arc<String>),
    FillRange(Arc<String>),
    GetDirectRange(Arc<String>, u64),
}

pub enum SequenceResult {
    NextId(u64),
    Range(SeqRange),
    None,
}

/// 序列请求中间状态
/// 用于判断是否需要异步
pub enum SequenceMiddleState {
    NextId(Arc<String>, Option<u64>, bool),
    FillRange(Arc<String>, u64),
    GetDirectRange(Arc<String>, u64),
    FillIgnore,
}
pub enum SequenceBeforeResult {
    NextId(Arc<String>, u64, bool),
    UseFromRange {
        key: Arc<String>,
        start: u64,
        len: u64,
    },
    FillRange {
        key: Arc<String>,
        start: u64,
        len: u64,
    },
    DirectRange {
        start: u64,
        len: u64,
    },
    FillIgnore,
}

impl Handler<SequenceRequest> for SequenceManager {
    type Result = ResponseActFuture<Self, anyhow::Result<SequenceResult>>;

    fn handle(&mut self, msg: SequenceRequest, _ctx: &mut Self::Context) -> Self::Result {
        let state = match msg {
            SequenceRequest::GetNextId(key) => {
                let (id, need_apply) = self.do_next_id(&key);
                SequenceMiddleState::NextId(key, id, need_apply)
            }
            SequenceRequest::FillRange(key) => {
                if let Some(v) = self.seq_map.get_mut(&key) {
                    if v.need_apply() {
                        v.mark_apply();
                        SequenceMiddleState::FillRange(key, self.seq_step)
                    } else {
                        SequenceMiddleState::FillIgnore
                    }
                } else {
                    SequenceMiddleState::FillIgnore
                }
            }
            SequenceRequest::GetDirectRange(key, len) => {
                SequenceMiddleState::GetDirectRange(key, len)
            }
        };
        let raft_router = self.raft_router.clone();
        let step = self.seq_step;
        let fut = Self::async_handle(state, raft_router, step)
            .into_actor(self)
            .map(|r, act, ctx| act.handle_result(r, ctx));
        Box::pin(fut)
    }
}
