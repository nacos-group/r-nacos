use crate::common::byte_utils::id_to_bin;
use crate::common::constant::{MCP_SERVER_TABLE_NAME, MCP_TOOL_SPEC_TABLE_NAME};
use crate::common::pb::data_object::{McpServerDo, McpToolSpecDo};
use crate::mcp::model::actor_model::{
    McpManagerRaftReq, McpManagerRaftResult, McpManagerReq, McpManagerResult,
    McpToolSpecQueryParam, ToolSpecDto,
};
use crate::mcp::model::mcp::{
    McpQueryParam, McpServer, McpServerDto, McpServerParam, McpServerWrap,
};
use crate::raft::filestore::model::SnapshotRecordDto;
use crate::raft::filestore::raftapply::{RaftApplyDataRequest, RaftApplyDataResponse};
use crate::raft::filestore::raftsnapshot::{SnapshotWriterActor, SnapshotWriterRequest};
use crate::sequence::SequenceManager;
use actix::prelude::*;
use bean_factory::{bean, BeanFactory, FactoryData, Inject};
use quick_protobuf::{BytesReader, Writer};
use std::collections::BTreeMap;
use std::sync::Arc;
use crate::mcp::model::tools::{ToolKey, ToolSpec, ToolSpecParam};

#[bean(inject)]
pub struct McpManager {
    pub(crate) server_map: BTreeMap<u64, McpServerWrap>,
    pub(crate) tool_spec_map: BTreeMap<ToolKey, Arc<ToolSpec>>,
    sequence_manager: Option<Addr<SequenceManager>>,
}

impl McpManager {
    pub fn new() -> Self {
        McpManager {
            server_map: BTreeMap::new(),
            tool_spec_map: BTreeMap::new(),
            sequence_manager: None,
        }
    }

    fn create_server(&mut self, server_param: McpServerParam) -> anyhow::Result<Arc<McpServer>> {
        let id = server_param.id.unwrap_or_default();
        if id == 0 {
            return Err(anyhow::anyhow!(
                "CreateServer McpServerParam.id==0 is invalid!"
            ));
        }
        if self.server_map.contains_key(&id) {
            return Err(anyhow::anyhow!("Server already exists"));
        }

        let server_info: McpServer = server_param.into();
        server_info.check_valid()?;

        //let now = now_millis();
        // TODO 在实际实现中，需要设置 create_time 和 last_modified_millis

        let value = Arc::new(server_info);
        self.server_map
            .insert(value.id, McpServerWrap::new(value.clone()));

        Ok(value)
    }

    fn update_server(&mut self, server_param: McpServerParam) -> anyhow::Result<()> {
        let id = server_param.id.unwrap_or_default();
        if id == 0 {
            return Err(anyhow::anyhow!(
                "UpdateServer McpServerParam.id==0 is invalid!"
            ));
        }

        if let Some(server_wrap) = self.server_map.get_mut(&id) {
            let server = &server_wrap.server;
            let mut new_server = server.as_ref().clone();
            new_server.update_param(server_param);
            new_server.check_valid()?;

            let value = Arc::new(new_server);
            server_wrap.server = value.clone();
        } else {
            return Err(anyhow::anyhow!("UpdateServer, Nonexistent server"));
        }
        Ok(())
    }

    fn remove_server(&mut self, id: u64) {
        self.server_map.remove(&id);
    }

    fn do_update_server(&mut self, server: Arc<McpServer>) {
        if let Some(server_wrap) = self.server_map.get_mut(&server.id) {
            server_wrap.server = server;
        } else {
            self.server_map
                .insert(server.id, McpServerWrap::new(server.clone()));
        }
    }

    fn create_tool_spec(
        &mut self,
        tool_spec_param: ToolSpecParam,
    ) -> anyhow::Result<Arc<ToolSpec>> {
        let tool_key = ToolKey {
            namespace: tool_spec_param.namespace.clone().unwrap_or_default(),
            group: tool_spec_param.group.clone().unwrap_or_default(),
            tool_name: tool_spec_param.tool_name.clone().unwrap_or_default(),
        };

        if self.tool_spec_map.contains_key(&tool_key) {
            return Err(anyhow::anyhow!("ToolSpec already exists"));
        }

        let tool_spec: ToolSpec = tool_spec_param.into();
        tool_spec.check_valid()?;

        //let now = now_millis();
        // TODO 在实际实现中，需要设置 create_time 和 last_modified_millis

        let value = Arc::new(tool_spec);
        self.tool_spec_map.insert(tool_key, value.clone());

        Ok(value)
    }

    fn update_tool_spec(&mut self, tool_spec_param: ToolSpecParam) -> anyhow::Result<()> {
        let tool_key = ToolKey {
            namespace: tool_spec_param.namespace.clone().unwrap_or_default(),
            group: tool_spec_param.group.clone().unwrap_or_default(),
            tool_name: tool_spec_param.tool_name.clone().unwrap_or_default(),
        };

        if let Some(tool_spec) = self.tool_spec_map.get_mut(&tool_key) {
            let mut new_tool_spec = tool_spec.as_ref().clone();
            new_tool_spec.update_param(tool_spec_param);
            new_tool_spec.check_valid()?;

            let value = Arc::new(new_tool_spec);
            *tool_spec = value;
        } else {
            return Err(anyhow::anyhow!("UpdateToolSpec, Nonexistent tool spec"));
        }
        Ok(())
    }

    fn remove_tool_spec(&mut self, namespace: String, group: String, tool_name: String) {
        let tool_key = ToolKey {
            namespace: Arc::new(namespace),
            group: Arc::new(group),
            tool_name: Arc::new(tool_name),
        };
        self.tool_spec_map.remove(&tool_key);
    }

    fn do_update_tool_spec(&mut self, tool_spec: Arc<ToolSpec>) {
        self.tool_spec_map.insert(tool_spec.key.clone(), tool_spec);
    }

    fn query_servers(&self, query_param: &McpQueryParam) -> (usize, Vec<McpServerDto>) {
        let mut rlist = Vec::new();
        let end_index = query_param.offset + query_param.limit;
        let mut index = 0;

        for server_wrap in self.server_map.values() {
            let server = &server_wrap.server;
            if query_param.match_namespace(&server.namespace)
                && query_param.match_name(&server.name)
            {
                if index >= query_param.offset && index < end_index {
                    rlist.push(McpServerDto::new_from(server));
                }
                index += 1;
            }
        }

        (index, rlist)
    }

    fn query_tool_specs(&self, query_param: &McpToolSpecQueryParam) -> (usize, Vec<ToolSpecDto>) {
        let mut rlist = Vec::new();
        let end_index = query_param.offset + query_param.limit;
        let mut index = 0;

        for tool_spec in self.tool_spec_map.values() {
            if Self::match_tool_spec_filter(query_param, tool_spec) {
                if index >= query_param.offset && index < end_index {
                    rlist.push(ToolSpecDto::new_from(tool_spec));
                }
                index += 1;
            }
        }

        (index, rlist)
    }

    fn match_tool_spec_filter(query_param: &McpToolSpecQueryParam, tool_spec: &ToolSpec) -> bool {
        if let Some(ref namespace_filter) = query_param.namespace_filter {
            if !tool_spec.key.namespace.contains(namespace_filter) {
                return false;
            }
        }
        if let Some(ref group_filter) = query_param.group_filter {
            if !tool_spec.key.group.contains(group_filter) {
                return false;
            }
        }
        if let Some(ref tool_name_filter) = query_param.tool_name_filter {
            if !tool_spec.key.tool_name.contains(tool_name_filter) {
                return false;
            }
        }
        true
    }

    fn build_snapshot(&self, writer: Addr<SnapshotWriterActor>) -> anyhow::Result<()> {
        // 服务器快照
        for (key, server_wrap) in &self.server_map {
            let mut buf = Vec::new();
            {
                let mut writer = Writer::new(&mut buf);
                let value_do = server_wrap.server.as_ref().to_do();
                writer.write_message(&value_do)?;
            }
            let record = SnapshotRecordDto {
                tree: MCP_SERVER_TABLE_NAME.clone(),
                key: id_to_bin(*key),
                value: buf,
                op_type: 0,
            };
            writer.do_send(SnapshotWriterRequest::Record(record));
        }

        // 工具规范快照
        for (tool_key, tool_spec) in &self.tool_spec_map {
            let mut buf = Vec::new();
            {
                let mut writer = Writer::new(&mut buf);
                let value_do = tool_spec.as_ref().to_do();
                writer.write_message(&value_do)?;
            }
            // 使用 tool_key 生成唯一键
            let key_str = format!(
                "{}:{}:{}",
                tool_key.namespace, tool_key.group, tool_key.tool_name
            );
            let record = SnapshotRecordDto {
                tree: MCP_TOOL_SPEC_TABLE_NAME.clone(),
                key: key_str.into_bytes(),
                value: buf,
                op_type: 0,
            };
            writer.do_send(SnapshotWriterRequest::Record(record));
        }

        Ok(())
    }

    fn load_snapshot_record(&mut self, record: SnapshotRecordDto) -> anyhow::Result<()> {
        if record.tree.as_str() == MCP_SERVER_TABLE_NAME.as_str() {
            let mut reader = BytesReader::from_bytes(&record.value);
            let value_do: McpServerDo = reader.read_message(&record.value)?;
            let value = Arc::new(value_do.into());
            self.do_update_server(value);
        } else if record.tree.as_str() == MCP_TOOL_SPEC_TABLE_NAME.as_str() {
            let mut reader = BytesReader::from_bytes(&record.value);
            let value_do: McpToolSpecDo = reader.read_message(&record.value)?;
            let value: Arc<ToolSpec> = Arc::new(value_do.into());
            self.do_update_tool_spec(value);
        }
        Ok(())
    }

    fn load_completed(&mut self, _ctx: &mut Context<Self>) -> anyhow::Result<()> {
        log::info!("McpManager load snapshot completed");
        Ok(())
    }
}

impl Actor for McpManager {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("McpManager started");
    }
}

impl Inject for McpManager {
    type Context = Context<Self>;

    fn inject(
        &mut self,
        factory_data: FactoryData,
        _factory: BeanFactory,
        _ctx: &mut Self::Context,
    ) {
        self.sequence_manager = factory_data.get_actor();
    }
}

impl Handler<McpManagerReq> for McpManager {
    type Result = anyhow::Result<McpManagerResult>;

    fn handle(&mut self, msg: McpManagerReq, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            McpManagerReq::GetServer(id) => {
                let server_info = if let Some(server_wrap) = self.server_map.get(&id) {
                    Some(server_wrap.server.clone())
                } else {
                    None
                };
                Ok(McpManagerResult::ServerInfo(server_info))
            }
            McpManagerReq::QueryServer(query_param) => {
                let (size, list) = self.query_servers(&query_param);
                Ok(McpManagerResult::ServerPageInfo(size, list))
            }
            McpManagerReq::GetToolSpec(namespace, group, tool_name) => {
                let tool_key = ToolKey {
                    namespace: Arc::new(namespace),
                    group: Arc::new(group),
                    tool_name: Arc::new(tool_name),
                };
                let tool_spec = self.tool_spec_map.get(&tool_key).cloned();
                Ok(McpManagerResult::ToolSpecInfo(tool_spec))
            }
            McpManagerReq::QueryToolSpec(query_param) => {
                let (size, list) = self.query_tool_specs(&query_param);
                Ok(McpManagerResult::ToolSpecPageInfo(size, list))
            }
        }
    }
}

impl Handler<McpManagerRaftReq> for McpManager {
    type Result = anyhow::Result<McpManagerRaftResult>;

    fn handle(&mut self, msg: McpManagerRaftReq, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            McpManagerRaftReq::AddServer(server_param) => {
                let value = self.create_server(server_param)?;
                Ok(McpManagerRaftResult::ServerInfo(value))
            }
            McpManagerRaftReq::UpdateServer(server_param) => {
                self.update_server(server_param)?;
                Ok(McpManagerRaftResult::None)
            }
            McpManagerRaftReq::RemoveServer(id) => {
                self.remove_server(id);
                Ok(McpManagerRaftResult::None)
            }
            McpManagerRaftReq::AddToolSpec(tool_spec_param) => {
                let value = self.create_tool_spec(tool_spec_param)?;
                Ok(McpManagerRaftResult::ToolSpecInfo(value))
            }
            McpManagerRaftReq::UpdateToolSpec(tool_spec_param) => {
                self.update_tool_spec(tool_spec_param)?;
                Ok(McpManagerRaftResult::None)
            }
            McpManagerRaftReq::RemoveToolSpec(namespace, group, tool_name) => {
                self.remove_tool_spec(namespace, group, tool_name);
                Ok(McpManagerRaftResult::None)
            }
        }
    }
}

impl Handler<RaftApplyDataRequest> for McpManager {
    type Result = anyhow::Result<RaftApplyDataResponse>;

    fn handle(&mut self, msg: RaftApplyDataRequest, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            RaftApplyDataRequest::BuildSnapshot(writer) => {
                self.build_snapshot(writer)?;
            }
            RaftApplyDataRequest::LoadSnapshotRecord(record) => {
                self.load_snapshot_record(record)?;
            }
            RaftApplyDataRequest::LoadCompleted => {
                self.load_completed(ctx)?;
            }
        }
        Ok(RaftApplyDataResponse::None)
    }
}
