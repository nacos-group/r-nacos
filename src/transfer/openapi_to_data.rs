use crate::common::constant::{CONFIG_TREE_NAME, EMPTY_STR, NAMESPACE_TREE_NAME};
use crate::config::core::{ConfigKey, ConfigValue};
use crate::config::model::ConfigValueDO;
use crate::namespace::model::{NamespaceDO, FROM_USER_VALUE};
use crate::now_millis_i64;
use crate::transfer::init_writer_actor;
use crate::transfer::model::{
    TransferRecordDto, TransferWriterAsyncRequest, TransferWriterRequest,
};
use crate::transfer::sqlite::TableSeq;
use crate::transfer::writer::TransferWriterActor;
use actix::Addr;
use nacos_rust_client::client::api_model::NamespaceInfo;
use nacos_rust_client::client::config_client::api_model::{ConfigInfoDto, ConfigQueryParams};
use nacos_rust_client::client::{AuthInfo, ClientBuilder, ConfigClient};
use std::sync::Arc;

pub async fn openapi_to_data(
    host: &str,
    username: &str,
    password: &str,
    data_file: &str,
) -> anyhow::Result<()> {
    let auth_info = if username.is_empty() || password.is_empty() {
        None
    } else {
        Some(AuthInfo::new(username, password))
    };
    let config_client = ClientBuilder::new()
        .set_endpoint_addrs(host)
        .set_auth_info(auth_info)
        .set_use_grpc(false)
        .build_config_client();

    let writer_actor = init_writer_actor(data_file);
    let mut table_seq = TableSeq::default();

    let result = config_client.get_namespace_list().await?;
    let namespaces = result.data.unwrap_or_default();
    apply_config(&mut table_seq, &namespaces, &config_client, &writer_actor).await?;
    apply_tenant(&namespaces, &writer_actor)?;
    writer_actor
        .send(TransferWriterAsyncRequest::Flush)
        .await
        .ok();
    Ok(())
}

fn apply_tenant(
    namespace_list: &[NamespaceInfo],
    writer_actor: &Addr<TransferWriterActor>,
) -> anyhow::Result<()> {
    let mut count = 0;
    for item in namespace_list {
        let key = if let Some(v) = &item.namespace {
            v.as_bytes().to_vec()
        } else {
            EMPTY_STR.as_bytes().to_vec()
        };
        let value_do = NamespaceDO {
            namespace_id: item.namespace.clone(),
            namespace_name: item.namespace_show_name.clone(),
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

async fn apply_config(
    table_seq: &mut TableSeq,
    namespace_list: &[NamespaceInfo],
    config_client: &ConfigClient,
    writer_actor: &Addr<TransferWriterActor>,
) -> anyhow::Result<()> {
    let mut count = 0;
    for namespace_info in namespace_list {
        let namespace_id = if let Some(namespace_id) = namespace_info.namespace.as_ref() {
            namespace_id
        } else {
            return Err(anyhow::anyhow!("namespace_id is none"));
        };
        let mut current_page = 0;
        let mut total_page = 1;
        let mut total_count = 0;
        let mut params = ConfigQueryParams {
            tenant: Some(namespace_id.to_owned()),
            page_no: Some(1),
            page_size: Some(100),
            ..Default::default()
        };
        while current_page < total_page {
            current_page += 1;
            params.page_no = Some(current_page);
            let res = config_client
                .query_accurate_config_page(params.clone())
                .await?;
            total_page = res.pages_available.unwrap_or_default();
            total_count = res.total_count.unwrap_or_default();
            let configs = res.page_items.unwrap_or_default();
            for config in &configs {
                let key = ConfigKey::new(&config.data_id, &config.group, namespace_id);
                let record = build_config_record(table_seq, key, config)?;
                count += 1;
                writer_actor.do_send(TransferWriterRequest::AddRecord(record));
            }
            if configs.is_empty() {
                break;
            }
        }
        log::info!("[namespace {}],config count:{}", namespace_id, total_count);
    }
    log::info!("transfer config total count:{count}");
    Ok(())
}

fn build_config_record(
    table_seq: &mut TableSeq,
    key: ConfigKey,
    config_info: &ConfigInfoDto,
) -> anyhow::Result<TransferRecordDto> {
    let content = Arc::new(config_info.content.clone().unwrap_or_default());
    let mut config_value = ConfigValue::new(content.clone());
    config_value.update_value(
        content,
        table_seq.next_config_id() as u64,
        now_millis_i64(),
        None,
        None,
    );
    let value_do: ConfigValueDO = config_value.into();
    let record = TransferRecordDto {
        table_name: Some(CONFIG_TREE_NAME.clone()),
        key: key.build_key().as_bytes().to_vec(),
        value: value_do.to_bytes()?,
        table_id: 0,
    };
    Ok(record)
}
