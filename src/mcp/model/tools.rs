use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::common::constant::EMPTY_ARC_STRING;
use crate::common::pb::data_object::{McpToolDo, McpToolSpecDo, ToolRouteRuleDo, ToolSpecVersionDo};

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

/// 工具函数定义
/// 对外提供时使用它包装
#[derive(Clone, Debug, Serialize)]
pub struct ToolFunctionWrap {
    /// type内容固定为function
    pub r#type: Arc<String>,
    pub function: ToolFunctionValue,
}

/// 工具参数定义
/// 对应function 内容
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct ToolFunctionValue {
    pub name: String,
    pub description: String,
    pub parameters: std::collections::HashMap<String, Box<JsonSchema>>,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct ToolSpecVersion {
    pub version: u64,
    pub parameters: Arc<ToolFunctionValue>,
    pub op_user: Arc<String>,
    pub update_time: i64,
    pub ref_count: i64,
}
 
impl ToolSpecVersion{
    pub fn to_do(&self) -> ToolSpecVersionDo {
        ToolSpecVersionDo {
            version: self.version,
            parameters_json: Cow::Owned(serde_json::to_string(&self.parameters).unwrap_or_default()),
            op_user: Cow::Borrowed(self.op_user.as_ref()),
            update_time: self.update_time,
            ref_count: self.ref_count,
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
            .map(|v| v.parameters.clone())
    }

    pub fn get_current_version(&self) -> Option<ToolSpecVersion> {
        self.versions.get(&self.current_version).map(|v| v.clone())
    }

    pub fn update_param(&mut self, param: ToolSpecParam) {
        // 计算新版本号(当前最大版本号+1)
        let new_version = self.versions.keys().max().unwrap_or(&0) + 1;

        // 获取当前版本的参数作为基础
        let current_params = self.get_current_value().unwrap_or_default();

        // 创建新版本，使用提供的参数或保持现有值
        let new_params = ToolFunctionValue {
            name: param
                .name
                .map(|n| n.to_string())
                .unwrap_or(current_params.name.clone()),
            description: param
                .description
                .map(|d| d.to_string())
                .unwrap_or(current_params.description.clone()),
            parameters: param
                .parameters
                .map(|p| p.parameters)
                .unwrap_or(current_params.parameters.clone()),
        };

        let spec_version = ToolSpecVersion {
            version: new_version,
            parameters: Arc::new(new_params),
            op_user: param
                .op_user
                .unwrap_or_else(|| Arc::new("system".to_string())),
            update_time: param
                .update_time
                .unwrap_or_else(|| chrono::Utc::now().timestamp_millis()),
            ref_count: 0,
        };

        // 添加新版本并更新当前版本
        self.versions.insert(new_version, spec_version);
        self.current_version = new_version;
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

    pub fn to_do(&self) -> McpToolSpecDo {
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
        for version in do_obj.versions{
            let parameters = serde_json::from_str::<ToolFunctionValue>(
                &version.parameters_json,
            )
            .unwrap_or_default();
            let spec_version = ToolSpecVersion {
                version: version.version,
                parameters: Arc::new(parameters),
                op_user: Arc::new(version.op_user.to_string()),
                update_time: version.update_time,
                ref_count: version.ref_count,
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
    pub namespace: Option<Arc<String>>,
    pub group: Option<Arc<String>>,
    pub tool_name: Option<Arc<String>>,
    pub name: Option<Arc<String>>,
    pub description: Option<Arc<String>>,
    pub parameters: Option<ToolFunctionValue>,
    pub update_time: Option<i64>,
    pub op_user: Option<Arc<String>>,
}

impl From<ToolSpecParam> for ToolSpec {
    fn from(param: ToolSpecParam) -> Self {
        let tool_key = ToolKey::new(
            param.namespace.unwrap_or_else(|| Arc::new(String::new())),
            param.group.unwrap_or_else(|| Arc::new(String::new())),
            param.tool_name.unwrap_or_else(|| Arc::new(String::new())),
        );

        let current_time = chrono::Utc::now().timestamp_millis();

        let mut versions = BTreeMap::new();
        let spec_version = ToolSpecVersion {
            version: 1,
            parameters: Arc::new(param.parameters.unwrap_or_default()),
            op_user: param
                .op_user
                .unwrap_or_else(|| Arc::new("system".to_string())),
            update_time: param.update_time.unwrap_or(current_time),
            ref_count: 0,
        };

        versions.insert(1, spec_version);

        Self {
            key: tool_key,
            current_version: 1,
            create_time: current_time,
            create_user: Arc::new("system".to_string()),
            versions,
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
        // 序列化additional_headers为JSON
        let headers_json =
            serde_json::to_string(&self.addition_headers).unwrap_or_else(|_| "{}".to_string());

        // 转换ConvertType为字符串
        let convert_type_str = match self.convert_type {
            ConvertType::None => "none",
            ConvertType::FormToJson => "formtojson",
            ConvertType::Custom => "custom",
        };

        ToolRouteRuleDo {
            protocol: Cow::Borrowed(&self.protocol),
            url: Cow::Borrowed(&self.url),
            method: Cow::Borrowed(&self.method),
            addition_headers_json: Cow::Owned(headers_json),
            convert_type: Cow::Borrowed(convert_type_str),
            service_namespace: Cow::Borrowed(&self.service_namespace),
            service_group: Cow::Borrowed(&self.service_group),
            service_name: Cow::Borrowed(&self.service_name),
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
        // 反序列化additional_headers
        let addition_headers = serde_json::from_str::<std::collections::HashMap<String, String>>(
            &do_obj.addition_headers_json,
        )
            .unwrap_or_default()
            .into_iter()
            .map(|(k, v)| (k, Arc::new(v)))
            .collect();

        // 转换ConvertType
        let convert_type = match do_obj.convert_type.as_ref() {
            "formtojson" => ConvertType::FormToJson,
            "custom" => ConvertType::Custom,
            _ => ConvertType::None,
        };

        Self {
            protocol: Arc::new(do_obj.protocol.to_string()),
            url: Arc::new(do_obj.url.to_string()),
            method: Arc::new(do_obj.method.to_string()),
            addition_headers,
            convert_type,
            service_namespace: Arc::new(do_obj.service_namespace.to_string()),
            service_group: Arc::new(do_obj.service_group.to_string()),
            service_name: Arc::new(do_obj.service_name.to_string()),
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
    pub spec: Arc<ToolFunctionValue>,
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
            spec_json: Cow::Owned(
                serde_json::to_string(&*self.spec).unwrap_or_else(|_| "{}".to_string()),
            ),
            route_rule_json: Cow::Owned(
                serde_json::to_string(&self.route_rule).unwrap_or_else(|_| "{}".to_string()),
            ),
        }
    }
}

impl<'a> From<McpToolDo<'a>> for McpTool {
    fn from(do_obj: McpToolDo<'a>) -> Self {
        // 反序列化spec
        let spec = serde_json::from_str::<ToolFunctionValue>(&do_obj.spec_json).unwrap_or_default();

        // 反序列化route_rule
        let route_rule =
            serde_json::from_str::<ToolRouteRule>(&do_obj.route_rule_json).unwrap_or_default();

        Self {
            id: do_obj.id,
            tool_name: Arc::new(do_obj.tool_name.to_string()),
            tool_key: ToolKey {
                namespace: Arc::new(do_obj.namespace.to_string()),
                group: Arc::new(do_obj.group.to_string()),
                tool_name: Arc::new(do_obj.tool_name.to_string()),
            },
            version: do_obj.version,
            spec: Arc::new(spec),
            route_rule,
        }
    }
}