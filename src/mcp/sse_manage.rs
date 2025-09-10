use crate::mcp::model::sse_model::{
    SseConnMetaInfo, SseStreamManageAsyncCmd, SseStreamManageCmd, SseStreamManageResult,
};
use crate::{now_millis, now_second_i32};
use actix::prelude::*;
use bean_factory::{bean, Inject};
use bytes::Bytes;
use inner_mem_cache::TimeoutSet;
use serde::Serialize;
use std::collections::HashSet;
use std::{collections::HashMap, sync::Arc, time::Duration};

type SseSender = tokio::sync::mpsc::Sender<anyhow::Result<Bytes>>;

/// SSE 连接缓存项
pub(crate) struct SseConnCacheItem {
    /// 最后活动时间
    last_active_time: u64,
    /// SSE 连接发送器
    sender: SseSender,
    meta: SseConnMetaInfo,
}

impl SseConnCacheItem {
    fn new(sender: SseSender, meta: SseConnMetaInfo) -> Self {
        Self {
            last_active_time: now_second_i32() as u64,
            sender,
            meta,
        }
    }
}

/// SSE 流管理器
#[bean(inject)]
#[derive(Default)]
pub struct SseStreamManager {
    /// 连接缓存，存储所有活跃的 SSE 连接
    conn_cache: HashMap<Arc<String>, SseConnCacheItem>,

    /// 活动时间检测集合，用于心跳检测
    active_time_set: TimeoutSet<Arc<String>>,

    /// 心跳检测超时时间（默认15秒）
    detection_time_out: u64,
}

impl SseStreamManager {
    /// 创建新的 SSE 流管理器
    pub fn new() -> Self {
        Self {
            detection_time_out: 15, // 30秒
            ..Default::default()
        }
    }

    /// 添加新的 SSE 连接
    pub fn add_conn(&mut self, meta: SseConnMetaInfo, sender: SseSender) {
        let session_id = meta.session_id.clone();
        log::info!("add_sse_conn session_id:{}", &session_id);
        let now = now_second_i32() as u64;
        let item = SseConnCacheItem::new(sender, meta);

        // 如果已存在相同 session_id 的连接，先关闭旧连接
        if let Some(old_item) = self.conn_cache.insert(session_id.clone(), item) {
            log::info!("add_sse_conn remove old conn:{}", &session_id);
            drop(old_item); // 关闭旧连接
        }

        // 添加到活动时间检测集合
        self.active_time_set
            .add(now + self.detection_time_out, session_id);
    }

    /// 检查活动时间集合，处理超时连接
    fn check_active_time_set(&mut self, now: u64, ctx: &mut Context<Self>) {
        let keys = self.active_time_set.timeout(now);
        #[cfg(feature = "debug")]
        log::info!(
            "check_active_time_set,keys len:{},item_size:{}; conn size:{}",
            keys.len(),
            self.active_time_set.item_size(),
            self.conn_cache.len()
        );
        if keys.is_empty() {
            return;
        }
        let mut check_keys = HashSet::new();

        for key in keys {
            if check_keys.contains(&key) {
                continue;
            }
            if let Some(item) = self.conn_cache.get_mut(&key) {
                let next_time = now + self.detection_time_out;
                self.active_time_set.add(next_time, key.clone());
                item.last_active_time = next_time;
                // 连接超时，需要发送心跳检测
                check_keys.insert(key);
            } else {
                #[cfg(feature = "debug")]
                log::info!("not found conn,key:{}", &key);
            }
        }

        // 对超时的连接发送心跳消息
        if !check_keys.is_empty() {
            #[cfg(feature = "debug")]
            log::info!("check timeout SSE connections, size:{}", check_keys.len());
        } else {
            #[cfg(feature = "debug")]
            log::info!("check_keys is empty");
        }

        let heartbeat_message = SseConnUtils::create_heartbeat_message();
        for key in check_keys {
            ctx.address().do_send(SseStreamManageAsyncCmd::SendMessage(
                key,
                heartbeat_message.clone(),
            ));
        }
    }

    /// 移除指定的 SSE 连接
    pub fn remove_conn(&mut self, session_id: Arc<String>) {
        log::info!("remove_sse_conn session_id:{}", &session_id);
        self.conn_cache.remove(&session_id);
    }

    /// 定时心跳检测
    pub fn time_out_heartbeat(&self, ctx: &mut Context<Self>) {
        ctx.run_later(Duration::new(5, 0), |act, ctx| {
            let now = now_second_i32() as u64;
            act.check_active_time_set(now, ctx);
            act.time_out_heartbeat(ctx);
        });
    }
}

impl Actor for SseStreamManager {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.time_out_heartbeat(ctx);
        log::info!("SseStreamManage started");
    }
}

impl Inject for SseStreamManager {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        _factory_data: bean_factory::FactoryData,
        _factory: bean_factory::BeanFactory,
        _ctx: &mut Self::Context,
    ) {
        log::info!("SseStreamManage inject complete");
    }
}

impl Supervised for SseStreamManager {
    fn restarting(&mut self, _ctx: &mut <Self as Actor>::Context) {
        log::warn!("SseStreamManage restart ...");
    }
}
impl Handler<SseStreamManageCmd> for SseStreamManager {
    type Result = anyhow::Result<SseStreamManageResult>;

    fn handle(&mut self, msg: SseStreamManageCmd, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            SseStreamManageCmd::AddConn(session_id, sender) => {
                self.add_conn(session_id, sender);
                Ok(SseStreamManageResult::None)
            }
            SseStreamManageCmd::RemoveConn(session_id) => {
                self.remove_conn(session_id);
                Ok(SseStreamManageResult::None)
            }
            SseStreamManageCmd::GetMetaInfo(session_id) => {
                let meta = self
                    .conn_cache
                    .get(&session_id)
                    .map(|item| item.meta.clone());
                Ok(SseStreamManageResult::MetaInfo(meta))
            }
        }
    }
}

impl Handler<SseStreamManageAsyncCmd> for SseStreamManager {
    type Result = ResponseActFuture<Self, anyhow::Result<SseStreamManageResult>>;

    fn handle(&mut self, msg: SseStreamManageAsyncCmd, _ctx: &mut Context<Self>) -> Self::Result {
        let (sender, session_id, message) = match msg {
            SseStreamManageAsyncCmd::SendMessage(session_id, message) => (
                self.conn_cache
                    .get(&session_id)
                    .map(|item| item.sender.clone()),
                session_id,
                message,
            ),
        };
        let fut = async move {
            if let Some(sender) = sender {
                match sender.send(Ok(message.into())).await {
                    Ok(_) => (session_id, Ok(SseStreamManageResult::None)),
                    Err(e) => (
                        session_id,
                        Err(anyhow::anyhow!("Failed to send SSE message: {}", e)),
                    ),
                }
            } else {
                (
                    session_id,
                    Err(anyhow::anyhow!("SSE connection is unregistered.")),
                )
            }
        }
        .into_actor(self)
        .map(|(session_id, res), act, _ctx| match res {
            Ok(res) => Ok(res),
            Err(e) => {
                log::warn!(
                    "Failed to send SSE message to session_id: {}, error: {}",
                    &session_id,
                    &e
                );
                act.remove_conn(session_id);
                Err(e)
            }
        });
        Box::pin(fut)
    }
}

/// SSE 连接工具类
pub struct SseConnUtils;

impl SseConnUtils {
    /// 创建 SSE 消息格式
    pub fn create_sse_message<T: Serialize>(data: &T) -> String {
        let json_string = serde_json::to_string(data).unwrap_or_default();
        format!("event: message\ndata: {}\n\n", json_string)
    }

    /// 创建 SSE 心跳消息
    pub fn create_heartbeat_message() -> String {
        format!(": ping - {}\n\n", now_millis())
    }
}
