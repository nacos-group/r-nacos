use std::sync::Arc;

use actix::prelude::*;
use async_raft_ext as async_raft;
use async_raft::raft::{EntryPayload};
use bean_factory::{bean, Inject};

use super::{
    model::{LogRecordLoader, MemberShip, SnapshotHeaderDto, ApplyRequestDto},
    raftindex::{RaftIndexManager, RaftIndexRequest, RaftIndexResponse},
    raftlog::{RaftLogManager, RaftLogManagerAsyncRequest, RaftLogManagerRequest},
    raftsnapshot::{
        RaftSnapshotManager, RaftSnapshotRequest, RaftSnapshotResponse, SnapshotReader,
    }, StoreUtils, log::SnapshotRange,
};

pub struct LogRecordLoaderInstance {
    //data_store: Addr<RaftDataStore>,
}

impl LogRecordLoaderInstance {
}

impl LogRecordLoader for LogRecordLoaderInstance {
    fn load(&self, record: super::model::LogRecordDto) -> anyhow::Result<()> {
        let entry = StoreUtils::log_record_to_entry(record)?;
        match entry.payload {
            EntryPayload::Normal(req) => {
                match req.data {
                    _ => {},
                }
            },
            _ => {}
        }
        Ok(())
    }
}

#[bean(inject)]
pub struct StateApplyManager {
    index_manager: Option<Addr<RaftIndexManager>>,
    snapshot_manager: Option<Addr<RaftSnapshotManager>>,
    log_manager: Option<Addr<RaftLogManager>>,
    //data_store: Option<Addr<RaftDataStore>>,
    snapshot_next_index: u64,
    last_applied_log: u64,
    last_snapshot_path: Option<Arc<String>>,
    swap_snapshot_header: Option<SnapshotHeaderDto>,
}

impl StateApplyManager {
    pub fn new() -> Self {
        Self {
            index_manager: None,
            snapshot_manager: None,
            log_manager: None,
            //data_store: None,
            snapshot_next_index: 1,
            last_applied_log: 0,
            last_snapshot_path: None,
            swap_snapshot_header: None,
        }
    }

    fn init(&mut self, ctx: &mut Context<Self>) {
        self.load_index(ctx);
        //加载历史数据

    }

    fn load_index(&mut self, ctx: &mut Context<Self>) {
        if self.index_manager.is_none() {
            return;
        }
        let index_manager = self.index_manager.clone().unwrap();
        async move {
            index_manager
                .send(super::raftindex::RaftIndexRequest::LoadIndexInfo)
                .await?
        }
        .into_actor(self)
        .map(|r, act, ctx| {
            if let Ok(RaftIndexResponse::RaftIndexInfo {
                raft_index,
                last_applied_log,
            }) = r
            {
                log::info!("load_index,{:?}",&raft_index);
                raft_index.snapshots.last().map(|e| {
                    act.snapshot_next_index = e.end_index + 1;
                    //act.last_snapshot_path = Arc::new(Self::pa)
                });
                act.last_applied_log = last_applied_log;
            }
            //加载镜像,镜像转成状态
            act.load_snapshot(ctx);
        })
        .wait(ctx);
    }

    fn load_snapshot(&mut self, ctx: &mut Context<Self>) {
        if self.snapshot_next_index == 0
            || self.snapshot_manager.is_none()
            //|| self.data_store.is_none()
        {
            self.load_log(ctx);
            return;
        }
        let snapshot_manager = self.snapshot_manager.clone().unwrap();
        //let data_store = self.data_store.clone().unwrap();
        async move {
            if let RaftSnapshotResponse::LastSnapshot(Some(path), _) = snapshot_manager
                .send(RaftSnapshotRequest::GetLastSnapshot)
                .await??
            {
                let reader = SnapshotReader::init(&path).await?;
                log::info!("load_snapshot header,{:?}",&reader.get_header());
                //Self::do_load_snapshot(reader).await?;
            }
            Ok(())
        }
        .into_actor(self)
        .map(|_r: anyhow::Result<()>, act, ctx| {
            //加载日志,日志转成状态
            act.load_log(ctx);
        })
        .wait(ctx);
    }

    fn apply_snapshot(&mut self, ctx: &mut Context<Self>, file: Box<tokio::fs::File>) {
        /*
        if self.data_store.is_none() && self.index_manager.is_none() {
            return;
        }
        let data_store = self.data_store.clone().unwrap();
         */
        let index_manager = self.index_manager.clone().unwrap();
        async move {
            let reader = SnapshotReader::init_by_file(file).await?;
            let header = reader.get_header();
            let member_after_consensus = if header.member_after_consensus.is_empty() {
                None
            }
            else{
                Some(header.member_after_consensus.clone())
            };
            index_manager.do_send(RaftIndexRequest::SaveMember { member: header.member.clone(), member_after_consensus, node_addr: Some(header.node_addrs.clone()) });
            //Self::do_load_snapshot(reader).await?;

            Ok(())
        }
        .into_actor(self)
        .map(|_r: anyhow::Result<()>, _act, _ctx| {})
        .wait(ctx);
    }

    async fn do_load_snapshot(
        //data_store: Addr<RaftDataStore>,
        mut reader: SnapshotReader,
    ) -> anyhow::Result<()> {
        while let Ok(Some(record)) = reader.read_record().await {
            if record.tree.as_str() == "KV" {
                let key = Arc::new(String::from_utf8(record.key)?);
                let value = Arc::new(String::from_utf8(record.value)?);
                //data_store.do_send(super::raftstore::RaftDataStoreRequest::Set { key, value });
            }
        }
        Ok(())
    }

    fn load_log(&mut self, ctx: &mut Context<Self>) {
        if self.last_applied_log == 0 || self.log_manager.is_none() 
        //|| self.data_store.is_none() 
        {
            return;
        }
        let start_index = self.snapshot_next_index;
        let end_index = self.last_applied_log + 1;
        let log_manager = self.log_manager.clone().unwrap();
        //let data_store = self.data_store.clone().unwrap();
        let loader = Arc::new(LogRecordLoaderInstance{});
        async move {
            log_manager
                .send(RaftLogManagerRequest::Load {
                    start: start_index,
                    end: end_index,
                    loader,
                })
                .await??;
            Ok(())
        }
        .into_actor(self)
        .map(|_r: anyhow::Result<()>, _act, _ctx| {})
        .wait(ctx);
    }

    fn apply_request_to_state_machine(&mut self, request: ApplyRequestDto) -> anyhow::Result<()> {
        self.last_applied_log = request.index;
        //todo
        match request.request {
            _ => {}
        };
        if let Some(index_manager) = &self.index_manager {
            index_manager.do_send(super::raftindex::RaftIndexRequest::SaveLastAppliedLog(
                self.last_applied_log,
            ));
        }
        Ok(())
    }

    async fn do_build_snapshot(
        log_manager: Addr<RaftLogManager>,
        index_manager: Addr<RaftIndexManager>,
        snapshot_manager: Addr<RaftSnapshotManager>,
        //data_store: Addr<RaftDataStore>,
        last_index: u64,
    ) -> anyhow::Result<(SnapshotHeaderDto, Arc<String>, u64)> {
        //1. get last applied log
        let last_log = match log_manager
            .send(RaftLogManagerAsyncRequest::Query {
                start: last_index,
                end: last_index + 1,
            })
            .await??
        {
            super::raftlog::RaftLogResponse::QueryResult(mut list) => {
                list.pop().unwrap_or_default()
            }
            _ => return Err(anyhow::anyhow!("RaftLogResponse is error")),
        };
        //2. get membership
        let member_ship = match index_manager
            .send(super::raftindex::RaftIndexRequest::LoadMember)
            .await??
        {
            RaftIndexResponse::MemberShip {
                member,
                member_after_consensus,
                node_addrs
            } => MemberShip {
                member,
                member_after_consensus,
                node_addrs,
            },
            _ => return Err(anyhow::anyhow!("RaftIndexResponse is error")),
        };
        //3. build writer
        let header = SnapshotHeaderDto {
            last_index,
            last_term: last_log.term,
            member: member_ship.member,
            member_after_consensus: member_ship.member_after_consensus,
            node_addrs: member_ship.node_addrs,
        };
        let (writer, snapshot_id, path) = match snapshot_manager
            .send(RaftSnapshotRequest::NewSnapshot(header.clone()))
            .await??
        {
            RaftSnapshotResponse::NewSnapshot(writer, id, path) => (writer, id, path),
            _ => return Err(anyhow::anyhow!("RaftSnapshotResponse is error")),
        };
        //4. write data
        /*
        data_store
            .send(RaftDataStoreRequest::BuildSnapshot(writer.clone()))
            .await??;
         */

        //5. flush to file
        writer
            .send(super::raftsnapshot::SnapshotWriterRequest::Flush)
            .await??;

        let snapshot_range = SnapshotRange {
            id: snapshot_id,
            end_index: last_index,
        };
        snapshot_manager
            .send(RaftSnapshotRequest::CompleteSnapshot(snapshot_range))
            .await??;
        //log_manager.do_send(RaftLogManagerRequest::SplitOff(last_index));
        Ok((header, path, snapshot_id))
    }
}

impl Actor for StateApplyManager {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!("StateApplyManager started");
    }
}

impl Inject for StateApplyManager {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        factory_data: bean_factory::FactoryData,
        _factory: bean_factory::BeanFactory,
        ctx: &mut Self::Context,
    ) {
        self.index_manager = factory_data.get_actor();
        self.snapshot_manager = factory_data.get_actor();
        self.log_manager = factory_data.get_actor();
        //self.data_store = factory_data.get_actor();

        self.init(ctx);
    }
}

#[derive(Message, Debug)]
#[rtype(result = "anyhow::Result<StateApplyResponse>")]
pub enum StateApplyRequest {
    GetLastAppliedLog,
    //ApplyEntries(Vec<Entry<ClientRequest>>),
    ApplyRequest(ApplyRequestDto),
    ApplyBatchRequest(Vec<ApplyRequestDto>),
    ApplySnapshot { snapshot: Box<tokio::fs::File> },
}

#[derive(Message, Debug)]
#[rtype(result = "anyhow::Result<StateApplyResponse>")]
pub enum StateApplyAsyncRequest {
    BuildSnapshot,
}

pub enum StateApplyResponse {
    None,
    Snapshot(SnapshotHeaderDto, Arc<String>,u64),
    LastAppliedLog(u64),
}

impl Handler<StateApplyRequest> for StateApplyManager {
    type Result = anyhow::Result<StateApplyResponse>;

    fn handle(&mut self, msg: StateApplyRequest, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            StateApplyRequest::ApplyRequest(request) => {
                self.apply_request_to_state_machine(request)?;
                Ok(StateApplyResponse::None)
            }
            StateApplyRequest::ApplyBatchRequest(requests) => {
                for request in requests.into_iter() {
                    self.apply_request_to_state_machine(request)?;
                }
                Ok(StateApplyResponse::None)
            }
            StateApplyRequest::ApplySnapshot { snapshot } => {
                self.apply_snapshot(ctx, snapshot);
                Ok(StateApplyResponse::None)
            }
            StateApplyRequest::GetLastAppliedLog => {
                Ok(StateApplyResponse::LastAppliedLog(self.last_applied_log))
            }
        }
    }
}

impl Handler<StateApplyAsyncRequest> for StateApplyManager {
    type Result = ResponseActFuture<Self, anyhow::Result<StateApplyResponse>>;

    fn handle(&mut self, msg: StateApplyAsyncRequest, ctx: &mut Self::Context) -> Self::Result {
        let log_manager = self.log_manager.clone().unwrap();
        let index_manager = self.index_manager.clone().unwrap();
        let snapshot_manager = self.snapshot_manager.clone().unwrap();
        //let data_store = self.data_store.clone().unwrap();
        let last_index = self.last_applied_log;
        let fut = async move {
            let (header, path,snapshot_id) = match msg {
                StateApplyAsyncRequest::BuildSnapshot => {
                    Self::do_build_snapshot(
                        log_manager,
                        index_manager,
                        snapshot_manager,
                        //data_store,
                        last_index,
                    )
                    .await?
                }
            };
            Ok(StateApplyResponse::Snapshot(header, path, snapshot_id))
        }
        .into_actor(self)
        .map(|r, act, ctx| r);
        Box::pin(fut)
    }
}
