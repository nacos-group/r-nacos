use crate::common::appdata::AppShareData;
use crate::common::option_utils::OptionUtils;
use crate::common::string_utils::StringUtils;
use crate::config::core::ConfigActor;
use crate::console::model::{ConsoleResult, NamespaceInfo};
use crate::console::NamespaceUtils;
use crate::merge_web_param;
use actix::Addr;
use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NamespaceVO {
    pub namespace: Option<String>,
    pub namespace_show_name: Option<String>,
    pub namespace_desc: Option<String>,
    pub quota: u32,
    pub config_count: u32,
    pub r#type: u32,
}

impl From<NamespaceInfo> for NamespaceVO {
    fn from(value: NamespaceInfo) -> Self {
        Self {
            namespace: value.namespace_id,
            namespace_show_name: value.namespace_name,
            namespace_desc: None,
            quota: 200,
            config_count: 0,
            r#type: value
                .r#type
                .unwrap_or("2".to_string())
                .parse()
                .unwrap_or_default(),
        }
    }
}

///
/// 命名空间接口参数
/// 这里参数字段需要与nacos对应接口完全兼容
///
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NamespaceParam {
    /// custom_namespace_id,创建时的id
    pub custom_namespace_id: Option<String>,
    /// namespace,更新时的id
    pub namespace: Option<String>,
    /// namespace_id,删除时的id
    pub namespace_id: Option<String>,
    /// namespace_name,创建时的名称
    pub namespace_name: Option<String>,
    /// namespace_show_name,更新时的名称
    pub namespace_show_name: Option<String>,
    pub namespace_desc: Option<String>,
}

impl NamespaceParam {
    pub fn merge(self, other: Self) -> Self {
        Self {
            custom_namespace_id: OptionUtils::select(
                self.custom_namespace_id,
                other.custom_namespace_id,
            ),
            namespace: OptionUtils::select(self.namespace, other.namespace),
            namespace_id: OptionUtils::select(self.namespace_id, other.namespace_id),
            namespace_name: OptionUtils::select(self.namespace_name, other.namespace_name),
            namespace_show_name: OptionUtils::select(
                self.namespace_show_name,
                other.namespace_show_name,
            ),
            namespace_desc: OptionUtils::select(self.namespace_desc, other.namespace_desc),
        }
    }
}

impl From<NamespaceParam> for NamespaceInfo {
    fn from(value: NamespaceParam) -> Self {
        Self {
            namespace_id: OptionUtils::select(
                OptionUtils::select(value.custom_namespace_id, value.namespace),
                value.namespace_id,
            ),
            namespace_name: OptionUtils::select(value.namespace_show_name, value.namespace_name),
            r#type: None,
        }
    }
}

pub async fn query_namespace_list(config_addr: web::Data<Addr<ConfigActor>>) -> impl Responder {
    //HttpResponse::InternalServerError().body("system error")
    let namespaces = NamespaceUtils::get_namespaces(&config_addr).await;
    let list: Vec<NamespaceVO> = namespaces
        .iter()
        .map(|e| e.as_ref().to_owned().into())
        .collect();
    let result = ConsoleResult::success(list);
    HttpResponse::Ok().json(result)
}

pub async fn add_namespace(
    web::Query(param): web::Query<NamespaceParam>,
    payload: web::Payload,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let param = merge_web_param!(param, payload);
    let mut param: NamespaceInfo = param.into();
    if StringUtils::is_option_empty(&param.namespace_id) {
        param.namespace_id = Some(Uuid::new_v4().to_string());
    }
    match NamespaceUtils::add_namespace(&app_data, param).await {
        Ok(_) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body("true"),
        Err(e) => HttpResponse::InternalServerError()
            .content_type("text/html; charset=utf-8")
            .body(e.to_string()),
    }
}

pub async fn update_namespace(
    web::Query(param): web::Query<NamespaceParam>,
    payload: web::Payload,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let param = merge_web_param!(param, payload);
    match NamespaceUtils::update_namespace(&app_data, param.into()).await {
        Ok(_) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body("true"),
        Err(e) => HttpResponse::InternalServerError()
            .content_type("text/html; charset=utf-8")
            .body(e.to_string()),
    }
}

pub async fn remove_namespace(
    web::Query(param): web::Query<NamespaceParam>,
    payload: web::Payload,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let param = merge_web_param!(param, payload);
    match NamespaceUtils::remove_namespace(&app_data, param.namespace_id).await {
        Ok(_) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body("true"),
        Err(e) => HttpResponse::InternalServerError()
            .content_type("text/html; charset=utf-8")
            .body(e.to_string()),
    }
}
