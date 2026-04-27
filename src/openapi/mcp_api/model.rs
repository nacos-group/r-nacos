use crate::mcp::model::actor_model::McpToolSpecQueryParam;
use crate::mcp::model::mcp::{McpQueryParam, McpServerDto};
use crate::mcp::model::tools::ToolFunctionValue;
use crate::namespace;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenMcpServerQueryParam {
    pub page_no: Option<usize>,
    pub page_size: Option<usize>,
    pub namespace_id: Option<String>,
    pub name_filter: Option<String>,
}

impl OpenMcpServerQueryParam {
    pub fn validate(&self) -> anyhow::Result<()> {
        if let Some(page_no) = self.page_no {
            if page_no == 0 {
                return Err(anyhow::anyhow!("page_no cannot be 0"));
            }
        }

        if let Some(page_size) = self.page_size {
            if page_size == 0 {
                return Err(anyhow::anyhow!("page_size cannot be 0"));
            }
            if page_size > 1000 {
                return Err(anyhow::anyhow!("page_size cannot exceed 1000"));
            }
        }

        Ok(())
    }

    pub fn to_mcp_query_param(&self) -> McpQueryParam {
        let limit = self.page_size.unwrap_or(20);
        let offset = (self.page_no.unwrap_or(1).max(1) - 1) * limit;
        let namespace_id = if let Some(ref ns) = self.namespace_id {
            if ns.is_empty() {
                Arc::new(namespace::default_namespace("".to_string()))
            } else {
                Arc::new(ns.clone())
            }
        } else {
            Arc::new(namespace::default_namespace("".to_string()))
        };

        McpQueryParam {
            offset,
            limit,
            namespace_id: Some(namespace_id),
            name_filter: self.name_filter.as_ref().map(|s| Arc::new(s.clone())),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenToolSpecQueryParam {
    pub page_no: Option<usize>,
    pub page_size: Option<usize>,
    pub namespace_id: Option<String>,
    pub group_filter: Option<String>,
    pub tool_name_filter: Option<String>,
}

impl OpenToolSpecQueryParam {
    pub fn validate(&self) -> anyhow::Result<()> {
        if let Some(page_no) = self.page_no {
            if page_no == 0 {
                return Err(anyhow::anyhow!("page_no cannot be 0"));
            }
        }

        if let Some(page_size) = self.page_size {
            if page_size == 0 {
                return Err(anyhow::anyhow!("page_size cannot be 0"));
            }
            if page_size > 1000 {
                return Err(anyhow::anyhow!("page_size cannot exceed 1000"));
            }
        }

        Ok(())
    }

    pub fn to_tool_spec_query_param(&self) -> McpToolSpecQueryParam {
        let limit = self.page_size.unwrap_or(20);
        let offset = (self.page_no.unwrap_or(1).max(1) - 1) * limit;
        let namespace_id =
            namespace::default_namespace(self.namespace_id.clone().unwrap_or_default());

        McpToolSpecQueryParam {
            offset,
            limit,
            namespace_id: Some(namespace_id),
            group_filter: self.group_filter.clone(),
            tool_name_filter: self.tool_name_filter.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenMcpServerDto {
    pub name: Arc<String>,
    pub description: Arc<String>,
    pub namespace: Arc<String>,
    pub unique_key: Arc<String>,
    pub tools: Vec<ToolFunctionValue>,
}

impl OpenMcpServerDto {
    pub fn from_server_dto(dto: &McpServerDto) -> Self {
        let tools: Vec<ToolFunctionValue> = if let Some(ref release_value) = dto.release_value {
            release_value
                .tools
                .iter()
                .map(|t| t.spec.as_ref().clone())
                .collect()
        } else if let Some(ref current_value) = dto.current_value {
            current_value
                .tools
                .iter()
                .map(|t| t.spec.as_ref().clone())
                .collect()
        } else {
            Vec::new()
        };

        Self {
            name: dto.name.clone(),
            unique_key: dto.unique_key.clone(),
            namespace: dto.namespace.clone(),
            description: dto.description.clone(),
            tools,
        }
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenToolFunctionValue {
    pub namespace: Arc<String>,
    pub group: Arc<String>,
    pub tool: ToolFunctionValue,
}

impl OpenToolFunctionValue {
    pub fn new(namespace: Arc<String>, group: Arc<String>, tool: ToolFunctionValue) -> Self {
        Self {
            namespace,
            group,
            tool,
        }
    }
}
