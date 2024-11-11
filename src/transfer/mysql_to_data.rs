use crate::common::constant::{
    CONFIG_TREE_NAME, EMPTY_ARC_STRING, EMPTY_STR, NAMESPACE_TREE_NAME, USER_TREE_NAME,
};
use crate::common::sqlx_utils::MySqlExecutor;
use crate::config::core::{ConfigKey, ConfigValue};
use crate::config::model::ConfigValueDO;
use crate::config::ConfigUtils;
use crate::namespace::model::{NamespaceDO, FROM_USER_VALUE};
use crate::now_millis_i64;
use crate::transfer::init_writer_actor;
use crate::transfer::model::{TransferRecordDto, TransferWriterRequest};
use crate::transfer::mysql::dao::config::{ConfigInfoDO, ConfigInfoDao, ConfigInfoParam};
use crate::transfer::mysql::dao::config_history::{
    ConfigHistoryDO, ConfigHistoryDao, ConfigHistoryParam,
};
use crate::transfer::mysql::dao::tenant::{TenantDao, TenantParam};
use crate::transfer::mysql::dao::user::{UserDao, UserParam};
use crate::transfer::sqlite::TableSeq;
use crate::transfer::writer::TransferWriterActor;
use crate::user::model::UserDo;
use actix::Addr;
use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions};
use sqlx::{MySql, Pool};
use std::str::FromStr;
use std::sync::Arc;

pub async fn mysql_to_data(db_uri: &str, data_file: &str) -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,sqlx::query=error");
    let option = MySqlConnectOptions::from_str(db_uri)?;
    let pool = MySqlPoolOptions::new()
        .max_connections(2)
        .connect_with(option)
        .await?;
    let writer_actor = init_writer_actor(data_file);
    let mut table_seq = TableSeq::default();
    apply_config(&pool, &mut table_seq, &writer_actor).await?;
    apply_tenant(&pool, &writer_actor).await?;
    apply_user(&pool, &writer_actor).await?;
    Ok(())
}

async fn apply_config(
    pool: &Pool<MySql>,
    table_seq: &mut TableSeq,
    writer_actor: &Addr<TransferWriterActor>,
) -> anyhow::Result<()> {
    let mut config_pool = pool.clone();
    let mut config_history_pool = pool.clone();
    let mut config_dao = ConfigInfoDao::new(MySqlExecutor::new_by_pool(&mut config_pool));
    let mut config_history_dao =
        ConfigHistoryDao::new(MySqlExecutor::new_by_pool(&mut config_history_pool));
    let mut count = 0;
    let mut offset = 0;
    let limit = 100;
    let mut config_query_param = ConfigInfoParam {
        id: None,
        limit: Some(limit),
        offset: Some(offset),
    };
    let mut patch = config_dao.query(&config_query_param).await?;
    let mut patch_is_empty = patch.is_empty();
    while !patch_is_empty {
        for item in patch {
            let mut tenant_id = Arc::new(item.tenant_id.clone().unwrap_or_default());
            if ConfigUtils::is_default_tenant(&tenant_id) {
                tenant_id = EMPTY_ARC_STRING.clone();
            }
            let key = ConfigKey::new_by_arc(
                Arc::new(item.data_id.clone().unwrap_or_default()),
                Arc::new(item.group_id.clone().unwrap_or_default()),
                tenant_id,
            );
            let history_query_param = ConfigHistoryParam {
                id: None,
                data_id: Some(key.data_id.clone()),
                group_id: Some(key.group.clone()),
                tenant_id: Some(key.tenant.clone()),
                limit: Some(100),
                offset: None,
                order_by_gmt_create_desc: true,
            };
            let histories = config_history_dao.query(&history_query_param).await?;
            let record = build_config_record(table_seq, key, item, histories)?;
            count += 1;
            writer_actor.do_send(TransferWriterRequest::AddRecord(record));
        }
        offset += limit;
        config_query_param.offset = Some(offset);
        patch = config_dao.query(&config_query_param).await?;
        patch_is_empty = patch.is_empty();
    }
    log::info!("transfer config total count:{count}");
    Ok(())
}

fn build_config_record(
    table_seq: &mut TableSeq,
    key: ConfigKey,
    config_do: ConfigInfoDO,
    histories: Vec<ConfigHistoryDO>,
) -> anyhow::Result<TransferRecordDto> {
    let current_content = config_do.content.unwrap_or_default();
    let mut config_value = ConfigValue::new(Arc::new(current_content.clone()));
    let mut last_content = None;
    let mut use_histories = vec![];
    for item in histories {
        if let Some(op_type) = &item.op_type {
            if op_type == "D" {
                //删除
                break;
            }
        }
        use_histories.push(item);
    }
    let mut last_op_time = 0;
    for item in use_histories.into_iter().rev() {
        if let Some(content) = item.content {
            let op_time = item
                .gmt_create_timestamp
                .map(|v| v * 1000)
                .unwrap_or(now_millis_i64());
            last_op_time = op_time;
            last_content = Some(content.clone());
            config_value.update_value(
                Arc::new(content),
                table_seq.next_config_id() as u64,
                op_time,
                None,
                item.src_user.map(Arc::new),
            );
        }
    }
    let need_pull_current = if let Some(last_content) = &last_content {
        last_content != &current_content
    } else {
        true
    };
    if need_pull_current {
        let op_time = config_do
            .gmt_modified_timestamp
            .map(|v| v * 1000)
            .unwrap_or(now_millis_i64());
        let op_time = std::cmp::max(op_time, last_op_time);
        config_value.update_value(
            Arc::new(current_content),
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

async fn apply_tenant(
    pool: &Pool<MySql>,
    writer_actor: &Addr<TransferWriterActor>,
) -> anyhow::Result<()> {
    let mut count = 0;
    let mut new_pool = pool.clone();
    let mut tenant_dao = TenantDao::new(MySqlExecutor::new_by_pool(&mut new_pool));
    let query_param = TenantParam::default();
    for item in tenant_dao.query(&query_param).await? {
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

async fn apply_user(
    pool: &Pool<MySql>,
    writer_actor: &Addr<TransferWriterActor>,
) -> anyhow::Result<()> {
    let mut count = 0;
    let mut new_pool = pool.clone();
    let mut user_dao = UserDao::new(MySqlExecutor::new_by_pool(&mut new_pool));
    let query_param = UserParam::default();
    for item in user_dao.query(&query_param).await? {
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
