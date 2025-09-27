use crate::common::pb::data_object::{McpServerDo, McpServerValueDo};
use crate::mcp::model::tools::{McpSimpleTool, McpTool, ToolKey, ToolSpec};
use crate::mcp::utils::ToolSpecUtils;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// MCP 服务器值
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerValue {
    pub id: u64,
    pub description: Arc<String>,
    pub tools: Vec<McpTool>,
    pub op_user: Arc<String>,
    pub update_time: i64,
}

impl McpServerValue {
    pub fn update_param(
        &mut self,
        param: McpServerParam,
        tool_spec_map: &BTreeMap<ToolKey, Arc<ToolSpec>>,
    ) -> HashMap<ToolKey, HashMap<u64, i64>> {
        let mut tool_spec_version_ref_map = HashMap::new();
        if let Some(description) = param.description {
            self.description = description;
        }
        let mut tools = Vec::with_capacity(param.tools.len());
        for item in param.tools {
            let tool = item.to_mcp_tool(tool_spec_map);
            ToolSpecUtils::add_tool_ref_to_map(
                &mut tool_spec_version_ref_map,
                &tool.tool_key,
                tool.tool_version,
                1,
            );
            tools.push(tool);
        }
        if param.value_id > 0 {
            self.id = param.value_id;
        }
        for old_tool in &self.tools {
            ToolSpecUtils::add_tool_ref_to_map(
                &mut tool_spec_version_ref_map,
                &old_tool.tool_key,
                old_tool.tool_version,
                -1,
            );
        }
        self.tools = tools;
        self.op_user = param.op_user;
        self.update_time = param.update_time;
        tool_spec_version_ref_map
    }

    pub fn to_do(&self) -> McpServerValueDo<'_> {
        McpServerValueDo {
            id: self.id,
            description: Cow::Borrowed(self.description.as_str()),
            tools: self.tools.iter().map(|tool| tool.to_do()).collect(),
            op_user: Cow::Borrowed(self.op_user.as_str()),
            update_time: self.update_time,
        }
    }

    pub fn from_do(
        record_do: McpServerValueDo,
        tool_spec_map: &BTreeMap<ToolKey, Arc<ToolSpec>>,
    ) -> Self {
        let mut tools = Vec::with_capacity(record_do.tools.len());
        for item in record_do.tools {
            let simple_tool = McpSimpleTool::from(item);
            let tool = simple_tool.to_mcp_tool(tool_spec_map);
            tools.push(tool);
        }
        Self {
            id: record_do.id,
            description: Arc::new(record_do.description.to_string()),
            tools,
            op_user: Arc::new(record_do.op_user.to_string()),
            update_time: record_do.update_time,
        }
    }
}

/// MCP 服务器
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServer {
    pub id: u64,
    pub unique_key: Arc<String>,
    pub namespace: Arc<String>,
    pub name: Arc<String>,
    pub description: Arc<String>,
    pub auth_keys: Vec<Arc<String>>,
    pub create_time: i64,
    pub create_user: Arc<String>,
    pub current_value: Arc<McpServerValue>,
    pub release_value: Arc<McpServerValue>,
    pub histories: Vec<Arc<McpServerValue>>,
}

impl McpServer {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            ..Self::default()
        }
    }

    pub fn update_param(
        &mut self,
        param: McpServerParam,
        tool_spec_map: &BTreeMap<ToolKey, Arc<ToolSpec>>,
    ) -> HashMap<ToolKey, HashMap<u64, i64>> {
        if let Some(unique_key) = param.unique_key.as_ref() {
            self.unique_key = unique_key.clone();
        }
        if let Some(description) = param.description.as_ref() {
            self.description = description.clone();
        }
        if let Some(namespace) = param.namespace.as_ref() {
            self.namespace = namespace.clone();
        }
        if let Some(name) = param.name.as_ref() {
            self.name = name.clone();
        }
        if let Some(auth_keys) = param.auth_keys.as_ref() {
            self.auth_keys = auth_keys.clone();
        }
        if self.create_time == 0 {
            self.create_time = param.update_time;
            self.create_user = param.op_user.clone();
        }
        let mut current_value = self.current_value.as_ref().to_owned();
        let ref_map = current_value.update_param(param, tool_spec_map);
        self.current_value = Arc::new(current_value);
        ref_map
    }

    pub fn check_valid(&self) -> anyhow::Result<()> {
        if self.id == 0 {
            return Err(anyhow::anyhow!("id is empty!"));
        }
        if self.name.is_empty() {
            return Err(anyhow::anyhow!("name is empty!"));
        }
        if self.auth_keys.is_empty() {
            return Err(anyhow::anyhow!("auth_keys is empty!"));
        }
        Ok(())
    }

    pub fn publish(&mut self, new_value_id: u64) -> Option<Arc<McpServerValue>> {
        let mut new_value = self.current_value.as_ref().to_owned();
        new_value.id = new_value_id;
        self.release_value = self.current_value.clone();
        self.current_value = Arc::new(new_value);
        self.histories.push(self.release_value.clone());
        if self.histories.len() > 10 {
            Some(self.histories.remove(0))
        } else {
            None
        }
    }

    pub fn public_history(&mut self, history_id: u64) -> anyhow::Result<()> {
        if let Some(v) = self.histories.iter().find(|value| value.id == history_id) {
            self.release_value = v.clone();
            Ok(())
        } else {
            Err(anyhow::anyhow!("value not found"))
        }
    }

    pub fn to_do(&self) -> McpServerDo<'_> {
        McpServerDo {
            id: self.id,
            unique_key: Cow::Borrowed(self.unique_key.as_str()),
            namespace: Cow::Borrowed(self.namespace.as_str()),
            name: Cow::Borrowed(self.name.as_str()),
            description: Cow::Borrowed(self.description.as_str()),
            auth_keys: self
                .auth_keys
                .iter()
                .map(|key| Cow::Borrowed(key.as_str()))
                .collect(),
            create_time: self.create_time,
            create_user: Cow::Borrowed(self.create_user.as_str()),
            current_value: Some(self.current_value.to_do()),
            release_value: Some(self.release_value.to_do()),
            histories: self
                .histories
                .iter()
                .map(|history| history.to_do())
                .collect(),
        }
    }

    pub fn from_do(
        record_do: McpServerDo,
        tool_spec_map: &BTreeMap<ToolKey, Arc<ToolSpec>>,
    ) -> Self {
        let current_value = if let Some(current_value_do) = record_do.current_value {
            McpServerValue::from_do(current_value_do, tool_spec_map)
        } else {
            McpServerValue::default()
        };

        let release_value = if let Some(release_value_do) = record_do.release_value {
            McpServerValue::from_do(release_value_do, tool_spec_map)
        } else {
            McpServerValue::default()
        };

        let mut histories = Vec::with_capacity(record_do.histories.len());
        for history_do in record_do.histories {
            histories.push(Arc::new(McpServerValue::from_do(history_do, tool_spec_map)));
        }

        Self {
            id: record_do.id,
            unique_key: Arc::new(record_do.unique_key.to_string()),
            namespace: Arc::new(record_do.namespace.to_string()),
            name: Arc::new(record_do.name.to_string()),
            description: Arc::new(record_do.description.to_string()),
            auth_keys: record_do
                .auth_keys
                .into_iter()
                .map(|key| Arc::new(key.to_string()))
                .collect(),
            create_time: record_do.create_time,
            create_user: Arc::new(record_do.create_user.to_string()),
            current_value: Arc::new(current_value),
            release_value: Arc::new(release_value),
            histories,
        }
    }
}

/// MCP 服务器参数
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerParam {
    pub id: u64,
    pub unique_key: Option<Arc<String>>,
    pub value_id: u64,
    pub tools: Vec<McpSimpleTool>,
    pub op_user: Arc<String>,
    pub update_time: i64,
    pub namespace: Option<Arc<String>>,
    pub name: Option<Arc<String>>,
    pub description: Option<Arc<String>>,
    pub token: Option<Arc<String>>,
    pub auth_keys: Option<Vec<Arc<String>>>,
    /// 发布后的版本id,只在创建并发布时有值
    pub publish_value_id: Option<u64>,
}

impl McpServerParam {
    pub fn build_unique_key(&self) -> Arc<String> {
        Arc::new(format!("{}_{}", self.update_time, &self.id))
    }
}

/// MCP 服务器 DTO
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerDto {
    pub id: u64,
    pub unique_key: Arc<String>,
    pub namespace: Arc<String>,
    pub name: Arc<String>,
    pub description: Arc<String>,
    pub auth_keys: Vec<Arc<String>>,
    pub create_time: i64,
    pub last_modified_millis: i64,
    pub current_value: Option<Arc<McpServerValue>>,
    pub release_value: Option<Arc<McpServerValue>>,
    pub histories: Option<Vec<Arc<McpServerValue>>>,
}

impl McpServerDto {
    pub fn new_simple_from(server: &McpServer) -> Self {
        Self {
            id: server.id,
            unique_key: server.unique_key.clone(),
            namespace: server.namespace.clone(),
            name: server.name.clone(),
            description: server.description.clone(),
            auth_keys: server.auth_keys.clone(),
            create_time: server.create_time, // 实际实现中需要从 server 获取
            last_modified_millis: server.current_value.update_time, // 实际实现中需要从 server 获取
            current_value: None,
            release_value: None,
            histories: None,
        }
    }
    pub fn new_from(server: &McpServer) -> Self {
        let release_value = if server.release_value.id > 0 {
            Some(server.release_value.clone())
        } else {
            None
        };
        Self {
            id: server.id,
            unique_key: server.unique_key.clone(),
            namespace: server.namespace.clone(),
            name: server.name.clone(),
            description: server.description.clone(),
            auth_keys: server.auth_keys.clone(),
            create_time: server.create_time, // 实际实现中需要从 server 获取
            last_modified_millis: server.current_value.update_time, // 实际实现中需要从 server 获取
            current_value: Some(server.current_value.clone()),
            release_value,
            histories: Some(server.histories.clone()),
        }
    }
}

/// MCP 查询参数
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpQueryParam {
    pub offset: usize,
    pub limit: usize,
    pub namespace_id: Option<Arc<String>>,
    pub name_filter: Option<Arc<String>>,
}

impl McpQueryParam {
    pub fn match_namespace(&self, namespace: &Arc<String>) -> bool {
        if let Some(ref namespace_id) = self.namespace_id {
            namespace == namespace_id
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
