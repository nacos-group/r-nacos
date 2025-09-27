use crate::common::byte_utils::id_to_bin;
use crate::common::constant::{MCP_SERVER_TABLE_NAME, MCP_TOOL_SPEC_TABLE_NAME};
use crate::common::pb::data_object::{McpServerDo, McpToolSpecDo};
use crate::mcp::model::actor_model::{
    McpManagerRaftReq, McpManagerRaftResult, McpManagerReq, McpManagerResult,
    McpToolSpecQueryParam, ToolSpecDto,
};
use crate::mcp::model::mcp::{
    McpQueryParam, McpServer, McpServerDto, McpServerParam, McpServerValue,
};
use crate::mcp::model::tools::{ToolKey, ToolSpec, ToolSpecParam};
use crate::mcp::utils::ToolSpecUtils;
use crate::raft::filestore::model::SnapshotRecordDto;
use crate::raft::filestore::raftapply::{RaftApplyDataRequest, RaftApplyDataResponse};
use crate::raft::filestore::raftsnapshot::{SnapshotWriterActor, SnapshotWriterRequest};
use crate::sequence::SequenceManager;
use crate::transfer::model::{TransferRecordDto, TransferWriterRequest};
use crate::transfer::writer::TransferWriterActor;
use actix::prelude::*;
use bean_factory::{bean, BeanFactory, FactoryData, Inject};
use quick_protobuf::{BytesReader, Writer};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

#[bean(inject)]
pub struct McpManager {
    pub(crate) server_map: BTreeMap<u64, Arc<McpServer>>,
    pub(crate) tool_spec_map: BTreeMap<ToolKey, Arc<ToolSpec>>,
    pub(crate) tool_spec_version_ref_map: HashMap<ToolKey, HashMap<u64, i64>>,
    pub(crate) server_key_to_id_map: HashMap<Arc<String>, u64>,
    pub(crate) sequence_manager: Option<Addr<SequenceManager>>,
}

impl McpManager {
    pub fn new() -> Self {
        McpManager {
            server_map: BTreeMap::new(),
            tool_spec_map: BTreeMap::new(),
            tool_spec_version_ref_map: HashMap::new(),
            server_key_to_id_map: HashMap::new(),
            sequence_manager: None,
        }
    }

    /// 根据服务引用，初始化计算引用计数
    fn init_tool_spec_version_ref_map(&mut self) {
        let mut server_key_to_id_map: HashMap<Arc<String>, u64> = HashMap::new();
        let mut tool_spec_version_ref_map = HashMap::new();
        for mcp_server in self.server_map.values() {
            server_key_to_id_map.insert(mcp_server.unique_key.clone(), mcp_server.id);
            Self::calculate_tool_ref(&mut tool_spec_version_ref_map, mcp_server);
        }
        self.server_key_to_id_map = server_key_to_id_map;
        /*
        // 过滤不存在版本
        for (tool_key, ref_map) in tool_spec_version_ref_map.iter_mut() {
            let mut remove_keys = vec![];
            if let Some(tool_spec) = self.tool_spec_map.get(tool_key) {
                for version in ref_map.keys() {
                    if !tool_spec.versions.contains_key(version) {
                        remove_keys.push(*version);
                    }
                }
            }
            if !remove_keys.is_empty() {
                let versions = serde_json::to_string(&remove_keys).unwrap_or_default();
                log::warn!(
                    "Some service references non-existent tool version, tool_key:{:?}, versions:{}",
                    tool_key,
                    &versions
                );
            }
            for version in remove_keys {
                ref_map.remove(&version);
            }
        }
        */
        self.tool_spec_version_ref_map = tool_spec_version_ref_map;
    }

    fn calculate_tool_ref(
        tool_spec_version_ref_map: &mut HashMap<ToolKey, HashMap<u64, i64>>,
        mcp_server: &Arc<McpServer>,
    ) {
        let mut server_value_id_set = HashSet::new();
        let server_value = &mcp_server.current_value;
        server_value_id_set.insert(server_value.id);
        ToolSpecUtils::update_server_ref_to_map(tool_spec_version_ref_map, server_value.as_ref());
        let server_value = &mcp_server.release_value;
        server_value_id_set.insert(server_value.id);
        ToolSpecUtils::update_server_ref_to_map(tool_spec_version_ref_map, server_value.as_ref());
        for server_value in &mcp_server.histories {
            if server_value_id_set.contains(&server_value.id) {
                continue;
            }
            server_value_id_set.insert(server_value.id);
            ToolSpecUtils::update_server_ref_to_map(tool_spec_version_ref_map, &server_value);
        }
    }

    fn update_server(&mut self, server_param: McpServerParam) -> anyhow::Result<Arc<McpServer>> {
        let id = server_param.id;
        if id == 0 {
            return Err(anyhow::anyhow!(
                "UpdateServer McpServerParam.id==0 is invalid!"
            ));
        }
        let v = if let Some(server) = self.server_map.get(&id) {
            let mut new_server = server.as_ref().clone();
            let ref_map = new_server.update_param(server_param, &self.tool_spec_map);
            new_server.check_valid()?;
            let value = Arc::new(new_server);
            if server.unique_key != value.unique_key {
                self.server_key_to_id_map.remove(&server.unique_key);
            }
            self.server_key_to_id_map
                .insert(value.unique_key.clone(), id);
            self.server_map.insert(id, value.clone());
            ToolSpecUtils::merge_ref_map(&mut self.tool_spec_version_ref_map, &ref_map);
            self.update_tool_spec_ref_by_diff_map(&ref_map);
            value
        } else {
            let mut server = McpServer::new(server_param.id);
            let ref_map = server.update_param(server_param, &self.tool_spec_map);
            server.check_valid()?;
            let value = Arc::new(server);
            self.server_key_to_id_map
                .insert(value.unique_key.clone(), id);
            self.server_map.insert(id, value.clone());
            ToolSpecUtils::merge_ref_map(&mut self.tool_spec_version_ref_map, &ref_map);
            self.update_tool_spec_ref_by_diff_map(&ref_map);
            value
        };
        Ok(v)
    }

    fn publish_server(&mut self, id: u64, new_value_id: u64) {
        if let Some(server) = self.server_map.get(&id) {
            let mut new_server = server.as_ref().to_owned();
            new_server.publish(new_value_id);
            let mut ref_map = HashMap::new();
            ToolSpecUtils::update_server_ref_to_map(&mut ref_map, &new_server.release_value);
            ToolSpecUtils::merge_ref_map(&mut self.tool_spec_version_ref_map, &ref_map);
            self.update_tool_spec_ref_by_diff_map(&ref_map);
            self.do_update_server(Arc::new(new_server));
        }
    }

    fn publish_history_server(&mut self, id: u64, history_id: u64) {
        if let Some(server) = self.server_map.get(&id) {
            let mut new_server = server.as_ref().to_owned();
            new_server.public_history(history_id).ok();
            self.do_update_server(Arc::new(new_server));
        }
    }

    fn remove_server(&mut self, id: u64) {
        if let Some(_v) = self.server_map.remove(&id) {
            self.init_tool_spec_version_ref_map();
            /*
            // 上面使用重建全局索引。这里应该考虑改为增量更新索引可以提升性能。
            // TODO: 目前下面的增量索引方式在部分场景还有问题，待调整验证通过后再启动增量更新索引方式。
            self.server_key_to_id_map.remove(&v.unique_key);
            let mut tool_spec_version_ref_map = HashMap::new();
            Self::calculate_tool_ref(&mut tool_spec_version_ref_map, &v);
            ToolSpecUtils::remove_ref_map(
                &mut self.tool_spec_version_ref_map,
                &tool_spec_version_ref_map,
            );
             */
        }
    }

    fn do_update_server(&mut self, server: Arc<McpServer>) {
        self.server_map.insert(server.id, server);
    }

    fn update_tool_spec(&mut self, tool_spec_param: ToolSpecParam) -> anyhow::Result<()> {
        let tool_key = tool_spec_param.build_key();
        if let Some(tool_spec) = self.tool_spec_map.get(&tool_key) {
            let mut mul_tool_spec = tool_spec.as_ref().to_owned();
            mul_tool_spec.update_param(tool_spec_param);
            self.tool_spec_map.insert(tool_key, Arc::new(mul_tool_spec));
        } else {
            let tool_key = tool_spec_param.build_key();
            let tool_spec: ToolSpec = tool_spec_param.into();
            self.tool_spec_map.insert(tool_key, Arc::new(tool_spec));
        }
        Ok(())
    }

    fn update_tool_spec_ref(
        &mut self,
        tool_key: ToolKey,
        version: u64,
        ref_count: i64,
    ) -> anyhow::Result<()> {
        if let Some(tool_spec) = self.tool_spec_map.get(&tool_key) {
            let mut mul_tool_spec = tool_spec.as_ref().to_owned();
            if let Some(tool_spec_version) = mul_tool_spec.versions.get_mut(&version) {
                tool_spec_version.ref_count = ref_count;
            }
            if ref_count > 0 || version == tool_spec.current_version {
                self.tool_spec_map.insert(tool_key, Arc::new(mul_tool_spec));
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!("not found the tool spec"))
        }
    }

    fn update_tool_spec_ref_by_diff_map(&mut self, diff_map: &HashMap<ToolKey, HashMap<u64, i64>>) {
        for (tool_key, version_map) in diff_map {
            for (version, count) in version_map {
                self.add_tool_spec_ref(tool_key.clone(), *version, *count)
                    .ok();
            }
        }
    }

    /// 增加tool_spec的引用计数
    /// add_ref_count为负数时，表示减少引用计数
    fn add_tool_spec_ref(
        &mut self,
        tool_key: ToolKey,
        version: u64,
        add_ref_count: i64,
    ) -> anyhow::Result<()> {
        let mut ref_count = add_ref_count;
        if let Some(tool_spec) = self.tool_spec_map.get(&tool_key) {
            if let Some(tool_spec_version) = tool_spec.versions.get(&version) {
                ref_count = tool_spec_version.ref_count + add_ref_count;
            }
        }
        self.update_tool_spec_ref(tool_key, version, ref_count)
    }

    fn do_update_tool_spec(&mut self, tool_spec: Arc<ToolSpec>) {
        self.tool_spec_map.insert(tool_spec.key.clone(), tool_spec);
    }

    fn remove_tool_spec(&mut self, tool_key: ToolKey) -> anyhow::Result<()> {
        if let Some(map) = self.tool_spec_version_ref_map.get(&tool_key) {
            if !map.is_empty() {
                #[cfg(feature = "debug")]
                log::warn!(
                    "tool spec is used,{:?},{}",
                    &tool_key,
                    serde_json::to_string(&map).unwrap()
                );
                return Err(anyhow::anyhow!("tool spec is used"));
            }
        }
        self.tool_spec_map.remove(&tool_key);
        Ok(())
    }

    fn set_tool_spec(&mut self, tool_spec: Arc<ToolSpec>) {
        self.tool_spec_map.insert(tool_spec.key.clone(), tool_spec);
    }

    fn set_server(&mut self, mut server: McpServer) {
        if let Some(id) = self.server_key_to_id_map.get(&server.unique_key) {
            server.id = *id;
        }
        self.server_key_to_id_map
            .insert(server.unique_key.clone(), server.id);
        self.server_map.insert(server.id, Arc::new(server));
    }

    fn query_servers(&self, query_param: &McpQueryParam) -> (usize, Vec<McpServerDto>) {
        let mut rlist = Vec::new();
        let end_index = query_param.offset + query_param.limit;
        let mut index = 0;

        for server in self.server_map.values() {
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

    fn query_server_history(
        &self,
        server_id: u64,
        offset: usize,
        limit: usize,
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> (usize, Vec<McpServerValue>) {
        let mut rlist = Vec::new();

        if let Some(server) = self.server_map.get(&server_id) {
            let mut filtered_histories: Vec<_> = server
                .histories
                .iter()
                .filter(|history| {
                    // 时间范围筛选
                    if let Some(start) = start_time {
                        if history.update_time < start {
                            return false;
                        }
                    }
                    if let Some(end) = end_time {
                        if history.update_time > end {
                            return false;
                        }
                    }
                    true
                })
                .collect();

            // 按时间倒序排列（最新的在前面）
            filtered_histories.sort_by(|a, b| b.update_time.cmp(&a.update_time));

            let total_count = filtered_histories.len();

            if offset < total_count {
                for history in filtered_histories.iter().skip(offset).take(limit) {
                    rlist.push(history.as_ref().clone());
                }
            }

            (total_count, rlist)
        } else {
            (0, rlist)
        }
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
        if let Some(ref namespace_id) = query_param.namespace_id {
            if !tool_spec.key.namespace.as_str().eq(namespace_id) {
                return false;
            }
        }
        if let Some(ref group_filter) = query_param.group_filter {
            if !group_filter.is_empty() && !tool_spec.key.group.contains(group_filter) {
                return false;
            }
        }
        if let Some(ref tool_name_filter) = query_param.tool_name_filter {
            if !tool_name_filter.is_empty() && !tool_spec.key.tool_name.contains(tool_name_filter) {
                return false;
            }
        }
        true
    }

    fn build_snapshot(&self, writer: Addr<SnapshotWriterActor>) -> anyhow::Result<()> {
        // 工具规范快照
        for (tool_key, tool_spec) in &self.tool_spec_map {
            let mut buf = Vec::new();
            {
                let mut writer = Writer::new(&mut buf);
                let value_do = tool_spec.to_do();
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

        // 服务快照
        for (key, server) in &self.server_map {
            let mut buf = Vec::new();
            {
                let mut writer = Writer::new(&mut buf);
                let value_do = server.as_ref().to_do();
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

        Ok(())
    }

    fn load_snapshot_record(&mut self, record: SnapshotRecordDto) -> anyhow::Result<()> {
        if record.tree.as_str() == MCP_SERVER_TABLE_NAME.as_str() {
            let mut reader = BytesReader::from_bytes(&record.value);
            let value_do: McpServerDo = reader.read_message(&record.value)?;
            let value = Arc::new(McpServer::from_do(value_do, &self.tool_spec_map));
            self.do_update_server(value);
        } else if record.tree.as_str() == MCP_TOOL_SPEC_TABLE_NAME.as_str() {
            let mut reader = BytesReader::from_bytes(&record.value);
            let value_do: McpToolSpecDo = reader.read_message(&record.value)?;
            let value: ToolSpec = value_do.into();
            self.do_update_tool_spec(Arc::new(value));
        }
        Ok(())
    }

    ///
    /// 迁移数据备件
    pub(crate) fn transfer_backup(&self, writer: Addr<TransferWriterActor>) -> anyhow::Result<()> {
        // 1. tool信息
        for (tool_key, tool_spec) in &self.tool_spec_map {
            let mut buf = Vec::new();
            {
                let mut writer = Writer::new(&mut buf);
                let value_do = tool_spec.to_do();
                writer.write_message(&value_do)?;
            }
            // 使用 tool_key 生成唯一键
            let key_str = format!(
                "{}:{}:{}",
                tool_key.namespace, tool_key.group, tool_key.tool_name
            );
            let record = TransferRecordDto {
                table_name: Some(MCP_TOOL_SPEC_TABLE_NAME.clone()),
                key: key_str.into_bytes(),
                value: buf,
                table_id: 0,
            };
            writer.do_send(TransferWriterRequest::AddRecord(record));
        }
        // 2. server信息
        for (key, server) in &self.server_map {
            let mut buf = Vec::new();
            {
                let mut writer = Writer::new(&mut buf);
                let value_do = server.as_ref().to_do();
                writer.write_message(&value_do)?;
            }
            let record = TransferRecordDto {
                table_name: Some(MCP_SERVER_TABLE_NAME.clone()),
                key: id_to_bin(*key),
                value: buf,
                table_id: 0,
            };
            writer.do_send(TransferWriterRequest::AddRecord(record));
        }
        Ok(())
    }

    fn import_finish(&mut self, _ctx: &mut Context<Self>) -> anyhow::Result<()> {
        self.init_tool_spec_version_ref_map();
        log::info!("McpManager import_finish");
        Ok(())
    }

    fn load_completed(&mut self, _ctx: &mut Context<Self>) -> anyhow::Result<()> {
        self.init_tool_spec_version_ref_map();
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
                let server_info = if let Some(server) = self.server_map.get(&id) {
                    Some(server.clone())
                } else {
                    None
                };
                Ok(McpManagerResult::ServerInfo(server_info))
            }
            McpManagerReq::GetServerByKey(key) => {
                let server_info = if let Some(Some(server)) = self
                    .server_key_to_id_map
                    .get(&key)
                    .map(|id| self.server_map.get(id))
                {
                    Some(server.clone())
                } else {
                    None
                };
                Ok(McpManagerResult::ServerInfo(server_info))
            }
            McpManagerReq::QueryServer(query_param) => {
                let (size, list) = self.query_servers(&query_param);
                Ok(McpManagerResult::ServerPageInfo(size, list))
            }
            McpManagerReq::QueryServerHistory(id, offset, limit, start_time, end_time) => {
                let (size, list) =
                    self.query_server_history(id, offset, limit, start_time, end_time);
                Ok(McpManagerResult::ServerHistoryPageInfo(size, list))
            }
            McpManagerReq::GetToolSpec(tool_key) => {
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

    fn handle(&mut self, msg: McpManagerRaftReq, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            McpManagerRaftReq::AddServer(server_param) => {
                let new_value_id = server_param.publish_value_id.clone();
                let value = self.update_server(server_param)?;
                if let Some(new_value_id) = new_value_id {
                    self.publish_server(value.id, new_value_id);
                }
                Ok(McpManagerRaftResult::ServerInfo(value))
            }
            McpManagerRaftReq::UpdateServer(server_param) => {
                self.update_server(server_param)?;
                Ok(McpManagerRaftResult::None)
            }
            McpManagerRaftReq::PublishCurrentServer(id, new_value_id) => {
                self.publish_server(id, new_value_id);
                Ok(McpManagerRaftResult::None)
            }
            McpManagerRaftReq::PublishHistoryServer(id, history_id) => {
                self.publish_history_server(id, history_id);
                Ok(McpManagerRaftResult::None)
            }
            McpManagerRaftReq::RemoveServer(id) => {
                self.remove_server(id);
                Ok(McpManagerRaftResult::None)
            }
            McpManagerRaftReq::UpdateToolSpec(tool_spec_param) => {
                self.update_tool_spec(tool_spec_param)?;
                Ok(McpManagerRaftResult::None)
            }
            McpManagerRaftReq::UpdateToolSpecList(tool_spec_list) => {
                for tool_spec in tool_spec_list {
                    self.update_tool_spec(tool_spec)?;
                }
                Ok(McpManagerRaftResult::None)
            }
            McpManagerRaftReq::RemoveToolSpec(tool_key) => {
                self.remove_tool_spec(tool_key)?;
                Ok(McpManagerRaftResult::None)
            }
            McpManagerRaftReq::SetToolSpec(tool_spec) => {
                self.set_tool_spec(tool_spec);
                Ok(McpManagerRaftResult::None)
            }
            McpManagerRaftReq::SetServer(server) => {
                self.set_server(server);
                Ok(McpManagerRaftResult::None)
            }
            McpManagerRaftReq::ImportFinished => {
                self.import_finish(ctx)?;
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
