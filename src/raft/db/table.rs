use std::{sync::Arc, collections::HashMap};

use crate::common::sled_utils::TableSequence;

use serde::{Deserialize, Serialize};

#[derive(Clone, prost::Message, Serialize, Deserialize)]
pub struct TableDefinition {
    #[prost(string, tag = "1")]
    pub name : String,
    #[prost(uint32, tag = "2")]
    pub sequence_step: u32, // 0: None,  1: seq , _:step seq
}

impl TableDefinition {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::new();
        //self.encode(&mut v).unwrap();
        prost::Message::encode(self,&mut v).unwrap();
        v
    }
}


pub struct TableInfo {
    pub name: Arc<String>,
    pub seq: Option<TableSequence>,
}

pub struct TableManage {
    pub db: Arc<sled::Db>,
    pub table_map: HashMap<Arc<String>,TableInfo>,
}

impl TableManage {
    pub fn new(db: Arc<sled::Db>) -> Self {
        let mut s = Self {
            db,
            table_map: Default::default()
        };
        s.load_tables();
        s
    }

    /// load table info from db
    fn load_tables(&mut self) {
        let tables = self.db.open_tree("tables").unwrap();
        let mut iter = tables.iter();
        while let Some(Ok((_, v))) = iter.next() {
            let definition :TableDefinition = prost::Message::decode(v.as_ref()).unwrap();
            let name = Arc::new(definition.name.to_owned());
            self.table_map.insert(name.clone() , self.build_table_info(name,definition.sequence_step));
        }
    }

    fn build_table_info(&self,name: Arc<String> ,seq_step: u32) -> TableInfo {
        let seq = match seq_step {
            0 => None,
            _ => Some(TableSequence::new(self.db.clone(), format!("seq_{}",name.as_ref()), seq_step as u64))
        };
        TableInfo {
            name,
            seq,
        }
    }

    pub(crate) fn init_table(&mut self,name: Arc<String>,seq_step: u32) {
        if !self.table_map.contains_key(&name) {
            self.table_map.insert(name.clone(),self.build_table_info(name.clone(), seq_step));
            //let tables = self.db.open_tree("tables").unwrap();
            //tables.insert(name, value)
        }
    }

    //pub fn insert()
}