use crate::common::constant::{SEQ_MCP_SERVER_ID, SEQ_MCP_SERVER_VALUE_ID, SEQ_TOOL_SPEC_VERSION};
use crate::common::pb::data_object::McpServerDo;
use crate::mcp::model::mcp::{McpServer, McpServerValue};
use crate::mcp::model::tools::{ToolKey, ToolSpec, ToolSpecVersion};
use crate::sequence::{SequenceManager, SequenceRequest, SequenceResult};
use actix::Addr;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

#[derive(Debug)]
pub struct McpImportContext {
    pub tool_spec_map: BTreeMap<ToolKey, Arc<ToolSpec>>,
    /// 工具version映射,老id -> 新id
    pub tool_version_map: HashMap<ToolKey, HashMap<u64, u64>>,
    pub last_tool_id: u64,
    pub last_tool_version_id: u64,
    pub last_server_id: u64,
    pub(crate) sequence_manager: Addr<SequenceManager>,
}

impl McpImportContext {
    pub fn new(sequence_manager: Addr<SequenceManager>) -> Self {
        Self {
            tool_spec_map: BTreeMap::new(),
            tool_version_map: HashMap::new(),
            last_tool_id: 0,
            last_tool_version_id: 0,
            last_server_id: 0,
            sequence_manager,
        }
    }

    pub async fn reset_tool_spec(
        &mut self,
        mut tool_spec: ToolSpec,
    ) -> anyhow::Result<Arc<ToolSpec>> {
        let mut version_map = HashMap::new();
        let mut versions: BTreeMap<u64, ToolSpecVersion> = BTreeMap::new();
        for (_, tool_version) in tool_spec.versions.iter() {
            let old_id = tool_version.version;
            let new_id = self.next_tool_version_id().await?;
            if old_id == tool_spec.current_version {
                tool_spec.current_version = new_id;
            }
            version_map.insert(old_id, new_id);
            let mut new_tool_version = tool_version.clone();
            new_tool_version.version = new_id;
            versions.insert(new_id, new_tool_version);
        }
        self.tool_version_map
            .insert(tool_spec.key.clone(), version_map);
        tool_spec.versions = versions;
        let r = Arc::new(tool_spec);
        self.tool_spec_map.insert(r.key.clone(), r.clone());
        Ok(r)
    }

    pub async fn build_mcp_server(&self, value_do: McpServerDo<'_>) -> anyhow::Result<McpServer> {
        let mut server = McpServer::from_do(value_do, &self.tool_spec_map);
        server.id = self.next_server_id().await?;
        let mut histories = Vec::new();
        let mut value_id_map = HashMap::new();
        let mut set_release = false;
        //reset histories
        for item in server.histories.iter() {
            let old_id = item.id;
            let mut value = item.as_ref().clone();
            self.reset_mcp_server_value(&mut value_id_map, &mut value)
                .await?;
            let arc_value = Arc::new(value);
            if old_id == server.current_value.id {
                server.release_value = arc_value.clone();
                set_release = true;
            }
            histories.push(arc_value);
        }
        server.histories = histories;
        let mut current_value = server.current_value.as_ref().clone();
        self.reset_mcp_server_value(&mut value_id_map, &mut current_value)
            .await?;
        server.current_value = Arc::new(current_value);
        if !set_release {
            let mut release_value = server.release_value.as_ref().clone();
            self.reset_mcp_server_value(&mut value_id_map, &mut release_value)
                .await?;
            server.release_value = Arc::new(release_value);
        }
        Ok(server)
    }

    pub async fn reset_mcp_server_value(
        &self,
        value_id_map: &mut HashMap<u64, u64>,
        value: &mut McpServerValue,
    ) -> anyhow::Result<()> {
        let old_id = value.id;
        value.id = self.next_server_version_id().await?;
        value_id_map.insert(old_id, value.id);
        for tool in value.tools.iter_mut() {
            let old_id = tool.tool_version;
            if let Some(new_id) = self
                .tool_version_map
                .get(&tool.tool_key)
                .and_then(|m| m.get(&old_id))
            {
                tool.tool_version = *new_id;
            }
        }
        Ok(())
    }

    pub async fn next_tool_version_id(&self) -> anyhow::Result<u64> {
        if let Ok(Ok(SequenceResult::NextId(id))) = self
            .sequence_manager
            .send(SequenceRequest::GetNextId(SEQ_TOOL_SPEC_VERSION.clone()))
            .await
        {
            Ok(id)
        } else {
            Err(anyhow::anyhow!("get sequence error"))
        }
    }

    pub async fn next_server_id(&self) -> anyhow::Result<u64> {
        if let Ok(Ok(SequenceResult::NextId(id))) = self
            .sequence_manager
            .send(SequenceRequest::GetNextId(SEQ_MCP_SERVER_ID.clone()))
            .await
        {
            Ok(id)
        } else {
            Err(anyhow::anyhow!("get sequence error"))
        }
    }

    pub async fn next_server_version_id(&self) -> anyhow::Result<u64> {
        if let Ok(Ok(SequenceResult::NextId(id))) = self
            .sequence_manager
            .send(SequenceRequest::GetNextId(SEQ_MCP_SERVER_VALUE_ID.clone()))
            .await
        {
            Ok(id)
        } else {
            Err(anyhow::anyhow!("get sequence error"))
        }
    }
}
