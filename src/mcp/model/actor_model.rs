use crate::mcp::model::mcp::{
    McpQueryParam, McpServer, McpServerDto, McpServerParam, McpServerValue,
};
use crate::mcp::model::tools::{ToolFunctionValue, ToolKey, ToolSpec, ToolSpecParam};
use actix::Message;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// MCP 查询请求
#[derive(Debug, Message)]
#[rtype(result = "anyhow::Result<McpManagerResult>")]
pub enum McpManagerReq {
    GetServer(u64),
    GetServerByKey(Arc<String>),
    QueryServer(McpQueryParam),
    QueryServerHistory(u64, usize, usize, Option<i64>, Option<i64>),
    GetToolSpec(ToolKey),
    QueryToolSpec(McpToolSpecQueryParam),
}

/// MCP 查询结果
#[derive(Debug, Clone)]
pub enum McpManagerResult {
    ServerInfo(Option<Arc<McpServer>>),
    ServerPageInfo(usize, Vec<McpServerDto>),
    ServerHistoryPageInfo(usize, Vec<McpServerValue>),
    ToolSpecInfo(Option<Arc<ToolSpec>>),
    ToolSpecPageInfo(usize, Vec<ToolSpecDto>),
    None,
}

/// MCP Raft 请求
#[derive(Debug, Clone, Message, Deserialize, Serialize)]
#[rtype(result = "anyhow::Result<McpManagerRaftResult>")]
pub enum McpManagerRaftReq {
    AddServer(McpServerParam),
    UpdateServer(McpServerParam),
    PublishCurrentServer(u64, u64),
    PublishHistoryServer(u64, u64),
    RemoveServer(u64),
    SetServer(McpServer),
    UpdateToolSpec(ToolSpecParam),
    UpdateToolSpecList(Vec<ToolSpecParam>),
    RemoveToolSpec(ToolKey),
    SetToolSpec(Arc<ToolSpec>),
    ImportFinished,
}

/// MCP Raft 结果
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum McpManagerRaftResult {
    ServerInfo(Arc<McpServer>),
    ToolSpecInfo(Arc<ToolSpec>),
    None,
}

/// MCP 工具规范查询参数
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolSpecQueryParam {
    pub offset: usize,
    pub limit: usize,
    pub namespace_id: Option<String>,
    pub group_filter: Option<String>,
    pub tool_name_filter: Option<String>,
}

/// MCP 工具规范 DTO
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSpecDto {
    pub namespace: Arc<String>,
    pub group: Arc<String>,
    pub tool_name: Arc<String>,
    pub version: u64,
    pub name: Arc<String>,
    pub description: Arc<String>,
    pub create_time: i64,
    pub last_modified_millis: i64,
    pub function: Arc<ToolFunctionValue>,
}

impl ToolSpecDto {
    pub fn new_from(tool_spec: &ToolSpec) -> Self {
        let current_version = tool_spec.get_current_version().unwrap_or_default();
        Self {
            namespace: tool_spec.key.namespace.clone(),
            group: tool_spec.key.group.clone(),
            tool_name: tool_spec.key.tool_name.clone(),
            version: tool_spec.current_version,
            name: current_version.function.name.clone(),
            description: current_version.function.description.clone(),
            create_time: tool_spec.create_time,
            last_modified_millis: current_version.update_time,
            function: current_version.function.clone(),
        }
    }
}
