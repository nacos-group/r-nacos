use std::{sync::Arc, collections::HashMap};

use serde::{Deserialize, Serialize};

use actix::prelude::*;

#[derive(Clone, prost::Message, Serialize, Deserialize)]
pub struct TableDefinition {
    #[prost(string, tag = "1")]
    pub name : String,
}

impl TableDefinition {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::new();
        prost::Message::encode(self,&mut v).unwrap();
        v
    }

    pub fn from_bytes(v: &[u8]) -> anyhow::Result<Self> {
        Ok(prost::Message::decode(v)?)
    }
}

pub(crate) const TABLE_DEFINITION_TREE_NAME: &str = "tables";

pub struct TableInfo {
    pub name: Arc<String>,
    pub table_name: String,
}

impl TableInfo {
    pub fn new(name: Arc<String>) -> Self {
        let table_name = format!("t_{}",&name);
        Self {
            name,
            table_name,
        }
    }
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
        let tables = self.db.open_tree(TABLE_DEFINITION_TREE_NAME).unwrap();
        let mut iter = tables.iter();
        while let Some(Ok((_, v))) = iter.next() {
            if let Ok(definition) = TableDefinition::from_bytes(v.as_ref()) {
                let name = Arc::new(definition.name.to_owned());
                self.table_map.insert(name.clone() , TableInfo::new(name));
            }
        }
    }

    fn init_table(&mut self,name: Arc<String>) {
        let tables = self.db.open_tree(TABLE_DEFINITION_TREE_NAME).unwrap();
        let definition = TableDefinition {
            name:name.as_ref().to_owned(),
        };
        tables.insert(name.as_bytes(), definition.to_bytes()).unwrap();
    }

    pub fn drop_table(&mut self,name: &Arc<String>) {
        if let Some(table) = self.table_map.remove(name){
            self.db.drop_tree(&table.table_name).ok();
        }
    }

    pub fn insert<K>(&mut self,name: Arc<String>, key: K ,value: Vec<u8>) -> Option<sled::IVec>
        where K: AsRef<[u8]>
    {
        if let Some(table_info) = self.table_map.get(&name) {
            let table = self.db.open_tree(&table_info.table_name).unwrap();
            table.insert(key, value).unwrap()
        }
        else{
            self.init_table(name.clone());
            let table_info = TableInfo::new(name.clone());
            let table = self.db.open_tree(&table_info.table_name).unwrap();
            self.table_map.insert(name, table_info);
            table.insert(key, value).unwrap()
        }
    }

    pub fn remove<K>(&mut self,name: Arc<String>, key: K) -> Option<sled::IVec>
        where K: AsRef<[u8]>
    {
        if let Some(table_info) = self.table_map.get(&name) {
            let table = self.db.open_tree(&table_info.table_name).unwrap();
            table.remove(key).unwrap()
        }
        else{
            None
        }
    }
}

impl Actor for TableManage {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<TableManageResult>")]
pub enum TableManageAsyncCmd {
    Insert{
        table_name: Arc<String>,
        key: Vec<u8>,
        value: Vec<u8>,
    },
    Remove{
        table_name: Arc<String>,
        key: Vec<u8>,
    },
    Drop(Arc<String>),
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<TableManageResult>")]
pub enum TableManageCmd {
    Insert{
        table_name: Arc<String>,
        key: Vec<u8>,
        value: Vec<u8>,
    },
    Remove{
        table_name: Arc<String>,
        key: Vec<u8>,
    },
    Drop(Arc<String>),
}

pub enum TableManageResult {
    None,
    Value(Vec<u8>)
}


impl Handler<TableManageAsyncCmd> for TableManage {
    type Result = ResponseActFuture<Self, anyhow::Result<TableManageResult>>;

    fn handle(&mut self, msg: TableManageAsyncCmd, ctx: &mut Self::Context) -> Self::Result {
        todo!()
    }
}

impl Handler<TableManageCmd> for TableManage {
    type Result = anyhow::Result<TableManageResult>;

    fn handle(&mut self, msg: TableManageCmd, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            TableManageCmd::Insert { table_name, key, value } => {
                match self.insert(table_name, key, value) {
                    Some(v) => Ok(TableManageResult::Value(v.to_vec())),
                    None => Ok(TableManageResult::None),
                }
            },
            TableManageCmd::Remove { table_name, key } => {
                match self.remove(table_name, key) {
                    Some(v) => Ok(TableManageResult::Value(v.to_vec())),
                    None => Ok(TableManageResult::None),
                }
            },
            TableManageCmd::Drop(name) => {
                self.drop_table(&name);
                Ok(TableManageResult::None)
            },
        }
    }
}