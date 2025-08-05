use crate::common::byte_utils::bin_to_id;
use crate::common::constant::{
    CACHE_TREE_NAME, CONFIG_TREE_NAME, MCP_SERVER_TABLE_NAME, MCP_TOOL_SPEC_TABLE_NAME,
    NAMESPACE_TREE_NAME, SEQUENCE_TREE_NAME, SEQ_KEY_CONFIG, USER_TREE_NAME,
};
use crate::config::core::{ConfigActor, ConfigCmd, ConfigKey, ConfigValue};
use crate::config::model::{ConfigRaftCmd, ConfigValueDO};
use crate::mcp::core::McpManager;
use crate::namespace::NamespaceActor;
use crate::raft::db::table::{TableManager, TableManagerInnerReq, TableManagerReq};
use crate::raft::filestore::model::SnapshotRecordDto;
use crate::raft::filestore::raftapply::RaftApplyDataRequest;
use crate::raft::filestore::raftindex::{RaftIndexManager, RaftIndexRequest};
use crate::raft::filestore::raftsnapshot::SnapshotWriterActor;
use crate::raft::store::{ClientRequest, ClientResponse};
use crate::sequence::core::SequenceDbManager;
use actix::prelude::*;

#[derive(Clone)]
pub struct RaftDataHandler {
    pub config: Addr<ConfigActor>,
    pub table: Addr<TableManager>,
    pub namespace: Addr<NamespaceActor>,
    pub sequence_db: Addr<SequenceDbManager>,
    pub mcp_manager: Addr<McpManager>,
}

impl RaftDataHandler {
    pub async fn build_snapshot(&self, writer: Addr<SnapshotWriterActor>) -> anyhow::Result<()> {
        log::info!("RaftDataHandler|build_snapshot");
        self.sequence_db
            .send(RaftApplyDataRequest::BuildSnapshot(writer.clone()))
            .await??;
        self.config
            .send(ConfigCmd::BuildSnapshot(writer.clone()))
            .await??;
        self.table
            .send(TableManagerInnerReq::BuildSnapshot(writer.clone()))
            .await??;
        self.namespace
            .send(RaftApplyDataRequest::BuildSnapshot(writer.clone()))
            .await??;
        self.mcp_manager
            .send(RaftApplyDataRequest::BuildSnapshot(writer.clone()))
            .await??;
        Ok(())
    }

    pub async fn load_snapshot(&self, record: SnapshotRecordDto) -> anyhow::Result<()> {
        if record.tree.as_str() == CONFIG_TREE_NAME.as_str() {
            let config_key = ConfigKey::from(&String::from_utf8(record.key)? as &str);
            let value_do = ConfigValueDO::from_bytes(&record.value)?;
            self.config
                .send(ConfigCmd::SetFullValue(config_key, value_do.into()))
                .await??;
        } else if record.tree.as_str() == SEQUENCE_TREE_NAME.as_str() {
            let key = String::from_utf8_lossy(&record.key);
            let last_id = bin_to_id(&record.value);
            if &key as &str == SEQ_KEY_CONFIG {
                self.config
                    .send(ConfigCmd::InnerSetLastId(last_id))
                    .await??;
            } else {
                let req = RaftApplyDataRequest::LoadSnapshotRecord(record);
                self.sequence_db.send(req).await??;
            }
        } else if record.tree.as_str() == USER_TREE_NAME.as_str() {
            let key = record.key;
            let value = record.value;
            let req = TableManagerReq::Set {
                table_name: USER_TREE_NAME.clone(),
                key,
                value,
                last_seq_id: None,
            };
            self.table.send(req).await??;
        } else if record.tree.as_str() == CACHE_TREE_NAME.as_str() {
            let key = record.key;
            let value = record.value;
            let req = TableManagerReq::Set {
                table_name: CACHE_TREE_NAME.clone(),
                key,
                value,
                last_seq_id: None,
            };
            self.table.send(req).await??;
        } else if record.tree.as_str() == NAMESPACE_TREE_NAME.as_str() {
            let req = RaftApplyDataRequest::LoadSnapshotRecord(record);
            self.namespace.send(req).await??;
        } else if record.tree.as_str() == MCP_SERVER_TABLE_NAME.as_str()
            || record.tree.as_str() == MCP_TOOL_SPEC_TABLE_NAME.as_str()
        {
            let req = RaftApplyDataRequest::LoadSnapshotRecord(record);
            self.mcp_manager.send(req).await??;
        } else {
            log::warn!(
                "do_load_snapshot ignore data,table name:{}",
                record.tree.as_str()
            );
        }
        Ok(())
    }

    pub fn load_complete(&self) -> anyhow::Result<()> {
        log::info!("RaftDataHandler|load_complete");
        self.namespace.do_send(RaftApplyDataRequest::LoadCompleted);
        self.sequence_db
            .do_send(RaftApplyDataRequest::LoadCompleted);
        self.mcp_manager
            .do_send(RaftApplyDataRequest::LoadCompleted);
        Ok(())
    }

    /// 启动时加载日志
    pub async fn load_log(
        &self,
        req: ClientRequest,
        index_manager: &Addr<RaftIndexManager>,
    ) -> anyhow::Result<()> {
        match req {
            ClientRequest::NodeAddr { id, addr } => {
                index_manager
                    .send(RaftIndexRequest::AddNodeAddr(id, addr))
                    .await
                    .ok();
            }
            ClientRequest::Members(member) => {
                index_manager
                    .send(RaftIndexRequest::SaveMember {
                        member: member.clone(),
                        member_after_consensus: None,
                        node_addr: None,
                    })
                    .await
                    .ok();
            }
            ClientRequest::SequenceReq { req } => {
                self.sequence_db.send(req).await.ok();
            }
            ClientRequest::ConfigSet {
                key,
                value,
                config_type,
                desc,
                history_id,
                history_table_id,
                op_time,
                op_user,
            } => {
                let cmd = ConfigRaftCmd::ConfigAdd {
                    key,
                    value,
                    config_type,
                    desc,
                    history_id,
                    history_table_id,
                    op_time,
                    op_user,
                };
                self.config.send(cmd).await.ok();
            }
            ClientRequest::ConfigFullValue {
                key,
                value,
                last_seq_id: last_id,
            } => {
                let key = String::from_utf8_lossy(&key).to_string();
                let key: ConfigKey = (&key as &str).into();
                let value_do = ConfigValueDO::from_bytes(&value)?;
                let config_value: ConfigValue = value_do.into();
                let cmd = ConfigRaftCmd::SetFullValue {
                    key,
                    value: config_value,
                    last_id,
                };
                self.config.send(cmd).await.ok();
            }
            ClientRequest::ConfigRemove { key } => {
                let cmd = ConfigRaftCmd::ConfigRemove { key };
                self.config.send(cmd).await.ok();
            }
            ClientRequest::TableManagerReq(req) => {
                self.table.send(req).await.ok();
            }
            ClientRequest::NamespaceReq(req) => {
                self.namespace.send(req).await.ok();
            }
            ClientRequest::McpReq { req } => {
                self.mcp_manager.send(req).await.ok();
            }
        }
        Ok(())
    }

    /// 接收raft请求到状态机，需要返回结果到调用端
    pub async fn apply_log_to_state_machine(
        &self,
        req: ClientRequest,
        index_manager: &Addr<RaftIndexManager>,
    ) -> anyhow::Result<ClientResponse> {
        match req {
            ClientRequest::NodeAddr { id, addr } => {
                index_manager.do_send(RaftIndexRequest::AddNodeAddr(id, addr));
                Ok(ClientResponse::Success)
            }
            ClientRequest::Members(member) => {
                index_manager.do_send(RaftIndexRequest::SaveMember {
                    member: member.clone(),
                    member_after_consensus: None,
                    node_addr: None,
                });
                Ok(ClientResponse::Success)
            }
            ClientRequest::SequenceReq { req } => {
                let r = self.sequence_db.send(req).await??;
                Ok(ClientResponse::SequenceResp { resp: r })
            }
            ClientRequest::ConfigSet {
                key,
                value,
                config_type,
                desc,
                history_id,
                history_table_id,
                op_time,
                op_user,
            } => {
                let cmd = ConfigRaftCmd::ConfigAdd {
                    key,
                    value,
                    config_type,
                    desc,
                    history_id,
                    history_table_id,
                    op_time,
                    op_user,
                };
                self.config.send(cmd).await??;
                Ok(ClientResponse::Success)
            }
            ClientRequest::ConfigFullValue {
                key,
                value,
                last_seq_id: last_id,
            } => {
                let key = String::from_utf8_lossy(&key).to_string();
                let key: ConfigKey = (&key as &str).into();
                let value_do = ConfigValueDO::from_bytes(&value)?;
                let config_value: ConfigValue = value_do.into();
                let cmd = ConfigRaftCmd::SetFullValue {
                    key,
                    value: config_value,
                    last_id,
                };
                self.config.send(cmd).await??;
                Ok(ClientResponse::Success)
            }
            ClientRequest::ConfigRemove { key } => {
                let cmd = ConfigRaftCmd::ConfigRemove { key };
                self.config.send(cmd).await??;
                Ok(ClientResponse::Success)
            }
            ClientRequest::TableManagerReq(req) => {
                self.table.send(req).await??;
                Ok(ClientResponse::Success)
            }
            ClientRequest::NamespaceReq(req) => {
                self.namespace.send(req).await??;
                Ok(ClientResponse::Success)
            }
            ClientRequest::McpReq { req } => {
                let resp = self.mcp_manager.send(req).await??;
                Ok(ClientResponse::McpResp { resp })
            }
        }
    }

    pub fn do_send_log(
        &self,
        req: ClientRequest,
        index_manager: &Addr<RaftIndexManager>,
    ) -> anyhow::Result<()> {
        match req {
            ClientRequest::NodeAddr { id, addr } => {
                index_manager.do_send(RaftIndexRequest::AddNodeAddr(id, addr));
            }
            ClientRequest::Members(member) => {
                index_manager.do_send(RaftIndexRequest::SaveMember {
                    member: member.clone(),
                    member_after_consensus: None,
                    node_addr: None,
                });
            }
            ClientRequest::SequenceReq { req } => {
                self.sequence_db.do_send(req);
            }
            ClientRequest::ConfigSet {
                key,
                value,
                config_type,
                desc,
                history_id,
                history_table_id,
                op_time,
                op_user,
            } => {
                let cmd = ConfigRaftCmd::ConfigAdd {
                    key,
                    value,
                    config_type,
                    desc,
                    history_id,
                    history_table_id,
                    op_time,
                    op_user,
                };
                self.config.do_send(cmd);
            }
            ClientRequest::ConfigFullValue {
                key,
                value,
                last_seq_id: last_id,
            } => {
                let key = String::from_utf8_lossy(&key).to_string();
                let key: ConfigKey = (&key as &str).into();
                let value_do = ConfigValueDO::from_bytes(&value)?;
                let config_value: ConfigValue = value_do.into();
                let cmd = ConfigRaftCmd::SetFullValue {
                    key,
                    value: config_value,
                    last_id,
                };
                self.config.do_send(cmd);
            }
            ClientRequest::ConfigRemove { key } => {
                let cmd = ConfigRaftCmd::ConfigRemove { key };
                self.config.do_send(cmd);
            }
            ClientRequest::TableManagerReq(req) => {
                self.table.do_send(req);
            }
            ClientRequest::NamespaceReq(req) => {
                self.namespace.do_send(req);
            }
            ClientRequest::McpReq { req } => {
                self.mcp_manager.do_send(req);
            }
        };
        Ok(())
    }
}
