use crate::common::constant::EMPTY_ARC_STRING;
use crate::common::pb::data_object::{McpServerDo, McpToolDo, McpToolSpecDo, ToolRouteRuleDo};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

/// MCP 工具键，用于唯一标识一个工具规范
#[derive(Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolKey {
    pub namespace: Arc<String>,
    pub group: Arc<String>,
    pub tool_name: Arc<String>,
}

impl ToolKey {
    pub fn new(namespace: Arc<String>, group: Arc<String>, tool_name: Arc<String>) -> Self {
        Self {
            namespace,
            group,
            tool_name,
        }
    }
}

impl Default for ToolKey {
    fn default() -> Self {
        Self {
            namespace: Arc::new(String::new()),
            group: Arc::new(String::new()),
            tool_name: Arc::new(String::new()),
        }
    }
}

/// JSON Schema 类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JsonType {
    Object,
    Array,
    String,
    Integer,
    Number,
    Boolean,
    Null,
}

/// JSON Schema 定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    #[serde(rename = "type")]
    pub schema_type: JsonType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<std::collections::HashMap<String, Box<JsonSchema>>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<JsonSchema>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_items: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_items: Option<u32>,
}

impl JsonSchema {
    pub fn new_object() -> Self {
        Self {
            schema_type: JsonType::Object,
            properties: Some(std::collections::HashMap::new()),
            items: None,
            required: None,
            description: None,
            format: None,
            min_items: None,
            max_items: None,
        }
    }

    pub fn new_array(item_type: JsonSchema) -> Self {
        Self {
            schema_type: JsonType::Array,
            properties: None,
            items: Some(Box::new(item_type)),
            required: None,
            description: None,
            format: None,
            min_items: None,
            max_items: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn add_property(mut self, name: impl Into<String>, property: JsonSchema) -> Self {
        if let Some(ref mut props) = self.properties {
            props.insert(name.into(), Box::new(property));
        }
        self
    }

    pub fn add_required(mut self, required_fields: Vec<String>) -> Self {
        self.required = Some(required_fields);
        self
    }
}

/// 工具参数定义
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolParameters {
    #[serde(rename = "type")]
    pub param_type: JsonType,
    pub properties: std::collections::HashMap<String, Box<JsonSchema>>,
    pub required: Option<Vec<String>>,
}

impl Default for ToolParameters {
    fn default() -> Self {
        Self {
            param_type: JsonType::Object,
            properties: std::collections::HashMap::new(),
            required: None,
        }
    }
}

/// MCP 工具规范
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSpec {
    pub key: ToolKey,
    pub version: u64,
    pub name: Arc<String>,
    pub description: Arc<String>,
    pub parameters: ToolParameters,
}

impl ToolSpec {
    pub fn update_param(&mut self, param: ToolSpecParam) {
        if let Some(name) = param.name {
            self.name = name;
        }
        if let Some(description) = param.description {
            self.description = description;
        }
        if let Some(parameters) = param.parameters {
            self.parameters = parameters;
        }
        if let Some(_update_time) = param.update_time {
            // TODO 在实际实现中，这里应该更新时间戳
        }
    }

    pub fn check_valid(&self) -> anyhow::Result<()> {
        if self.key.namespace.is_empty()
            || self.key.group.is_empty()
            || self.key.tool_name.is_empty()
        {
            return Err(anyhow::anyhow!("ToolKey fields cannot be empty"));
        }
        if self.name.is_empty() {
            return Err(anyhow::anyhow!("ToolSpec name cannot be empty"));
        }
        Ok(())
    }

    pub fn to_do(&self) -> McpToolSpecDo {
        McpToolSpecDo {
            namespace: Cow::Borrowed(&self.key.namespace),
            group: Cow::Borrowed(&self.key.group),
            tool_name: Cow::Borrowed(&self.key.tool_name),
            version: self.version,
            name: Cow::Borrowed(&self.name),
            description: Cow::Borrowed(&self.description),
        }
    }
}

impl<'a> From<McpToolSpecDo<'a>> for ToolSpec {
    fn from(do_obj: McpToolSpecDo<'a>) -> Self {
        Self {
            key: ToolKey {
                namespace: Arc::new(do_obj.namespace.to_string()),
                group: Arc::new(do_obj.group.to_string()),
                tool_name: Arc::new(do_obj.tool_name.to_string()),
            },
            version: do_obj.version,
            name: Arc::new(do_obj.name.to_string()),
            description: Arc::new(do_obj.description.to_string()),
            parameters: ToolParameters {
                param_type: JsonType::Object,
                properties: std::collections::HashMap::new(),
                required: None,
            },
        }
    }
}

/// 工具规范参数
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSpecParam {
    pub namespace: Option<Arc<String>>,
    pub group: Option<Arc<String>>,
    pub tool_name: Option<Arc<String>>,
    pub name: Option<Arc<String>>,
    pub description: Option<Arc<String>>,
    pub parameters: Option<ToolParameters>,
    pub update_time: Option<u64>,
}

impl From<ToolSpecParam> for ToolSpec {
    fn from(param: ToolSpecParam) -> Self {
        Self {
            key: ToolKey {
                namespace: param.namespace.unwrap_or_default(),
                group: param.group.unwrap_or_default(),
                tool_name: param.tool_name.unwrap_or_default(),
            },
            version: 0,
            name: param.name.unwrap_or_default(),
            description: param.description.unwrap_or_default(),
            parameters: param.parameters.unwrap_or(ToolParameters {
                param_type: JsonType::Object,
                properties: std::collections::HashMap::new(),
                required: None,
            }),
        }
    }
}

/// 路由规则转换类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConvertType {
    None,
    FormToJson,
    Custom,
}

/// 工具路由规则
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRouteRule {
    pub protocol: Arc<String>,
    pub url: Arc<String>,
    pub method: Arc<String>,
    pub addition_headers: std::collections::HashMap<String, Arc<String>>,
    pub convert_type: ConvertType,
    pub service_namespace: Arc<String>,
    pub service_group: Arc<String>,
    pub service_name: Arc<String>,
}

impl ToolRouteRule {
    pub fn to_do(&self) -> ToolRouteRuleDo {
        ToolRouteRuleDo {
            protocol: Cow::Borrowed(&self.protocol),
            url: Cow::Borrowed(&self.url),
            method: Cow::Borrowed(&self.method),
        }
    }
}

impl Default for ToolRouteRule {
    fn default() -> Self {
        Self {
            protocol: Arc::new("http".to_string()),
            url: Arc::new("/".to_string()),
            method: Arc::new("GET".to_string()),
            addition_headers: std::collections::HashMap::new(),
            convert_type: ConvertType::None,
            service_namespace: EMPTY_ARC_STRING.clone(),
            service_group: EMPTY_ARC_STRING.clone(),
            service_name: EMPTY_ARC_STRING.clone(),
        }
    }
}

impl<'a> From<ToolRouteRuleDo<'a>> for ToolRouteRule {
    fn from(do_obj: ToolRouteRuleDo<'a>) -> Self {
        Self {
            protocol: Arc::new(do_obj.protocol.to_string()),
            url: Arc::new(do_obj.url.to_string()),
            method: Arc::new(do_obj.method.to_string()),
            addition_headers: std::collections::HashMap::new(),
            convert_type: ConvertType::None,
            service_namespace: EMPTY_ARC_STRING.clone(),
            service_group: EMPTY_ARC_STRING.clone(),
            service_name: EMPTY_ARC_STRING.clone(),
        }
    }
}

/// MCP 工具
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpTool {
    pub id: u64,
    pub tool_name: Arc<String>,
    pub tool_key: ToolKey,
    pub version: u64,
    pub spec: Arc<ToolSpec>,
    pub route_rule: ToolRouteRule,
}

impl McpTool {
    pub fn to_do(&self) -> McpToolDo {
        McpToolDo {
            id: self.id,
            tool_name: Cow::Borrowed(&self.tool_name),
            namespace: Cow::Borrowed(&self.tool_key.namespace),
            group: Cow::Borrowed(&self.tool_key.group),
            version: self.version,
        }
    }
}

impl<'a> From<McpToolDo<'a>> for McpTool {
    fn from(do_obj: McpToolDo<'a>) -> Self {
        Self {
            id: do_obj.id,
            tool_name: Arc::new(do_obj.tool_name.to_string()),
            tool_key: ToolKey {
                namespace: Arc::new(do_obj.namespace.to_string()),
                group: Arc::new(do_obj.group.to_string()),
                tool_name: Arc::new(do_obj.tool_name.to_string()),
            },
            version: do_obj.version,
            spec: Arc::new(ToolSpec::default()),
            route_rule: ToolRouteRule::default(),
        }
    }
}

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
