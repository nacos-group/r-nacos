#[derive(Clone, Debug)]
pub struct SimpleSequence {
    cache_size: u64,
    batch_size: u64,
    last_id: u64,
}

impl SimpleSequence {
    pub fn new(last_id: u64, batch_size: u64) -> Self {
        Self {
            last_id,
            batch_size,
            cache_size: 0,
        }
    }

    pub fn set_last_id(&mut self, last_id: u64) {
        self.last_id = last_id;
        self.cache_size = 0;
    }

    pub fn set_valid_last_id(&mut self, last_id: u64) {
        if (self.last_id + self.cache_size) < last_id {
            self.last_id = last_id;
            self.cache_size = 0;
        }
    }

    pub fn next_id(&mut self) -> u64 {
        if self.cache_size == 0 {
            self.cache_size = self.batch_size;
        }
        self.cache_size -= 1;
        self.last_id += 1;
        self.last_id
    }

    pub fn next_state(&mut self) -> anyhow::Result<(u64, Option<u64>)> {
        let mut update_table_id = None;
        if self.cache_size == 0 {
            let cache_last_id = self.last_id + self.batch_size;
            update_table_id = Some(cache_last_id);
            self.cache_size = self.batch_size;
        }
        self.cache_size -= 1;
        self.last_id += 1;
        Ok((self.last_id, update_table_id))
    }

    /// 获取下个连续号段
    /// 返回值：[start,end]
    pub fn next_section(&mut self, batch_size: u64) -> anyhow::Result<(u64, u64)> {
        if batch_size == 0 {
            return Ok((0, 0));
        }
        let start = self.last_id + 1;
        let end = start + batch_size - 1;
        self.last_id = end;
        self.cache_size = 0;
        Ok((start, end))
    }

    pub fn get_end_id(&self) -> u64 {
        self.last_id + self.cache_size
    }
}

#[derive(Clone, Debug)]
pub struct CacheSequence {
    start_id: u64,
    cache_size: u64,
}

impl CacheSequence {
    pub fn new(start_id: u64, cache_size: u64) -> Self {
        Self {
            start_id,
            cache_size,
        }
    }

    pub fn next_id(&mut self) -> Option<u64> {
        if self.cache_size == 0 {
            None
        } else {
            let id = self.start_id;
            self.start_id += 1;
            self.cache_size -= 1;
            Some(id)
        }
    }
}
