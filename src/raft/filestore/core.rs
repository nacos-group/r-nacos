#![allow(clippy::suspicious_open_options)]
use crate::raft::filestore::model::{ApplyRequestDto, LogIndexInfo};
use crate::raft::filestore::raftapply::{
    StateApplyAsyncRequest, StateApplyManager, StateApplyRequest, StateApplyResponse,
};
use crate::raft::filestore::raftindex::{RaftIndexManager, RaftIndexRequest, RaftIndexResponse};
use crate::raft::filestore::raftlog::{
    RaftLogManager, RaftLogManagerAsyncRequest, RaftLogManagerRequest, RaftLogResponse,
    WriteLogResult,
};
#[cfg(feature = "debug")]
use crate::raft::filestore::raftlog::InjectErrorSceneRequest;
use crate::raft::filestore::raftsnapshot::{
    RaftSnapshotManager, RaftSnapshotRequest, RaftSnapshotResponse,
};
use crate::raft::filestore::StoreUtils;
use crate::raft::store::{ClientRequest, ClientResponse, ShutdownError};
use actix::prelude::*;
use async_raft_ext::raft::{Entry, MembershipConfig};
use async_raft_ext::storage::{CurrentSnapshotData, HardState, InitialState};
use async_raft_ext::RaftStorage;
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub fn vec_to_set(list: &Vec<u64>) -> HashSet<u64> {
    let mut set = HashSet::new();
    for item in list {
        set.insert(item.to_owned());
    }
    set
}

#[derive(Clone)]
pub struct FileStore {
    node_id: u64,
    index_manager: Addr<RaftIndexManager>,
    snapshot_manager: Addr<RaftSnapshotManager>,
    log_manager: Addr<RaftLogManager>,
    //data_store: Addr<RaftDataStore>,
    apply_manager: Addr<StateApplyManager>,
    /// raft 禁写标记，进程生命周期内单向生效（false→true），仅重启进程可恢复
    close_write: Arc<AtomicBool>,
}

impl FileStore {
    pub fn new(
        node_id: u64,
        index_manager: Addr<RaftIndexManager>,
        snapshot_manager: Addr<RaftSnapshotManager>,
        log_manager: Addr<RaftLogManager>,
        //data_store: Addr<RaftDataStore>,
        apply_manager: Addr<StateApplyManager>,
    ) -> Self {
        Self {
            node_id,
            index_manager,
            snapshot_manager,
            log_manager,
            //data_store,
            apply_manager,
            close_write: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 设置 raft 禁写（单向，进程内不可恢复）
    pub fn set_close_write(&self) {
        self.close_write.store(true, Ordering::SeqCst);
        log::warn!(
            "raft close_write enabled, node:{} raft write disabled",
            self.node_id
        );
    }

    /// 判断是否处于 raft 禁写状态
    pub fn is_close_write(&self) -> bool {
        self.close_write.load(Ordering::SeqCst)
    }

    pub async fn get_last_log_index(&self) -> anyhow::Result<LogIndexInfo> {
        match self
            .log_manager
            .send(RaftLogManagerAsyncRequest::GetLastLogIndex)
            .await??
        {
            RaftLogResponse::LastLogIndex(last_index) => Ok(last_index),
            _ => Ok(LogIndexInfo::default()),
        }
    }

    /// 触发 raft 日志写入错误注入：设置后续 `times` 个 WriteBatch 静默丢弃（仅 debug feature）
    #[cfg(feature = "debug")]
    pub async fn inject_discard_log(&self, times: u64) -> anyhow::Result<()> {
        self.log_manager
            .send(InjectErrorSceneRequest::InjectDiscardLog { times })
            .await??;
        Ok(())
    }

    /*
    pub async fn get_state_value(&self, key: String) -> anyhow::Result<Option<Arc<String>>> {
        if let RaftDataStoreResponse::Value(v) = self
            .data_store
            .send(crate::common::raftstore::RaftDataStoreRequest::Get(
                Arc::new(key),
            ))
            .await??
        {
            Ok(v)
        } else {
            Err(anyhow::anyhow!("get_state_value error"))
        }
    }
     */

    pub async fn get_target_addr(&self, id: u64) -> anyhow::Result<Arc<String>> {
        if let RaftIndexResponse::TargetAddr(Some(addr)) = self
            .index_manager
            .send(RaftIndexRequest::GetTargetAddr(id))
            .await??
        {
            Ok(addr)
        } else {
            Err(anyhow::anyhow!("get_target_addr error"))
        }
    }

    /// Map the oneshot-delivered write result to the RaftStorage return value.
    fn write_log_result_to_result(r: WriteLogResult) -> anyhow::Result<()> {
        match r {
            WriteLogResult::Success | WriteLogResult::Ignore => Ok(()),
            WriteLogResult::LogIndexEqualError => {
                Err(anyhow::anyhow!("log write index not equal"))
            }
        }
    }
}

#[async_trait]
impl RaftStorage<ClientRequest, ClientResponse> for FileStore {
    type Snapshot = tokio::fs::File;
    type ShutdownError = ShutdownError;

    async fn get_membership_config(&self) -> anyhow::Result<MembershipConfig> {
        match self
            .index_manager
            .send(RaftIndexRequest::LoadMember)
            .await??
        {
            RaftIndexResponse::MemberShip {
                member,
                member_after_consensus,
                node_addrs: _node_addrs,
            } => {
                let membership = MembershipConfig {
                    members: vec_to_set(&member),
                    members_after_consensus: if member_after_consensus.is_empty() {
                        None
                    } else {
                        Some(vec_to_set(&member_after_consensus))
                    },
                };
                Ok(membership)
            }
            _ => {
                log::warn!("get_membership_config init");
                Ok(MembershipConfig::new_initial(self.node_id))
            }
        }
    }

    async fn get_initial_state(&self) -> anyhow::Result<InitialState> {
        let last_log_index = self.get_last_log_index().await.unwrap_or_default();
        match self
            .index_manager
            .send(RaftIndexRequest::LoadIndexInfo)
            .await??
        {
            RaftIndexResponse::RaftIndexInfo {
                raft_index,
                last_applied_log,
            } => {
                let voted_for = if raft_index.voted_for > 0 {
                    Some(raft_index.voted_for)
                } else {
                    None
                };
                let membership = MembershipConfig {
                    members: vec_to_set(&raft_index.member),
                    members_after_consensus: if raft_index.member_after_consensus.is_empty() {
                        None
                    } else {
                        Some(vec_to_set(&raft_index.member_after_consensus))
                    },
                };
                log::info!(
                    "get_initial_state from index_manager,{:?},{:?}",
                    &last_log_index,
                    &membership
                );
                Ok(InitialState {
                    last_log_index: last_log_index.index,
                    last_log_term: last_log_index.term,
                    last_applied_log,
                    hard_state: HardState {
                        current_term: raft_index.current_term,
                        voted_for,
                    },
                    membership,
                })
            }
            _ => Ok(InitialState::new_initial(self.node_id)),
        }
    }

    async fn save_hard_state(&self, hs: &HardState) -> anyhow::Result<()> {
        let req = RaftIndexRequest::SaveHardState {
            current_term: hs.current_term,
            voted_for: hs.voted_for.unwrap_or_default(),
        };
        self.index_manager.send(req).await??;
        Ok(())
    }

    async fn get_log_entries(
        &self,
        start: u64,
        stop: u64,
    ) -> anyhow::Result<Vec<Entry<ClientRequest>>> {
        match self
            .log_manager
            .send(RaftLogManagerAsyncRequest::Query { start, end: stop })
            .await??
        {
            //RaftLogResponse::QueryEntryResult(records) => Ok(records),
            RaftLogResponse::QueryResult(records) => {
                let mut entrys = Vec::with_capacity(records.len());
                for item in records.into_iter() {
                    match StoreUtils::log_record_to_entry(item) {
                        Ok(entry) => entrys.push(entry),
                        Err(e) => {
                            log::error!("get_log_entries error:{}", e);
                        }
                    }
                }
                Ok(entrys)
            }
            _ => Err(anyhow::anyhow!("RaftLogResponse result is error")),
        }
    }

    async fn delete_logs_from(&self, start: u64, _stop: Option<u64>) -> anyhow::Result<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.log_manager
            .send(RaftLogManagerRequest::StripLogToIndex {
                end_index: start,
                sender: tx,
            })
            .await??;
        let r = rx.await??;
        Self::write_log_result_to_result(r)
    }

    async fn append_entry_to_log(&self, entry: &Entry<ClientRequest>) -> anyhow::Result<()> {
        let record = StoreUtils::entry_to_record(entry)?;
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.log_manager
            .send(RaftLogManagerRequest::Write { record, sender: tx })
            .await??;
        let r = rx.await??;
        Self::write_log_result_to_result(r)
    }

    async fn replicate_to_log(&self, entries: &[Entry<ClientRequest>]) -> anyhow::Result<()> {
        let mut records = Vec::with_capacity(entries.len());
        for item in entries {
            let record = StoreUtils::entry_to_record(item)?;
            records.push(record);
        }
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.log_manager
            .send(RaftLogManagerRequest::WriteBatch {
                records,
                sender: tx,
            })
            .await??;
        let r = rx.await??;
        Self::write_log_result_to_result(r)
    }

    async fn apply_entry_to_state_machine(
        &self,
        index: &u64,
        data: &ClientRequest,
    ) -> anyhow::Result<ClientResponse> {
        if self.is_close_write() {
            return Err(anyhow::anyhow!(
                "raft is in close-write state, reject apply_entry"
            ));
        }
        match self
            .apply_manager
            .send(StateApplyAsyncRequest::ApplyRequest(ApplyRequestDto::new(
                *index,
                data.clone(),
            )))
            .await??
        {
            StateApplyResponse::RaftResponse(resp) => Ok(resp),
            _ => Err(anyhow::anyhow!(
                "apply_entry_to_state_machine response is error"
            )),
        }
    }

    async fn replicate_to_state_machine(
        &self,
        entries: &[(&u64, &ClientRequest)],
    ) -> anyhow::Result<()> {
        if self.is_close_write() {
            return Err(anyhow::anyhow!(
                "raft is in close-write state, reject replicate"
            ));
        }
        let mut list = Vec::with_capacity(entries.len());
        for (index, data) in entries {
            list.push(ApplyRequestDto::new((*index).to_owned(), (*data).clone()))
        }
        self.apply_manager
            .send(StateApplyRequest::ApplyBatchRequest(list))
            .await??;
        Ok(())
    }

    async fn do_log_compaction(&self) -> anyhow::Result<CurrentSnapshotData<Self::Snapshot>> {
        match self
            .apply_manager
            .send(StateApplyAsyncRequest::BuildSnapshot)
            .await??
        {
            StateApplyResponse::Snapshot(header, path, snapshot_id) => {
                let membership_config = MembershipConfig {
                    members: vec_to_set(&header.member),
                    members_after_consensus: if header.member_after_consensus.is_empty() {
                        None
                    } else {
                        Some(vec_to_set(&header.member_after_consensus))
                    },
                };
                let file = tokio::fs::OpenOptions::new()
                    .read(true)
                    .open(path.as_str())
                    .await?;
                let snapshot = CurrentSnapshotData {
                    term: header.last_term,
                    index: header.last_index,
                    membership: membership_config.clone(),
                    snapshot: Box::new(file),
                };
                if snapshot_id == 0 {
                    //使用原镜像
                    return Ok(snapshot);
                }
                let entry = Entry::new_snapshot_pointer(
                    header.last_index,
                    header.last_term,
                    snapshot_id.to_string(),
                    membership_config,
                );
                let record = StoreUtils::entry_to_record(&entry)?;
                self.log_manager
                    .send(RaftLogManagerRequest::BuildSnapshotPointerLog(record))
                    .await??;
                Ok(snapshot)
            }
            _ => Err(anyhow::anyhow!("StateApplyResponse result is error")),
        }
    }

    async fn create_snapshot(&self) -> anyhow::Result<(String, Box<Self::Snapshot>)> {
        log::info!("filestore create_snapshot");
        match self
            .snapshot_manager
            .send(RaftSnapshotRequest::NewSnapshotForLoad)
            .await??
        {
            RaftSnapshotResponse::NewSnapshotForLoad(path, snapshot_id) => {
                let file = tokio::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(path.as_str())
                    .await?;
                Ok((snapshot_id.to_string(), Box::new(file)))
            }
            _ => todo!(),
        }
    }

    async fn finalize_snapshot_installation(
        &self,
        index: u64,
        term: u64,
        delete_through: Option<u64>,
        id: String,
        snapshot: Box<Self::Snapshot>,
    ) -> anyhow::Result<()> {
        let snapshot_id: u64 = id.parse()?;
        self.snapshot_manager
            .send(RaftSnapshotRequest::InstallSnapshot {
                end_index: index,
                snapshot_id,
            })
            .await??;
        self.apply_manager
            .send(StateApplyRequest::ApplySnapshot { snapshot })
            .await??;
        //清除废弃日志
        let split_off_index = if let Some(v) = delete_through {
            v + 1
        } else {
            0
        };
        self.log_manager
            .send(RaftLogManagerRequest::SplitOff(split_off_index))
            .await??;
        //add new_snapshot_pointer
        let membership_config = self.get_membership_config().await?;
        let entry = Entry::new_snapshot_pointer(index, term, id, membership_config);
        let record = StoreUtils::entry_to_record(&entry)?;
        self.log_manager
            .send(RaftLogManagerRequest::InstallSnapshotPointerLog(record))
            .await??;
        Ok(())
    }

    async fn get_current_snapshot(
        &self,
    ) -> anyhow::Result<Option<CurrentSnapshotData<Self::Snapshot>>> {
        if let RaftSnapshotResponse::LastSnapshot(Some(path), Some(header)) = self
            .snapshot_manager
            .send(RaftSnapshotRequest::GetLastSnapshot)
            .await??
        {
            let membership_config = MembershipConfig {
                members: vec_to_set(&header.member),
                members_after_consensus: if header.member_after_consensus.is_empty() {
                    None
                } else {
                    Some(vec_to_set(&header.member_after_consensus))
                },
            };
            let file = tokio::fs::OpenOptions::new()
                .read(true)
                .open(path.as_str())
                .await?;
            let snapshot = CurrentSnapshotData {
                term: header.last_term,
                index: header.last_index,
                membership: membership_config,
                snapshot: Box::new(file),
            };
            Ok(Some(snapshot))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix::dev::channel;

    /// 构造仅用于测试禁写标记逻辑的 `FileStore`，所有 actor 地址为不连接的 channel
    fn new_test_store() -> FileStore {
        FileStore {
            node_id: 1,
            index_manager: Addr::new(channel::channel::<RaftIndexManager>(1).0),
            snapshot_manager: Addr::new(channel::channel::<RaftSnapshotManager>(1).0),
            log_manager: Addr::new(channel::channel::<RaftLogManager>(1).0),
            apply_manager: Addr::new(channel::channel::<StateApplyManager>(1).0),
            close_write: Arc::new(AtomicBool::new(false)),
        }
    }

    #[test]
    fn close_write_default_is_false() {
        let store = new_test_store();
        assert!(!store.is_close_write());
    }

    #[test]
    fn close_write_is_one_way_irreversible() {
        let store = new_test_store();
        assert!(!store.is_close_write());
        store.set_close_write();
        assert!(store.is_close_write());
        assert!(store.is_close_write());
        store.set_close_write();
        assert!(store.is_close_write());
    }

    use async_raft_ext::raft::EntryPayload;
    use crate::raft::filestore::raftlog::RaftLogManager;
    use std::time::Duration;

    /// 构造一条 Blank 负载的日志 Entry（负载内容对落盘验证无影响）
    fn one_entry(index: u64, term: u64) -> Entry<ClientRequest> {
        Entry {
            term,
            index,
            payload: EntryPayload::Blank,
        }
    }

    /// 启动单节点 FileStore 完整 actor 链，日志目录指向临时目录。
    /// 返回的 TempDir 必须在测试期间存活，否则文件被回收。
    fn new_persistent_store() -> (FileStore, tempfile::TempDir) {
        let temp = tempfile::tempdir().unwrap();
        let base_path = Arc::new(temp.path().to_string_lossy().into_owned());
        let index_manager = RaftIndexManager::new(base_path.clone()).start();
        let log_manager = RaftLogManager::new(base_path.clone(), Some(index_manager.clone())).start();
        let snapshot_manager =
            RaftSnapshotManager::new(base_path.clone(), Some(index_manager.clone())).start();
        let apply_manager = StateApplyManager::new().start();
        let store = FileStore::new(
            1,
            index_manager,
            snapshot_manager,
            log_manager,
            apply_manager,
        );
        (store, temp)
    }

    /// 用例1：单条写入返回成功且已真正落盘（`append_entry_to_log` / `Write`）。
    #[actix::test]
    async fn append_entry_to_log_persists_before_return() {
        let (store, _temp) = new_persistent_store();
        store.append_entry_to_log(&one_entry(1, 1)).await.unwrap();
        let got = store.get_log_entries(1, 2).await.unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].index, 1);
        assert_eq!(got[0].term, 1);
    }

    /// 用例2：批量写入返回成功且整批已落盘（`replicate_to_log` / `WriteBatch`）。
    #[actix::test]
    async fn replicate_to_log_persists_whole_batch_before_return() {
        let (store, _temp) = new_persistent_store();
        let entries: Vec<Entry<ClientRequest>> = (1..=5).map(|i| one_entry(i, 1)).collect();
        store.replicate_to_log(&entries).await.unwrap();
        let got = store.get_log_entries(1, 6).await.unwrap();
        assert_eq!(got.len(), 5);
        for (i, e) in got.iter().enumerate() {
            assert_eq!(e.index, (i as u64) + 1);
        }
    }

    /// 用例3：空批量写入按 Ignore 返回成功，不产生新记录、不空等待。
    #[actix::test]
    async fn replicate_to_log_empty_returns_success_without_new_record() {
        let (store, _temp) = new_persistent_store();
        store.append_entry_to_log(&one_entry(1, 1)).await.unwrap();
        let before = store.get_last_log_index().await.unwrap().index;
        // 空数据应返回 Ok(())（Ignore → Ok(())）且有限时间内完成
        let r = tokio::time::timeout(Duration::from_secs(3), store.replicate_to_log(&[]))
            .await
            .expect("replicate_to_log empty should not block");
        r.unwrap();
        let after = store.get_last_log_index().await.unwrap().index;
        assert_eq!(after, before);
    }

    /// 用例4：删除日志返回成功且被删 index 不可读、保留段仍可读（`delete_logs_from` / `StripLogToIndex`）。
    #[actix::test]
    async fn delete_logs_from_makes_removed_range_unreadable() {
        let (store, _temp) = new_persistent_store();
        let entries: Vec<Entry<ClientRequest>> = (1..=5).map(|i| one_entry(i, 1)).collect();
        store.replicate_to_log(&entries).await.unwrap();
        // 删除 index >= 3 的日志
        store.delete_logs_from(3, None).await.unwrap();
        let removed = store.get_log_entries(3, 6).await.unwrap();
        assert!(removed.is_empty(), "deleted range should be unreadable");
        let kept = store.get_log_entries(1, 3).await.unwrap();
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].index, 1);
        assert_eq!(kept[1].index, 2);
    }

    /// 用例6：debug 注入丢弃 WriteBatch 时，调用方不空等待；注入耗尽后恢复正常写入。
    #[cfg(feature = "debug")]
    #[actix::test]
    async fn discard_injected_writebatch_does_not_block_caller() {
        let (store, _temp) = new_persistent_store();
        // 注入：后续 1 个 WriteBatch 静默丢弃
        store.inject_discard_log(1).await.unwrap();
        let entries: Vec<Entry<ClientRequest>> = (1..=3).map(|i| one_entry(i, 1)).collect();
        // 被丢弃但调用方在有限时间内返回 Ok(())（discard 约定回传 Success）
        let r = tokio::time::timeout(
            Duration::from_secs(3),
            store.replicate_to_log(&entries),
        )
        .await
        .expect("discarded WriteBatch should not block caller");
        r.unwrap();
        // 注入次数耗尽后恢复正常写入路径，仍回传真实结果
        store.replicate_to_log(&entries).await.unwrap();
        let got = store.get_log_entries(1, 4).await.unwrap();
        assert_eq!(got.len(), 3);
        assert_eq!(got[0].index, 1);
        assert_eq!(got[2].index, 3);
    }
}
