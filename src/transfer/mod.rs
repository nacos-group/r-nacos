use crate::common::actor_utils::create_actor_at_thread;
use crate::common::constant::{
    CACHE_TREE_NAME, CONFIG_TREE_NAME, NAMESPACE_TREE_NAME, SEQUENCE_TREE_NAME, USER_TREE_NAME,
};
use crate::transfer::model::TransferWriterRequest;
use crate::transfer::writer::TransferWriterActor;
use actix::Addr;

pub mod context;
pub mod data_to_sqlite;
pub mod model;
pub mod mysql;
pub mod mysql_to_data;
pub mod openapi_to_data;
pub mod reader;
pub mod sqlite;
pub mod sqlite_to_data;
pub mod writer;

pub(crate) fn init_writer_actor(data_file: &str) -> Addr<TransferWriterActor> {
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
