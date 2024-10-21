use crate::common::actor_utils::create_actor_at_thread;
use crate::common::constant::{
    CACHE_TREE_NAME, CONFIG_TREE_NAME, EMPTY_ARC_STRING, EMPTY_STR, NAMESPACE_TREE_NAME,
    SEQUENCE_TREE_NAME, USER_TREE_NAME,
};
use crate::config::core::{ConfigKey, ConfigValue};
use crate::config::model::ConfigValueDO;
use crate::config::ConfigUtils;
use crate::namespace::model::{NamespaceDO, FROM_USER_VALUE};
use crate::now_millis_i64;
use crate::transfer::model::{
    TransferRecordDto, TransferWriterAsyncRequest, TransferWriterRequest,
};
use crate::transfer::sqlite::dao::config::{ConfigDO, ConfigDao, ConfigParam};
use crate::transfer::sqlite::dao::config_history::{
    ConfigHistoryDO, ConfigHistoryDao, ConfigHistoryParam,
};
use crate::transfer::sqlite::dao::tenant::{TenantDao, TenantParam};
use crate::transfer::sqlite::dao::user::{UserDao, UserParam};
use crate::transfer::sqlite::TableSeq;
use crate::transfer::writer::TransferWriterActor;
use crate::user::model::UserDo;
use actix::Addr;
use rusqlite::Connection;

pub async fn sqlite_to_data(db_path: &str, data_file: &str) -> anyhow::Result<()> {
    let conn = Connection::open(db_path)?;
    let writer_actor = init_writer_actor(data_file);
    let mut table_seq = TableSeq::default();
    apply_config(&conn, &mut table_seq, &writer_actor)?;
    apply_tenant(&conn, &writer_actor)?;
    apply_user(&conn, &writer_actor)?;
    writer_actor
        .send(TransferWriterAsyncRequest::Flush)
        .await
        .ok();
    Ok(())
}

fn init_writer_actor(data_file: &str) -> Addr<TransferWriterActor> {
    let writer_actor = create_actor_at_thread(TransferWriterActor::new(data_file.into(), 0));
    writer_actor.do_send(TransferWriterRequest::AddTableNameMap(
        CONFIG_TREE_NAME.clone(),
    ));
    writer_actor.do_send(TransferWriterRequest::AddTableNameMap(
        SEQUENCE_TREE_NAME.clone(),
    ));
    writer_actor.do_send(TransferWriterRequest::AddTableNameMap(
        NAMESPACE_TREE_NAME.clone(),
    ));
    writer_actor.do_send(TransferWriterRequest::AddTableNameMap(
        USER_TREE_NAME.clone(),
    ));
    writer_actor.do_send(TransferWriterRequest::AddTableNameMap(
        CACHE_TREE_NAME.clone(),
    ));
    writer_actor.do_send(TransferWriterRequest::InitHeader);
    writer_actor
}

fn apply_config(
    conn: &Connection,
    table_seq: &mut TableSeq,
    writer_actor: &Addr<TransferWriterActor>,
) -> anyhow::Result<()> {
    let config_dao = ConfigDao::new(conn);
    let config_history_dao = ConfigHistoryDao::new(conn);
    let config_param = ConfigParam::default();
    let mut count = 0;
    //query all config
    for item in config_dao.query(&config_param)? {
        let mut tenant_id = item.tenant_id.clone().unwrap_or_default();
        if ConfigUtils::is_default_tenant(&tenant_id) {
            tenant_id = EMPTY_ARC_STRING.clone();
        }
        let key = ConfigKey::new_by_arc(
            item.data_id.clone().unwrap_or_default(),
            item.group_id.clone().unwrap_or_default(),
            tenant_id,
        );
        let history_query_param = ConfigHistoryParam {
            id: None,
            data_id: item.data_id.clone(),
            group_id: item.group_id.clone(),
            tenant_id: item.tenant_id.clone(),
            limit: None,
            offset: None,
        };
        let histories = config_history_dao
            .query(&history_query_param)
            .unwrap_or_default();
        let record = build_config_record(table_seq, key, item, histories)?;
        count += 1;
        writer_actor.do_send(TransferWriterRequest::AddRecord(record));
    }
    log::info!("transfer config count:{count}");
    Ok(())
}

fn build_config_record(
    table_seq: &mut TableSeq,
    key: ConfigKey,
    config_do: ConfigDO,
    histories: Vec<ConfigHistoryDO>,
) -> anyhow::Result<TransferRecordDto> {
    let current_current = config_do.content.unwrap_or_default();
    let mut config_value = ConfigValue::new(current_current.clone());
    let mut last_content = None;
    for item in histories {
        if let Some(content) = item.content {
            let op_time = item.last_time.unwrap_or(now_millis_i64());
            last_content = Some(content.clone());
            config_value.update_value(
                content,
                table_seq.next_config_id() as u64,
                op_time,
                None,
                item.op_user,
            );
        }
    }
    let need_pull_current = if let Some(last_content) = &last_content {
        last_content != &current_current
    } else {
        true
    };
    if need_pull_current {
        let op_time = config_do.last_time.unwrap_or(now_millis_i64());
        config_value.update_value(
            current_current,
            table_seq.next_config_id() as u64,
            op_time,
            None,
            None,
        );
    }
    let value_do: ConfigValueDO = config_value.into();
    let record = TransferRecordDto {
        table_name: Some(CONFIG_TREE_NAME.clone()),
        key: key.build_key().as_bytes().to_vec(),
        value: value_do.to_bytes()?,
        table_id: 0,
    };
    Ok(record)
}

fn apply_tenant(conn: &Connection, writer_actor: &Addr<TransferWriterActor>) -> anyhow::Result<()> {
    let mut count = 0;
    let tenant_dao = TenantDao::new(conn);
    let param = TenantParam::default();
    for item in tenant_dao.query(&param)? {
        let key = if let Some(v) = &item.tenant_id {
            v.as_bytes().to_vec()
        } else {
            EMPTY_STR.as_bytes().to_vec()
        };
        let value_do = NamespaceDO {
            namespace_id: item.tenant_id,
            namespace_name: item.tenant_name,
            r#type: Some(FROM_USER_VALUE.to_string()),
        };
        let record = TransferRecordDto {
            table_name: Some(NAMESPACE_TREE_NAME.clone()),
            key,
            value: value_do.to_bytes()?,
            table_id: 0,
        };
        writer_actor.do_send(TransferWriterRequest::AddRecord(record));
        count += 1;
    }
    log::info!("transfer tenant count:{count}");
    Ok(())
}

fn apply_user(conn: &Connection, writer_actor: &Addr<TransferWriterActor>) -> anyhow::Result<()> {
    let mut count = 0;
    let user_dao = UserDao::new(conn);
    let param = UserParam::default();
    for item in user_dao.query(&param)? {
        let key = if let Some(v) = &item.username {
            v.as_bytes().to_vec()
        } else {
            EMPTY_STR.as_bytes().to_vec()
        };
        let value_do: UserDo = item.into();
        let record = TransferRecordDto {
            table_name: Some(USER_TREE_NAME.clone()),
            key,
            value: value_do.to_bytes(),
            table_id: 0,
        };
        writer_actor.do_send(TransferWriterRequest::AddRecord(record));
        count += 1;
    }
    log::info!("transfer user count:{count}");
    Ok(())
}
