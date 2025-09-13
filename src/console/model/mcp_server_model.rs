use crate::common::string_utils::StringUtils;
use crate::mcp::model::mcp::{McpQueryParam, McpServerParam, McpServerValue};
use crate::mcp::model::tools::{McpSimpleTool, ToolRouteRule};
use crate::namespace;
use actix_web::{HttpMessage, HttpRequest};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// McpServer查询请求参数
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerQueryRequest {
    pub page_no: Option<usize>,
    pub page_size: Option<usize>,
    pub namespace_id: Option<String>,
    pub name_filter: Option<String>,
}

impl McpServerQueryRequest {
    /// 转换为MCP查询参数
    pub fn to_mcp_query_param(&self) -> McpQueryParam {
        let limit = self.page_size.unwrap_or(20);
        let offset = (self.page_no.unwrap_or(1) - 1) * limit;
        let namespace_id = if StringUtils::is_option_empty(&self.namespace_id) {
            Arc::new(namespace::default_namespace("".to_string()))
        } else {
            Arc::new(self.namespace_id.clone().unwrap())
        };

        McpQueryParam {
            offset,
            limit,
            namespace_id: Some(namespace_id),
            name_filter: self.name_filter.as_ref().map(|s| Arc::new(s.clone())),
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

/// McpServer历史版本查询请求参数
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerHistoryQueryRequest {
    pub id: u64,
    pub page_no: Option<usize>,
    pub page_size: Option<usize>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
}

impl McpServerHistoryQueryRequest {
    /// 验证历史查询参数
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.id == 0 {
            return Err(anyhow::anyhow!("McpServer ID不能为0"));
        }

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

        if let (Some(start_time), Some(end_time)) = (self.start_time, self.end_time) {
            if start_time >= end_time {
                return Err(anyhow::anyhow!("开始时间必须小于结束时间"));
            }
        }

        Ok(())
    }

    /// 获取分页参数
    pub fn get_pagination(&self) -> (usize, usize) {
        let limit = self.page_size.unwrap_or(20);
        let offset = (self.page_no.unwrap_or(1) - 1) * limit;
        (offset, limit)
    }
}

/// McpServer历史版本发布参数
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerHistoryPublishParams {
    pub id: u64,
    pub history_value_id: u64,
}

impl McpServerHistoryPublishParams {
    /// 验证历史版本发布参数
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.id == 0 {
            return Err(anyhow::anyhow!("McpServer ID不能为0"));
        }
        if self.history_value_id == 0 {
            return Err(anyhow::anyhow!("历史版本ID不能为0"));
        }
        Ok(())
    }
}

/// 简化的工具参数，用于控制台接口
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSimpleToolParams {
    pub id: Option<u64>,
    pub tool_name: Arc<String>,
    pub namespace: Arc<String>,
    pub group: Arc<String>,
    pub tool_version: Option<u64>,
    pub route_rule: Option<ToolRouteRule>,
}

impl McpSimpleToolParams {
    /// 验证工具参数
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.tool_name.is_empty() {
            return Err(anyhow::anyhow!("工具名称不能为空"));
        }
        if self.group.is_empty() {
            return Err(anyhow::anyhow!("工具组不能为空"));
        }
        Ok(())
    }

    /// 转换为McpSimpleTool
    pub fn to_mcp_simple_tool(&self) -> McpSimpleTool {
        let namespace = if self.namespace.is_empty() {
            Arc::new(namespace::default_namespace("".to_string()))
        } else {
            self.namespace.clone()
        };

        McpSimpleTool {
            tool_name: self.tool_name.clone(),
            tool_key: crate::mcp::model::tools::ToolKey::new(
                namespace,
                self.group.clone(),
                self.tool_name.clone(),
            ),
            tool_version: self.tool_version.unwrap_or(1),
            route_rule: self.route_rule.clone().unwrap_or_default(),
        }
    }
}

/// McpServer参数结构体
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerParams {
    pub id: Option<u64>,
    pub unique_key: Option<String>,
    pub namespace: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub auth_keys: Option<Vec<String>>,
    pub tools: Option<Vec<McpSimpleToolParams>>,
}

impl McpServerParams {
    /// 验证参数（用于新增操作）
    pub fn validate(&self) -> anyhow::Result<()> {
        if let Some(ref name) = self.name {
            if name.is_empty() {
                return Err(anyhow::anyhow!("McpServer名称不能为空"));
            }
        } else {
            return Err(anyhow::anyhow!("McpServer名称不能为空"));
        }

        if let Some(ref auth_keys) = self.auth_keys {
            if auth_keys.is_empty() {
                return Err(anyhow::anyhow!("认证密钥不能为空"));
            }
            for key in auth_keys {
                if key.is_empty() {
                    return Err(anyhow::anyhow!("认证密钥不能包含空值"));
                }
            }
        } else {
            return Err(anyhow::anyhow!("认证密钥不能为空"));
        }

        if let Some(ref tools) = self.tools {
            for tool in tools {
                tool.validate()?;
            }
        }

        Ok(())
    }

    /// 验证更新参数（用于更新操作）
    pub fn validate_for_update(&self) -> anyhow::Result<()> {
        if self.id.is_none() || self.id == Some(0) {
            return Err(anyhow::anyhow!("McpServer ID不能为空"));
        }

        if let Some(ref name) = self.name {
            if name.is_empty() {
                return Err(anyhow::anyhow!("McpServer名称不能为空"));
            }
        }

        if let Some(ref auth_keys) = self.auth_keys {
            if auth_keys.is_empty() {
                return Err(anyhow::anyhow!("认证密钥不能为空"));
            }
            for key in auth_keys {
                if key.is_empty() {
                    return Err(anyhow::anyhow!("认证密钥不能包含空值"));
                }
            }
        }

        if let Some(ref tools) = self.tools {
            for tool in tools {
                tool.validate()?;
            }
        }

        Ok(())
    }

    /// 验证删除参数（只需要验证ID字段）
    pub fn validate_for_delete(&self) -> anyhow::Result<()> {
        if self.id.is_none() || self.id == Some(0) {
            return Err(anyhow::anyhow!("McpServer ID不能为空"));
        }
        Ok(())
    }

    /// 转换为McpServerParam
    pub fn to_mcp_server_param(&self, op_user: Option<Arc<String>>) -> McpServerParam {
        let namespace = if let Some(ref ns) = self.namespace {
            if ns.is_empty() {
                Some(Arc::new(namespace::default_namespace("".to_string())))
            } else {
                Some(Arc::new(ns.clone()))
            }
        } else {
            Some(Arc::new(namespace::default_namespace("".to_string())))
        };

        let unique_key = if let Some(ref unique_key) = self.unique_key {
            if unique_key.is_empty() {
                None
            } else {
                Some(Arc::new(unique_key.clone()))
            }
        } else {
            None
        };

        McpServerParam {
            id: self.id.unwrap_or(0),
            unique_key,
            value_id: 0, // 新版本ID由系统生成
            namespace,
            name: self.name.as_ref().map(|s| Arc::new(s.clone())),
            description: self.description.as_ref().map(|s| Arc::new(s.clone())),
            auth_keys: self
                .auth_keys
                .as_ref()
                .map(|keys| keys.iter().map(|k| Arc::new(k.clone())).collect()),
            tools: self
                .tools
                .as_ref()
                .map(|tools| tools.iter().map(|t| t.to_mcp_simple_tool()).collect())
                .unwrap_or_default(),
            op_user: op_user.unwrap_or_else(|| Arc::new("".to_string())),
            update_time: chrono::Utc::now().timestamp_millis(),
            publish_value_id: None,
            token: None, // 控制台接口不使用token
        }
    }

    /// 从HttpRequest中提取用户信息并设置op_user
    pub fn with_user_from_request(&mut self, req: &HttpRequest) -> anyhow::Result<Arc<String>> {
        // 从HttpRequest中获取用户会话信息
        if let Some(session) = req.extensions().get::<crate::common::model::UserSession>() {
            Ok(session.username.clone())
        } else {
            // 如果没有用户会话，使用默认用户
            Ok(Arc::new("system".to_string()))
        }
    }
}

/// McpServer值DTO，用于历史版本展示
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerValueDto {
    pub id: u64,
    pub description: Arc<String>,
    pub tools: Vec<crate::mcp::model::tools::McpTool>,
    pub op_user: Arc<String>,
    pub update_time: i64,
}

impl McpServerValueDto {
    /// 从McpServerValue创建DTO
    pub fn from_value(value: &McpServerValue) -> Self {
        Self {
            id: value.id,
            description: value.description.clone(),
            tools: value.tools.clone(),
            op_user: value.op_user.clone(),
            update_time: value.update_time,
        }
    }
}
