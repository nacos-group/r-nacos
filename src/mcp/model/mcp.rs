use crate::common::constant::EMPTY_ARC_STRING;
use crate::common::pb::data_object::McpServerDo;
use crate::mcp::model::tools::{McpTool, ToolSpec};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

/// MCP 服务器值
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerValue {
    pub description: Arc<String>,
    pub tools: Vec<McpTool>,
}

impl McpServerValue {
    pub fn new(description: Arc<String>) -> Self {
        Self {
            description,
            tools: Vec::new(),
        }
    }
}

/// MCP 服务器
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServer {
    pub id: u64,
    pub namespace: Arc<String>,
    pub name: Arc<String>,
    pub description: Arc<String>,
    pub token: Arc<String>,
    pub auth_keys: Vec<Arc<String>>,
    pub current_value: Option<Arc<McpServerValue>>,
    pub draft_value: Option<Arc<McpServerValue>>,
    pub histories: Vec<Arc<McpServerValue>>,
}

impl McpServer {
    pub fn new(id: u64, namespace: Arc<String>, name: Arc<String>, token: Arc<String>) -> Self {
        Self {
            id,
            namespace,
            name,
            description: EMPTY_ARC_STRING.clone(),
            token,
            auth_keys: Vec::new(),
            current_value: None,
            draft_value: None,
            histories: Vec::new(),
        }
    }

    pub fn update_param(&mut self, param: McpServerParam) {
        if let Some(name) = param.name {
            self.name = name;
        }
        if let Some(description) = param.description {
            self.description = description;
        }
        if let Some(token) = param.token {
            self.token = token;
        }
        if let Some(_update_time) = param.update_time {
            // TODO 在实际实现中，这里应该更新时间戳
        }
    }

    pub fn check_valid(&self) -> anyhow::Result<()> {
        if self.id == 0 {
            return Err(anyhow::anyhow!("id is empty!"));
        }
        if self.namespace.is_empty() {
            return Err(anyhow::anyhow!("namespace is empty!"));
        }
        if self.name.is_empty() {
            return Err(anyhow::anyhow!("name is empty!"));
        }
        if self.token.is_empty() {
            return Err(anyhow::anyhow!("token is empty!"));
        }
        Ok(())
    }

    pub fn to_do(&self) -> McpServerDo {
        McpServerDo {
            id: self.id,
            namespace: Cow::Borrowed(&self.namespace),
            name: Cow::Borrowed(&self.name),
            description: Cow::Borrowed(&self.description),
            token: Cow::Borrowed(&self.token),
        }
    }
}

impl<'a> From<McpServerDo<'a>> for McpServer {
    fn from(do_obj: McpServerDo<'a>) -> Self {
        Self {
            id: do_obj.id,
            namespace: Arc::new(do_obj.namespace.to_string()),
            name: Arc::new(do_obj.name.to_string()),
            description: Arc::new(do_obj.description.to_string()),
            token: Arc::new(do_obj.token.to_string()),
            auth_keys: Vec::new(),
            current_value: None,
            draft_value: None,
            histories: Vec::new(),
        }
    }
}

/// MCP 服务器参数
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerParam {
    pub id: Option<u64>,
    pub namespace: Option<Arc<String>>,
    pub name: Option<Arc<String>>,
    pub description: Option<Arc<String>>,
    pub token: Option<Arc<String>>,
    pub update_time: Option<u64>,
}

impl From<McpServerParam> for McpServer {
    fn from(param: McpServerParam) -> Self {
        Self {
            id: param.id.unwrap_or_default(),
            namespace: param.namespace.unwrap_or_default(),
            name: param.name.unwrap_or_default(),
            description: param.description.unwrap_or_default(),
            token: param
                .token
                .unwrap_or_else(|| Arc::new(uuid::Uuid::new_v4().to_string())),
            auth_keys: Vec::new(),
            current_value: None,
            draft_value: None,
            histories: Vec::new(),
        }
    }
}

/// MCP 服务器 DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerDto {
    pub id: u64,
    pub namespace: Arc<String>,
    pub name: Arc<String>,
    pub description: Arc<String>,
    pub token: Arc<String>,
    pub create_time: u64,
    pub last_modified_millis: u64,
}

impl McpServerDto {
    pub fn new_from(server: &McpServer) -> Self {
        Self {
            id: server.id,
            namespace: server.namespace.clone(),
            name: server.name.clone(),
            description: server.description.clone(),
            token: server.token.clone(),
            create_time: 0,          // 实际实现中需要从 server 获取
            last_modified_millis: 0, // 实际实现中需要从 server 获取
        }
    }
}

/// MCP 查询参数
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpQueryParam {
    pub offset: usize,
    pub limit: usize,
    pub namespace_filter: Option<Arc<String>>,
    pub name_filter: Option<Arc<String>>,
}

impl McpQueryParam {
    pub fn match_namespace(&self, namespace: &Arc<String>) -> bool {
        if let Some(ref filter) = self.namespace_filter {
            namespace.contains(filter.as_str())
        } else {
            true
        }
    }

    pub fn match_name(&self, name: &Arc<String>) -> bool {
        if let Some(ref filter) = self.name_filter {
            name.contains(filter.as_str())
        } else {
            true
        }
    }
}

/// MCP 服务器包装器
pub struct McpServerWrap {
    pub server: Arc<McpServer>,
    pub related_data: BTreeMap<u64, Arc<ToolSpec>>,
}

impl McpServerWrap {
    pub fn new(server: Arc<McpServer>) -> Self {
        Self {
            server,
            related_data: BTreeMap::new(),
        }
    }
}
