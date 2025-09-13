use crate::mcp::model::actor_model::{McpToolSpecQueryParam, ToolSpecDto};
use crate::mcp::model::tools::{JsonSchema, ToolFunctionValue, ToolKey, ToolSpecParam};
use crate::namespace;
use actix_web::{HttpMessage, HttpRequest};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// ToolSpec查询请求参数
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSpecQueryRequest {
    pub page_no: Option<usize>,
    pub page_size: Option<usize>,
    pub namespace_id: Option<String>,
    pub group_filter: Option<String>,
    pub tool_name_filter: Option<String>,
}

impl ToolSpecQueryRequest {
    /// 转换为MCP查询参数
    pub fn to_query_param(&self) -> McpToolSpecQueryParam {
        let limit = self.page_size.unwrap_or(20);
        let offset = (self.page_no.unwrap_or(1) - 1) * limit;
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

    /// 验证查询参数
    pub fn validate(&self) -> anyhow::Result<()> {
        if let Some(page_no) = self.page_no {
            if page_no == 0 {
                return Err(anyhow::anyhow!("页码不能为0"));
            }
        }

        if let Some(page_size) = self.page_size {
            if page_size == 0 {
                return Err(anyhow::anyhow!("页面大小不能为0"));
            }
            if page_size > 1000 {
                return Err(anyhow::anyhow!("页面大小不能超过1000"));
            }
        }

        Ok(())
    }
}

/// ToolSpec参数结构体
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSpecParams {
    pub namespace: Arc<String>,
    pub group: Arc<String>,
    pub tool_name: Arc<String>,
    pub function: Option<ToolFunctionValue>,
    pub op_user: Option<Arc<String>>,
}

impl ToolSpecParams {
    /// 验证参数
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.group.is_empty() {
            return Err(anyhow::anyhow!("group不能为空"));
        }
        if self.tool_name.is_empty() {
            return Err(anyhow::anyhow!("tool_name不能为空"));
        }
        Ok(())
    }

    /// 验证删除参数（只需要验证key字段）
    pub fn validate_for_delete(&self) -> anyhow::Result<()> {
        if self.group.is_empty() {
            return Err(anyhow::anyhow!("group不能为空"));
        }
        if self.tool_name.is_empty() {
            return Err(anyhow::anyhow!("tool_name不能为空"));
        }

        Ok(())
    }

    /// 转换为ToolSpecParam
    pub fn to_tool_spec_param(&self, op_user: Option<Arc<String>>) -> ToolSpecParam {
        let namespace = if self.namespace.is_empty() {
            Arc::new(namespace::default_namespace("".to_string()))
        } else {
            self.namespace.clone()
        };
        ToolSpecParam {
            namespace,
            group: self.group.clone(),
            tool_name: self.tool_name.clone(),
            parameters: self.function.clone().unwrap_or_default(),
            version: 0,
            update_time: chrono::Utc::now().timestamp_millis(),
            op_user,
        }
    }

    /// 转换为ToolKey
    pub fn to_tool_key(&self) -> ToolKey {
        let namespace = if self.namespace.is_empty() {
            Arc::new(namespace::default_namespace("".to_string()))
        } else {
            self.namespace.clone()
        };
        ToolKey::new(namespace, self.group.clone(), self.tool_name.clone())
    }

    /// 从HttpRequest中提取用户信息并设置op_user
    pub fn with_user_from_request(&mut self, req: &HttpRequest) -> anyhow::Result<()> {
        // 从HttpRequest中获取用户会话信息
        if let Some(session) = req.extensions().get::<crate::common::model::UserSession>() {
            self.op_user = Some(session.username.clone());
        } else {
            // 如果没有用户会话，使用默认用户
            self.op_user = None;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSpecImportDto {
    pub group: Arc<String>,
    pub name: Arc<String>,
    pub description: Arc<String>,
    pub input_schema: Box<JsonSchema>,
}

impl From<&ToolSpecDto> for ToolSpecImportDto {
    fn from(dto: &ToolSpecDto) -> Self {
        Self {
            group: dto.group.clone(),
            name: dto.name.clone(),
            description: dto.description.clone(),
            input_schema: dto.function.input_schema.clone(),
        }
    }
}
