use crate::common::constant::EMPTY_ARC_STRING;
use crate::common::pb::data_object::{McpToolDo, McpToolSpecDo, ToolSpecVersionDo};
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

impl Default for JsonSchema {
    fn default() -> Self {
        Self::new_object()
    }
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
/// 对应function 内容
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolFunctionValue {
    pub name: Arc<String>,
    pub description: Arc<String>,
    pub input_schema: Box<JsonSchema>,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct ToolSpecVersion {
    pub version: u64,
    pub function: Arc<ToolFunctionValue>,
    pub op_user: Arc<String>,
    pub update_time: i64,
    pub ref_count: i64,
}

impl ToolSpecVersion {
    pub fn to_do(&self) -> ToolSpecVersionDo<'_> {
        ToolSpecVersionDo {
            version: self.version,
            parameters_json: Cow::Owned(serde_json::to_string(&self.function).unwrap_or_default()),
            op_user: Cow::Borrowed(self.op_user.as_ref()),
            update_time: self.update_time,
        }
    }

    pub fn from_param(param: ToolSpecParam) -> Self {
        Self {
            version: param.version,
            function: Arc::new(param.parameters),
            op_user: param.op_user.unwrap_or_default(),
            update_time: param.update_time,
            ref_count: 0,
        }
    }
}

/// MCP 工具规范
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct ToolSpec {
    pub key: ToolKey,
    pub current_version: u64,
    pub create_time: i64,
    pub create_user: Arc<String>,
    pub versions: BTreeMap<u64, ToolSpecVersion>,
}

impl ToolSpec {
    pub fn get_current_value(&self) -> Option<Arc<ToolFunctionValue>> {
        self.versions
            .get(&self.current_version)
            .map(|v| v.function.clone())
    }

    pub fn get_current_version(&self) -> Option<ToolSpecVersion> {
        self.versions.get(&self.current_version).map(|v| v.clone())
    }

    pub fn update_param(&mut self, param: ToolSpecParam) {
        let old_version = self.current_version;
        let old_ref_count = self
            .get_current_version()
            .map(|v| v.ref_count)
            .unwrap_or_default();
        let new_version = param.version;
        // 获取当前版本的参数作为基础
        let spec_version = ToolSpecVersion::from_param(param);
        self.versions.insert(new_version, spec_version);
        self.current_version = new_version;
        if old_ref_count == 0 {
            self.versions.remove(&old_version);
        }
    }

    pub fn check_valid(&self) -> anyhow::Result<()> {
        if self.key.namespace.is_empty()
            || self.key.group.is_empty()
            || self.key.tool_name.is_empty()
        {
            return Err(anyhow::anyhow!("ToolKey fields cannot be empty"));
        }
        Ok(())
    }

    pub fn to_do(&self) -> McpToolSpecDo<'_> {
        let current_version = self.get_current_version().unwrap_or_default();

        McpToolSpecDo {
            namespace: Cow::Borrowed(&self.key.namespace),
            group: Cow::Borrowed(&self.key.group),
            tool_name: Cow::Borrowed(&self.key.tool_name),
            current_version: current_version.version,
            create_time: self.create_time,
            create_user: Cow::Borrowed(&self.create_user),
            versions: self.versions.values().map(|v| v.to_do()).collect(),
        }
    }
}

impl<'a> From<McpToolSpecDo<'a>> for ToolSpec {
    fn from(do_obj: McpToolSpecDo<'a>) -> Self {
        let tool_key = ToolKey::new(
            Arc::new(do_obj.namespace.to_string()),
            Arc::new(do_obj.group.to_string()),
            Arc::new(do_obj.tool_name.to_string()),
        );

        let mut versions = BTreeMap::new();
        for version in do_obj.versions {
            let parameters = serde_json::from_str::<ToolFunctionValue>(&version.parameters_json)
                .unwrap_or_default();
            let spec_version = ToolSpecVersion {
                version: version.version,
                function: Arc::new(parameters),
                op_user: Arc::new(version.op_user.to_string()),
                update_time: version.update_time,
                ref_count: 0,
            };
            versions.insert(spec_version.version, spec_version);
        }

        Self {
            key: tool_key,
            current_version: do_obj.current_version,
            create_time: do_obj.create_time,
            create_user: Arc::new(do_obj.create_user.to_string()),
            versions,
        }
    }
}

/// 工具规范参数
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSpecParam {
    pub namespace: Arc<String>,
    pub group: Arc<String>,
    pub tool_name: Arc<String>,
    pub parameters: ToolFunctionValue,
    pub version: u64,
    pub update_time: i64,
    pub op_user: Option<Arc<String>>,
}

impl ToolSpecParam {
    pub fn build_key(&self) -> ToolKey {
        ToolKey::new(
            self.namespace.clone(),
            self.group.clone(),
            self.tool_name.clone(),
        )
    }
}

impl From<ToolSpecParam> for ToolSpec {
    fn from(param: ToolSpecParam) -> Self {
        let tool_key = ToolKey::new(param.namespace, param.group, param.tool_name);

        let mut versions = BTreeMap::new();
        let op_user = param.op_user.unwrap_or_else(|| EMPTY_ARC_STRING.clone());
        let spec_version = ToolSpecVersion {
            version: param.version,
            function: Arc::new(param.parameters),
            op_user: op_user.clone(),
            update_time: param.update_time,
            ref_count: 0,
        };

        versions.insert(param.version, spec_version);

        Self {
            key: tool_key,
            current_version: param.version,
            create_time: param.update_time,
            create_user: op_user,
            versions,
        }
    }
}

/// 路由规则转换类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConvertType {
    None,
    JsonToForm,
    JsonToUrl,
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

impl ToolRouteRule {
    pub fn is_need_host(&self) -> bool {
        if let Some(i) = self.url.find("/") {
            if i == 0 {
                return true;
            }
        }
        self.url.find("{IP_PORT}").is_some()
    }
    pub fn build_url(&self, host: Option<(Arc<String>, u16)>) -> anyhow::Result<String> {
        if let Some((ip, port)) = host {
            if let Some(i) = self.url.find("/") {
                if i == 0 {
                    return Ok(format!("http:/{}:{}{}", ip, port, self.url));
                }
            }
            Ok(self.url.replace("{IP_PORT}", &format!("{}:{}", ip, port)))
        } else {
            if self.is_need_host() {
                Err(anyhow::anyhow!("build url error: host is empty"))
            } else {
                Ok(self.url.as_ref().to_string())
            }
        }
    }
}

/// MCP 工具轻量对象
/// 不包含spec字段
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSimpleTool {
    pub tool_name: Arc<String>,
    pub tool_key: ToolKey,
    pub tool_version: u64,
    pub route_rule: ToolRouteRule,
}

impl McpSimpleTool {
    pub fn to_mcp_tool(self, tool_spec_map: &BTreeMap<ToolKey, Arc<ToolSpec>>) -> McpTool {
        let tool_value = if let Some(tool_spec) = tool_spec_map.get(&self.tool_key) {
            if let Some(v) = tool_spec.versions.get(&self.tool_version) {
                v.function.clone()
            } else if let Some(v) = tool_spec.versions.get(&tool_spec.current_version) {
                #[cfg(feature = "debug")]
                log::warn!(
                    "tool_spec not found, use default; key:{:?},tool_version:{}",
                    &self.tool_key,
                    &self.tool_version
                );
                v.function.clone()
            } else {
                Arc::new(ToolFunctionValue::default())
            }
        } else {
            Arc::new(ToolFunctionValue::default())
        };
        McpTool {
            tool_name: self.tool_name,
            tool_key: self.tool_key,
            tool_version: self.tool_version,
            spec: tool_value,
            route_rule: self.route_rule,
        }
    }
}

impl<'a> From<McpToolDo<'a>> for McpSimpleTool {
    fn from(do_obj: McpToolDo<'a>) -> Self {
        // 反序列化route_rule
        let route_rule =
            serde_json::from_str::<ToolRouteRule>(&do_obj.route_rule_json).unwrap_or_default();

        Self {
            tool_name: Arc::new(do_obj.tool_name.to_string()),
            tool_key: ToolKey {
                namespace: Arc::new(do_obj.namespace.to_string()),
                group: Arc::new(do_obj.group.to_string()),
                tool_name: Arc::new(do_obj.tool_name.to_string()),
            },
            tool_version: do_obj.version,
            route_rule,
        }
    }
}

/// MCP 工具
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpTool {
    pub tool_name: Arc<String>,
    pub tool_key: ToolKey,
    pub tool_version: u64,
    pub spec: Arc<ToolFunctionValue>,
    pub route_rule: ToolRouteRule,
}

impl McpTool {
    pub fn to_do(&self) -> McpToolDo<'_> {
        McpToolDo {
            tool_name: Cow::Borrowed(&self.tool_name),
            namespace: Cow::Borrowed(&self.tool_key.namespace),
            group: Cow::Borrowed(&self.tool_key.group),
            version: self.tool_version,
            route_rule_json: Cow::Owned(
                serde_json::to_string(&self.route_rule).unwrap_or_else(|_| "{}".to_string()),
            ),
        }
    }
}
