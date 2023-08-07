use std::sync::Arc;

use prost::Message;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Message, Deserialize, Serialize)]
pub struct IdValue {
    #[prost(int64, tag = "1")]
    pub value: i64,
}

impl IdValue {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let mut v = Vec::new();
        self.encode(&mut v)?;
        Ok(v)
    }
}

fn load_table_last_id(db: &sled::Db, table_key: &str) -> anyhow::Result<u64> {
    let tree = db.open_tree(TABLE_SEQUENCE_TREE_NAME)?;
    if let Some(value) = tree.get(table_key)? {
        let v: IdValue = IdValue::decode(value.as_ref())?;
        Ok(v.value as u64)
    } else {
        Ok(0)
    }
}

fn save_table_last_id(db: &sled::Db, table_key: &str, id: u64) -> anyhow::Result<()> {
    let tree = db.open_tree(TABLE_SEQUENCE_TREE_NAME)?;
    let value = IdValue { value: id as i64 };
    tree.insert(table_key, value.to_bytes()?)?;
    Ok(())
}

fn compare_generate_batch_id(
    db: &sled::Db,
    table_key: &str,
    batch_size: u64,
) -> anyhow::Result<u64> {
    let tree = db.open_tree(TABLE_SEQUENCE_TREE_NAME)?;
    let mut last_id = batch_size;
    loop {
        let old_bytes = tree.get(table_key)?;
        let new_bytes = if let Some(value) = &old_bytes {
            let mut v: IdValue = IdValue::decode(value.as_ref())?;
            v.value += batch_size as i64;
            last_id = v.value as u64;
            Some(v.to_bytes()?)
        } else {
            Some(
                IdValue {
                    value: batch_size as i64,
                }
                .to_bytes()?,
            )
        };
        if let Ok(Ok(_)) = tree.compare_and_swap(table_key, old_bytes, new_bytes) {
            break;
        }
    }
    Ok(last_id)
}

pub(crate) const TABLE_SEQUENCE_TREE_NAME: &str = "table_sequence";

///
/// 通用的sled表id sequence
#[derive(Debug)]
pub struct TableSequence {
    last_id: u64,
    batch_size: u64,
    cache_size: u64,
    db: Arc<sled::Db>,
    table_seq_key: String,
}

impl TableSequence {
    pub fn new(db: Arc<sled::Db>, table_seq_key: String, batch_size: u64) -> Self {
        let last_id = load_table_last_id(&db, &table_seq_key).unwrap_or_default();
        assert!(batch_size > 0, "batch size is greater than 0");
        Self {
            cache_size: 0,
            last_id,
            batch_size,
            db,
            table_seq_key,
        }
    }

    /// 一个表id只能用同一个对象,在单线程或加锁使用
    /// 缓存批次少一次查询，并且没有锁；
    /// 单线程使用更快
    pub fn next_id(&mut self) -> anyhow::Result<u64> {
        if self.cache_size == 0 {
            let cache_last_id = self.last_id + self.batch_size;
            save_table_last_id(&self.db, &self.table_seq_key, cache_last_id)?;
            self.cache_size = self.batch_size;
        }
        self.cache_size -= 1;
        self.last_id += 1;
        Ok(self.last_id)
    }

    pub(crate) fn set_table_last_id(&mut self, id: u64) -> anyhow::Result<()> {
        if (self.last_id + self.batch_size) < id {
            save_table_last_id(&self.db, &self.table_seq_key, id)?;
        }
        Ok(())
    }

    pub fn next_state(&mut self) -> anyhow::Result<(u64, Option<u64>)> {
        let mut update_table_id = None;
        if self.cache_size == 0 {
            let cache_last_id = self.last_id + self.batch_size;
            update_table_id = Some(cache_last_id.to_owned());
            save_table_last_id(&self.db, &self.table_seq_key, cache_last_id)?;
            self.cache_size = self.batch_size;
        }
        self.cache_size -= 1;
        self.last_id += 1;
        Ok((self.last_id, update_table_id))
    }

    /// 一个表id支持用于多个对象,支持多线程；
    /// 单线程使用每个批次多查一次；
    /// 多线程使用缓存批次会有锁;
    pub fn next_id_by_compare(&mut self) -> anyhow::Result<u64> {
        if self.cache_size == 0 {
            let cache_last_id =
                compare_generate_batch_id(&self.db, &self.table_seq_key, self.batch_size)?;
            self.last_id = cache_last_id - self.batch_size;
            self.cache_size = self.batch_size;
        }
        self.cache_size -= 1;
        self.last_id += 1;
        Ok(self.last_id)
    }
}
