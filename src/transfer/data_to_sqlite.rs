use crate::common::constant::{CONFIG_TREE_NAME, NAMESPACE_TREE_NAME, USER_TREE_NAME};
use crate::config::core::{ConfigKey, ConfigValue};
use crate::config::model::ConfigValueDO;
use crate::namespace::model::NamespaceDO;
use crate::transfer::model::TransferRecordRef;
use crate::transfer::reader::{reader_transfer_record, TransferFileReader};
use crate::transfer::sqlite::dao::config::{ConfigDO, ConfigDao};
use crate::transfer::sqlite::dao::config_history::{ConfigHistoryDO, ConfigHistoryDao};
use crate::transfer::sqlite::dao::tenant::{TenantDO, TenantDao};
use crate::transfer::sqlite::dao::user::{UserDO, UserDao};
use crate::user::model::UserDo;
use rusqlite::Connection;

#[derive(Debug, Default)]
pub struct TableSeq {
    pub(crate) config_id: i64,
    pub(crate) config_history_id: i64,
    pub(crate) tenant_id: i64,
    pub(crate) user_id: i64,
}

impl TableSeq {
    pub fn next_config_id(&mut self) -> i64 {
        self.config_id += 1;
        self.config_id
    }

    pub fn next_config_history_id(&mut self) -> i64 {
        self.config_history_id += 1;
        self.config_history_id
    }

    pub fn next_tenant_id(&mut self) -> i64 {
        self.tenant_id += 1;
        self.tenant_id
    }
    pub fn next_user_id(&mut self) -> i64 {
        self.user_id += 1;
        self.user_id
    }
}

pub async fn data_to_sqlite(data_file: &str, db_path: &str) -> anyhow::Result<()> {
    let mut file_reader = TransferFileReader::new(data_file).await?;
    let conn = open_init_db(db_path)?;
    let mut config_count = 0;
    let mut tenant_count = 0;
    let mut user_count = 0;
    let mut ignore = 0;
    let mut table_seq = TableSeq::default();
    let config_dao = ConfigDao::new(&conn);
    let config_history_dao = ConfigHistoryDao::new(&conn);
    let user_dao = UserDao::new(&conn);
    let tenant_dao = TenantDao::new(&conn);
    while let Ok(Some(vec)) = file_reader.read_record_vec().await {
        let record = reader_transfer_record(&vec, &file_reader.header)?;
        if record.table_name.as_str() == CONFIG_TREE_NAME.as_str() {
            config_count += 1;
            insert_config(&mut table_seq, &config_dao, &config_history_dao, record)?;
        } else if record.table_name.as_str() == NAMESPACE_TREE_NAME.as_str() {
            tenant_count += 1;
            insert_namespace(&mut table_seq, &tenant_dao, record)?
        } else if record.table_name.as_str() == USER_TREE_NAME.as_str() {
            user_count += 1;
            insert_user(&mut table_seq, &user_dao, record)?
        } else {
            ignore += 1;
        }
    }
    log::info!(
        "transfer to sqlite db finished,config count:{},tenant count:{},use count:{},ignore count:{}",
        config_count,
        tenant_count,
        user_count,
        ignore
    );
    Ok(())
}

fn insert_config(
    table_seq: &mut TableSeq,
    config_dao: &ConfigDao<'_>,
    config_history_dao: &ConfigHistoryDao<'_>,
    record: TransferRecordRef<'_>,
) -> anyhow::Result<()> {
    let value_do = ConfigValueDO::from_bytes(&record.value)?;
    let key = String::from_utf8_lossy(&record.key).to_string();
    let key: ConfigKey = (&key as &str).into();
    let config_value: ConfigValue = value_do.into();
    let config_do = ConfigDO {
        id: Some(table_seq.next_config_id()),
        data_id: Some(key.data_id.clone()),
        group_id: Some(key.group.clone()),
        tenant_id: Some(key.tenant.clone()),
        content: Some(config_value.content.clone()),
        config_type: config_value.config_type.clone(),
        config_desc: config_value.desc,
        last_time: Some(config_value.last_modified),
    };
    config_dao.insert(&config_do)?;
    for history_item in config_value.histories {
        let history = ConfigHistoryDO {
            id: Some(table_seq.next_config_history_id()),
            data_id: Some(key.data_id.clone()),
            group_id: Some(key.group.clone()),
            tenant_id: Some(key.tenant.clone()),
            content: Some(history_item.content.clone()),
            config_type: None,
            config_desc: None,
            op_user: history_item.op_user.clone(),
            last_time: Some(history_item.modified_time),
        };
        config_history_dao.insert(&history)?;
    }
    Ok(())
}

fn insert_namespace(
    table_seq: &mut TableSeq,
    tenant_dao: &TenantDao<'_>,
    record: TransferRecordRef<'_>,
) -> anyhow::Result<()> {
    let value_do: NamespaceDO = NamespaceDO::from_bytes(&record.value)?;
    let tenant_do = TenantDO {
        id: Some(table_seq.next_tenant_id()),
        tenant_id: value_do.namespace_id,
        tenant_name: value_do.namespace_name,
        tenant_desc: None,
        create_flag: None,
    };
    tenant_dao.insert(&tenant_do)?;
    Ok(())
}

fn insert_user(
    table_seq: &mut TableSeq,
    user_dao: &UserDao<'_>,
    record: TransferRecordRef<'_>,
) -> anyhow::Result<()> {
    let value_do = UserDo::from_bytes(&record.value)?;
    let user_do = UserDO {
        id: Some(table_seq.next_user_id()),
        username: Some(value_do.username),
        nickname: Some(value_do.nickname),
        password_hash: value_do.password_hash,
        gmt_create: Some(value_do.gmt_create as i64),
        gmt_modified: Some(value_do.gmt_modified as i64),
        enabled: Some(value_do.enable.to_string()),
        roles: Some(serde_json::to_string(&value_do.roles)?),
        extend_info: Some(serde_json::to_string(&value_do.extend_info)?),
    };
    user_dao.insert(&user_do)?;
    Ok(())
}

pub fn open_init_db(db_path: &str) -> anyhow::Result<Connection> {
    let conn = Connection::open(db_path)?;
    let create_table_sql = r"
create table if not exists tb_config(
    id integer primary key autoincrement,
    data_id text,
    group_id text,
    tenant_id text,
    content text,
    config_type text,
    config_desc text,
    last_time long
);
create index if not exists tb_config_key_idx on tb_config(data_id,group_id,tenant_id);

create table if not exists tb_config_history(
    id integer primary key autoincrement,
    data_id text,
    group_id text,
    tenant_id text,
    content text,
    config_type text,
    config_desc text,
    op_user text,
    last_time long
);
create index if not exists tb_config_history_key_idx on tb_config_history(data_id,group_id,tenant_id);

create table if not exists tb_tenant(
    id integer primary key autoincrement,
    tenant_id text,
    tenant_name text,
    tenant_desc text,
    create_flag integer
);

create table if not exists tb_user(
    id integer primary key autoincrement,
    username text,
    nickname text,
    password_hash text,
    gmt_create integer,
    gmt_modified integer,
    enabled text,
    roles text,
    extend_info text
);
        ";
    conn.execute_batch(create_table_sql)?;
    Ok(conn)
}
