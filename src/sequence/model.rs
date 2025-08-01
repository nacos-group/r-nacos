use actix::Message;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeqGroup {
    range_a: SeqRange,
    range_b: SeqRange,
    use_a: bool,
    step: u64,
    // 是否正在添加,用户防止多次申请缓存
    next_adding: bool,
}

impl SeqGroup {
    pub fn new(step: u64) -> Self {
        Self {
            range_a: SeqRange::new(0, 0),
            range_b: SeqRange::new(0, 0),
            step,
            use_a: false,
            next_adding: false,
        }
    }
    pub fn next_id(&mut self) -> Option<u64> {
        let v = self.do_next_id();
        if v.is_none() {
            self.switch_state();
            self.do_next_id()
        } else {
            v
        }
    }

    pub fn apply_range(&mut self, start: u64, len: u64) {
        if self.use_a && !self.range_a.has_next() || !self.use_a && self.range_b.has_next() {
            self.range_a.renew(start, len);
        } else {
            self.range_b.renew(start, len);
        }
    }

    pub fn mark_apply(&mut self) {
        self.next_adding = true;
    }

    pub fn clear_apply_mark(&mut self) {
        self.next_adding = false;
    }

    pub fn need_apply(&self) -> bool {
        if self.next_adding {
            return false;
        }
        if !self.range_a.has_next() || !self.range_b.has_next() {
            true
        } else {
            false
        }
    }

    /*
    fn current_range(&self) -> &SeqRange {
        if self.use_a {
            &self.range_a
        }
        else{
            &self.range_b
        }
    }

    fn next_range(&self) -> &SeqRange {
        if self.use_a {
            &self.range_b
        }
        else{
            &self.range_a
        }
    }
     */

    fn do_next_id(&mut self) -> Option<u64> {
        if self.use_a {
            self.range_a.next_id()
        } else {
            self.range_b.next_id()
        }
    }

    fn switch_state(&mut self) {
        self.use_a = !self.use_a;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SeqRange {
    pub(crate) start: u64,
    pub(crate) len: u64,
    current_index: u64,
}

impl SeqRange {
    pub fn new(start: u64, len: u64) -> Self {
        Self {
            start,
            len,
            current_index: 0,
        }
    }

    pub fn renew(&mut self, start: u64, len: u64) {
        self.start = start;
        self.len = len;
        self.current_index = 0;
    }

    pub fn next_id(&mut self) -> Option<u64> {
        if self.current_index >= self.len {
            return None;
        }
        let v = self.start + self.current_index;
        self.current_index += 1;
        Some(v)
    }

    pub fn has_next(&self) -> bool {
        self.current_index < self.len
    }
}

#[derive(Message, Clone, Debug, Serialize, Deserialize)]
#[rtype(result = "anyhow::Result<SequenceRaftResult>")]
pub enum SequenceRaftReq {
    NextId(Arc<String>),
    NextRange(Arc<String>, u64),
    SetId(Arc<String>, u64),
    RemoveId(Arc<String>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SequenceRaftResult {
    NextId(u64),
    NextRange { start: u64, len: u64 },
    None,
}
