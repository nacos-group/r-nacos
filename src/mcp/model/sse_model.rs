use actix::Message;
use bytes::Bytes;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct SseConnMetaInfo {
    pub session_id: Arc<String>,
    pub mcp_server_key: Arc<String>,
}

/// SSE 流管理器命令
#[derive(Message)]
#[rtype(result = "anyhow::Result<SseStreamManageResult>")]
pub enum SseStreamManageCmd {
    /// 添加新的 SSE 连接
    AddConn(
        SseConnMetaInfo,
        tokio::sync::mpsc::Sender<anyhow::Result<Bytes>>,
    ),
    /// 移除 SSE 连接
    RemoveConn(Arc<String>),
    GetMetaInfo(Arc<String>),
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<SseStreamManageResult>")]
pub enum SseStreamManageAsyncCmd {
    /// 发送消息到 SSE 连接
    SendMessage(Arc<String>, String),
}

/// SSE 流管理器命令结果
pub enum SseStreamManageResult {
    /// 空结果
    None,
    MetaInfo(Option<SseConnMetaInfo>),
}
