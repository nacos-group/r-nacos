use crate::cache::actor_model::{CacheManagerRaftReq, CacheSetParam};
use crate::now_second_i32;
use crate::raft::cache::model::{CacheItemDo, CacheKey, CacheValue};
use std::convert::TryInto;

pub struct AdaptationUtils;

impl AdaptationUtils {
    pub fn build_key_from_old(k: &[u8]) -> anyhow::Result<crate::cache::model::CacheKey> {
        CacheKey::from_db_key_ref(k)
    }

    pub fn build_value_from_old(v: &[u8]) -> anyhow::Result<crate::cache::model::CacheValue> {
        let cache_item = CacheItemDo::from_bytes(&v)?;
        let old_value: CacheValue = cache_item.try_into()?;
        let value: crate::cache::model::CacheValue = old_value.into();
        Ok(value)
    }

    pub fn build_value_with_ttl_from_old(
        v: &[u8],
    ) -> anyhow::Result<(crate::cache::model::CacheValue, i32)> {
        let now = now_second_i32();
        let cache_item = CacheItemDo::from_bytes(&v)?;
        let ttl = cache_item.timeout - now;
        let old_value: CacheValue = cache_item.try_into()?;
        let value: crate::cache::model::CacheValue = old_value.into();
        Ok((value, ttl))
    }

    pub fn build_raft_req_from_old(k: &[u8], v: &[u8]) -> anyhow::Result<CacheManagerRaftReq> {
        let key = Self::build_key_from_old(k)?;
        let (value, ttl) = Self::build_value_with_ttl_from_old(v)?;
        if ttl < 0 {
            Ok(CacheManagerRaftReq::Remove(key))
        } else {
            let set_info = CacheSetParam::new_with_ttl(key, value, ttl);
            Ok(CacheManagerRaftReq::Set(set_info))
        }
    }
}
