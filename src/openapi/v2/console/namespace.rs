use crate::common::appdata::AppShareData;
use crate::common::option_utils::OptionUtils;
use crate::common::string_utils::StringUtils;
use crate::console::model::NamespaceInfo;
use crate::console::NamespaceUtils;
use crate::merge_web_param;
use crate::openapi::v1::console::namespace::NamespaceVO;
use crate::openapi::v2::model::ApiResult;
use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

const NAMESPACE_ID_MAX_LENGTH: usize = 128;

/// nacos v2 命名空间接口参数
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NamespaceFormParam {
    pub namespace_id: Option<Arc<String>>,
    pub namespace_name: Option<String>,
    pub namespace_desc: Option<String>,
}

impl NamespaceFormParam {
    pub fn merge(self, other: Self) -> Self {
        Self {
            namespace_id: OptionUtils::select(self.namespace_id, other.namespace_id),
            namespace_name: OptionUtils::select(self.namespace_name, other.namespace_name),
            namespace_desc: OptionUtils::select(self.namespace_desc, other.namespace_desc),
        }
    }
}

impl From<NamespaceFormParam> for NamespaceInfo {
    fn from(value: NamespaceFormParam) -> Self {
        Self {
            namespace_id: value.namespace_id,
            namespace_name: value.namespace_name,
            r#type: None,
        }
    }
}

fn is_valid_namespace_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

fn is_valid_namespace_name(name: &str) -> bool {
    !name.chars().any(|c| "@#$%^&*".contains(c))
}

/// 查询命名空间列表
/// GET /nacos/v2/console/namespace/list
pub async fn query_namespace_list(app_data: web::Data<Arc<AppShareData>>) -> impl Responder {
    let namespaces = match NamespaceUtils::get_namespaces(&app_data).await {
        Ok(namespaces) => namespaces,
        Err(e) => {
            return HttpResponse::Ok().json(ApiResult::error(
                30000,
                e.to_string(),
                Vec::<NamespaceVO>::new(),
            ))
        }
    };
    let list: Vec<NamespaceVO> = namespaces.iter().map(|e| e.clone().into()).collect();
    HttpResponse::Ok().json(ApiResult::success(list))
}

/// 查询命名空间详情
/// GET /nacos/v2/console/namespace?namespaceId=xxx
pub async fn query_namespace(
    web::Query(param): web::Query<NamespaceFormParam>,
    payload: web::Payload,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let param = merge_web_param!(param, payload);
    if param.namespace_id.is_none()
        || param
            .namespace_id
            .as_ref()
            .map_or(true, |id| id.is_empty())
    {
        return HttpResponse::Ok().json(ApiResult::<String>::error(
            22000,
            "namespaceId is required".to_string(),
            String::new(),
        ));
    }
    match NamespaceUtils::get_namespace(&app_data, param.namespace_id).await {
        Ok(namespace) => {
            let namespace: NamespaceVO = namespace.into();
            HttpResponse::Ok().json(ApiResult::success(namespace))
        }
        Err(e) => HttpResponse::Ok().json(ApiResult::error(
            22001,
            format!("namespace not exist: {}", e),
            NamespaceVO::default(),
        )),
    }
}

/// 创建命名空间
/// POST /nacos/v2/console/namespace
pub async fn add_namespace(
    web::Query(param): web::Query<NamespaceFormParam>,
    payload: web::Payload,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let param = merge_web_param!(param, payload);
    let namespace_name = match &param.namespace_name {
        Some(name) if !name.is_empty() => name.clone(),
        _ => {
            return HttpResponse::Ok().json(ApiResult::<bool>::error(
                10000,
                "required parameter 'namespaceName' is missing".to_string(),
                false,
            ));
        }
    };
    if !is_valid_namespace_name(&namespace_name) {
        return HttpResponse::Ok().json(ApiResult::<bool>::error(
            22000,
            format!("namespaceName [{}] contains illegal char", namespace_name),
            false,
        ));
    }
    let mut param: NamespaceInfo = param.into();
    if StringUtils::is_option_empty_arc(&param.namespace_id) {
        param.namespace_id = Some(Arc::new(Uuid::new_v4().to_string()));
    } else {
        let id = param.namespace_id.as_ref().unwrap();
        let id_str = id.as_str().trim();
        if !is_valid_namespace_id(id_str) {
            return HttpResponse::Ok().json(ApiResult::<bool>::error(
                22000,
                format!("namespaceId [{}] mismatch the pattern", id_str),
                false,
            ));
        }
        if id_str.len() > NAMESPACE_ID_MAX_LENGTH {
            return HttpResponse::Ok().json(ApiResult::<bool>::error(
                22000,
                format!("too long namespaceId, over {}", NAMESPACE_ID_MAX_LENGTH),
                false,
            ));
        }
        param.namespace_id = Some(Arc::new(id_str.to_string()));
    }
    match NamespaceUtils::add_namespace(&app_data, param).await {
        Ok(_) => HttpResponse::Ok().json(ApiResult::success(true)),
        Err(e) => HttpResponse::Ok().json(ApiResult::error(30000, e.to_string(), false)),
    }
}

/// 编辑命名空间
/// PUT /nacos/v2/console/namespace
pub async fn update_namespace(
    web::Query(param): web::Query<NamespaceFormParam>,
    payload: web::Payload,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let param = merge_web_param!(param, payload);
    if param.namespace_id.is_none()
        || param
            .namespace_id
            .as_ref()
            .map_or(true, |id| id.is_empty())
    {
        return HttpResponse::Ok().json(ApiResult::<bool>::error(
            10000,
            "required parameter 'namespaceId' is missing".to_string(),
            false,
        ));
    }
    if let Some(name) = &param.namespace_name {
        if !name.is_empty() && !is_valid_namespace_name(name) {
            return HttpResponse::Ok().json(ApiResult::<bool>::error(
                22000,
                format!("namespaceName [{}] contains illegal char", name),
                false,
            ));
        }
    }
    match NamespaceUtils::update_namespace(&app_data, param.into()).await {
        Ok(_) => HttpResponse::Ok().json(ApiResult::success(true)),
        Err(e) => HttpResponse::Ok().json(ApiResult::error(30000, e.to_string(), false)),
    }
}

/// 删除命名空间
/// DELETE /nacos/v2/console/namespace?namespaceId=xxx
pub async fn remove_namespace(
    web::Query(param): web::Query<NamespaceFormParam>,
    payload: web::Payload,
    app_data: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let param = merge_web_param!(param, payload);
    if param.namespace_id.is_none()
        || param
            .namespace_id
            .as_ref()
            .map_or(true, |id| id.is_empty())
    {
        return HttpResponse::Ok().json(ApiResult::<bool>::error(
            10000,
            "required parameter 'namespaceId' is missing".to_string(),
            false,
        ));
    }
    match NamespaceUtils::remove_namespace(&app_data, param.namespace_id).await {
        Ok(_) => HttpResponse::Ok().json(ApiResult::success(true)),
        Err(e) => HttpResponse::Ok().json(ApiResult::error(30000, e.to_string(), false)),
    }
}
