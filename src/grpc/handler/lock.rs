use crate::common::appdata::AppShareData;
use crate::grpc::api_model::{LockOperationEnum, LockOperationRequest};
use crate::grpc::nacos_proto::Payload;
use crate::grpc::{HandlerResult, PayloadHandler, PayloadUtils, RequestMeta};
use crate::lock::model::LockRaftReq;
use async_trait::async_trait;
use std::sync::Arc;

pub(crate) const LOCK_OPERATION_RESPONSE: &str = "LockOperationResponse";

pub struct LockRequestHandler {
    app_data: Arc<AppShareData>,
}

impl LockRequestHandler {
    pub fn new(app_data: Arc<AppShareData>) -> Self {
        Self { app_data }
    }
}

#[async_trait]
impl PayloadHandler for LockRequestHandler {
    async fn handle(
        &self,
        request_payload: Payload,
        _request_meta: RequestMeta,
    ) -> anyhow::Result<HandlerResult> {
        let body_vec = request_payload.body.unwrap_or_default().value;
        let request: LockOperationRequest = serde_json::from_slice(&body_vec)?;
        log::info!(
            "Lock request: {:?}, instance: {:?}",
            request.lock_operation_enum,
            request.lock_instance
        );

        let result = match request.lock_operation_enum {
            LockOperationEnum::Acquire => {
                self.app_data
                    .raft_request_route
                    .request_lock(LockRaftReq::Acquire {
                        instance: request.lock_instance,
                    })
                    .await?
            }
            LockOperationEnum::Release => {
                self.app_data
                    .raft_request_route
                    .request_lock(LockRaftReq::Release {
                        instance: request.lock_instance,
                    })
                    .await?
            }
            LockOperationEnum::Expire => {
                // 暂不支持
                return Ok(HandlerResult::success(PayloadUtils::build_payload(
                    LOCK_OPERATION_RESPONSE,
                    serde_json::to_string(&"Expire operation not supported")?,
                )));
            }
        };

        Ok(HandlerResult::success(PayloadUtils::build_payload(
            LOCK_OPERATION_RESPONSE,
            serde_json::to_string(&result)?,
        )))
    }
}
