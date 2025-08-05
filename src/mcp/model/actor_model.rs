use crate::mcp::model::mcp::{
    McpQueryParam, McpServer, McpServerDto, McpServerParam, ToolSpec, ToolSpecParam,
};
use actix::Message;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// MCP 查询请求
#[derive(Debug, Message)]
#[rtype(result = "anyhow::Result<McpManagerResult>")]
pub enum McpManagerReq {
    GetServer(u64),
    QueryServer(McpQueryParam),
    GetToolSpec(String, String, String),
    QueryToolSpec(McpToolSpecQueryParam),
}

/// MCP 查询结果
#[derive(Debug, Clone)]
pub enum McpManagerResult {
    ServerInfo(Option<Arc<McpServer>>),
    ServerPageInfo(usize, Vec<McpServerDto>),
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
    RemoveServer(u64),
    AddToolSpec(ToolSpecParam),
    UpdateToolSpec(ToolSpecParam),
    RemoveToolSpec(String, String, String),
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
    pub namespace_filter: Option<String>,
    pub group_filter: Option<String>,
    pub tool_name_filter: Option<String>,
}

/// MCP 工具规范 DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSpecDto {
    pub namespace: String,
    pub group: String,
    pub tool_name: String,
    pub version: u64,
    pub name: String,
    pub description: String,
    pub create_time: u64,
    pub last_modified_millis: u64,
}

impl ToolSpecDto {
    pub fn new_from(tool_spec: &ToolSpec) -> Self {
        Self {
            namespace: tool_spec.key.namespace.to_string(),
            group: tool_spec.key.group.to_string(),
            tool_name: tool_spec.key.tool_name.to_string(),
            version: tool_spec.version,
            name: tool_spec.name.to_string(),
            description: tool_spec.description.to_string(),
            create_time: 0,          // 实际实现中需要从 tool_spec 获取
            last_modified_millis: 0, // 实际实现中需要从 tool_spec 获取
        }
    }
}
