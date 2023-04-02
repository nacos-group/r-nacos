use std::{hash::Hash, collections::HashMap};

use inner_mem_cache::TimeoutSet;

use crate::now_millis;

pub trait NotifyEvent {
    fn on_event(&self) -> anyhow::Result<()>;
    fn merge(&mut self,other:Self) -> anyhow::Result<()>;
}

#[derive(Default)]
pub struct DelayNotify<K,T> 
where 
    K: Eq + Hash,
    T: NotifyEvent 
{
    pub(crate) timeout_set: TimeoutSet<K>,
    pub(crate) notify_map: HashMap<K,T>,
}

impl <K,T> DelayNotify<K,T> 
where 
    K: Eq + Hash + Clone,
    T: Clone + NotifyEvent
{
    pub fn new() -> Self {
        DelayNotify{
            timeout_set:Default::default(),
            notify_map: Default::default(),
        }
    }

    pub fn add_event(&mut self,delay:u64,key:K,event:T) -> anyhow::Result<()>{
        if let Some(v)=self.notify_map.get_mut(&key)  {
            v.merge(event)?;
        }
        else{
            let time_out = now_millis()+delay;
            self.timeout_set.add(time_out,key.to_owned());
            self.notify_map.insert(key, event);
        }
        Ok(())
    }

    pub fn notify(&mut self,key:&K) -> anyhow::Result<()> {
        if let Some(v) = self.notify_map.remove(key) {
            v.on_event()?;
        }
        /*
        else{
            Err(anyhow::anyhow!("not found event"))
        }
         */
        Ok(())
    }

    pub fn notify_timeout(&mut self) -> anyhow::Result<()> {
        for key in self.timeout_set.timeout(now_millis()) {
            self.notify(&key)?;
        }
        Ok(())
    }
}