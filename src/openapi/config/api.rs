use std::collections::HashMap;
use std::sync::Arc;

use actix::Addr;
use actix_web::{web, HttpRequest, HttpResponse, Responder, Scope};
use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::common::appdata::AppShareData;
use crate::common::model::ApiResult;
use crate::common::option_utils::OptionUtils;
use crate::common::string_utils::StringUtils;
use crate::common::web_utils::get_req_body;
use crate::config::config_index::ConfigQueryParam;
use crate::config::config_type::ConfigType;
use crate::config::core::{
    ConfigActor, ConfigCmd, ConfigInfoDto, ConfigKey, ConfigResult, ListenerItem, ListenerResult,
};
use crate::config::utils::param_utils;
use crate::config::ConfigUtils;
use crate::console::v2::ERROR_CODE_SYSTEM_ERROR;
use crate::merge_web_param;
use crate::openapi::constant::EMPTY;
use crate::raft::cluster::model::{DelConfigReq, SetConfigReq};
use crate::utils::select_option_by_clone;

pub(super) fn service() -> Scope {
    web::scope("/configs")
        .service(
            web::resource(EMPTY)
                .route(web::get().to(get_config))
                .route(web::post().to(add_config))
                .route(web::put().to(add_config))
                .route(web::delete().to(del_config)),
        )
        .service(web::resource("/listener").route(web::post().to(listener_config)))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigWebParams {
    pub data_id: Option<String>,
    pub group: Option<String>,
    pub tenant: Option<String>,
    pub content: Option<String>,
    pub desc: Option<String>,
    pub r#type: Option<String>,
    pub search: Option<String>,   //search type
    pub page_no: Option<usize>,   //use at search
    pub page_size: Option<usize>, //use at search
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSearchPage<T> {
    pub total_count: Option<usize>,
    pub page_number: Option<usize>,
    pub pages_available: Option<usize>,
    pub page_items: Option<Vec<T>>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConfigInfo {
    pub id: Option<String>,
    pub encrypted_data_key: Option<String>,
    pub app_name: Option<String>,
    pub r#type: Option<String>,

    pub tenant: Arc<String>,
    pub group: Arc<String>,
    pub data_id: Arc<String>,
    pub content: Option<Arc<String>>,
    pub md5: Option<Arc<String>>,
}

impl From<ConfigInfoDto> for ConfigInfo {
    fn from(v: ConfigInfoDto) -> Self {
        Self {
            tenant: v.tenant,
            group: v.group,
            data_id: v.data_id,
            content: v.content,
            md5: v.md5,
            ..Default::default()
        }
    }
}

impl ConfigWebParams {
    pub fn merge(self, other: Self) -> Self {
        Self {
            data_id: OptionUtils::select(self.data_id, other.data_id),
            group: OptionUtils::select(self.group, other.group),
            tenant: OptionUtils::select(self.tenant, other.tenant),
            content: OptionUtils::select(self.content, other.content),
            desc: OptionUtils::select(self.desc, other.desc),
            r#type: OptionUtils::select(self.r#type, other.r#type),
            search: OptionUtils::select(self.search, other.search),
            page_no: OptionUtils::select(self.page_no, other.page_no),
            page_size: OptionUtils::select(self.page_size, other.page_size),
        }
    }

    pub fn to_confirmed_param(&self) -> Result<ConfigWebConfirmedParam, String> {
        let mut param = ConfigWebConfirmedParam::default();
        if let Some(v) = self.data_id.as_ref() {
            if v.is_empty() {
                return Err("dataId is empty".to_owned());
            }
            v.clone_into(&mut param.data_id);
        }
        self.group
            .as_ref()
            .unwrap_or(&"DEFAULT_GROUP".to_owned())
            .clone_into(&mut param.group);
        //param.tenant= self.tenant.as_ref().unwrap_or(&"public".to_owned()).to_owned();
        self.tenant
            .as_ref()
            .unwrap_or(&"".to_owned())
            .clone_into(&mut param.tenant);
        if param.tenant == "public" {
            "".clone_into(&mut param.tenant)
        }
        if let Some(v) = self.content.as_ref() {
            if !v.is_empty() {
                v.clone_into(&mut param.content);
            }
        }
        Ok(param)
    }

    pub fn build_like_search_param(self) -> ConfigQueryParam {
        let limit = self.page_size.unwrap_or(0xffff_ffff);
        let offset = (self.page_no.unwrap_or(1) - 1) * limit;
        let mut param = ConfigQueryParam {
            limit,
            offset,
            like_group: self.group,
            like_data_id: self.data_id,
            query_context: true,
            ..Default::default()
        };
        param.tenant = Some(Arc::new(ConfigUtils::default_tenant(
            self.tenant.unwrap_or_default(),
        )));
        param
    }

    pub fn build_search_param(self) -> ConfigQueryParam {
        let limit = self.page_size.unwrap_or(0xffff_ffff);
        let offset = (self.page_no.unwrap_or(1) - 1) * limit;
        let mut param = ConfigQueryParam {
            limit,
            offset,
            group: self.group.map(Arc::new),
            data_id: self.data_id.map(Arc::new),
            query_context: true,
            ..Default::default()
        };
        param.tenant = Some(Arc::new(ConfigUtils::default_tenant(
            self.tenant.unwrap_or_default(),
        )));
        param
    }
}

#[derive(Debug, Default, Clone)]
pub struct ConfigWebConfirmedParam {
    pub data_id: String,
    pub group: String,
    pub tenant: String,
    pub content: String,
}

pub(crate) async fn add_config(
    a: web::Query<ConfigWebParams>,
    payload: web::Payload,
    appdata: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let selected_param = merge_web_param!(a.0, payload);
    match param_utils::check_tenant(&selected_param.tenant) {
        Ok(v) => v,
        Err(err) => {
            return HttpResponse::InternalServerError().body(err.to_string());
        }
    }

    match param_utils::check_param(
        &selected_param.data_id,
        &selected_param.group,
        &Some(String::from("datumId")),
        &selected_param.content,
    ) {
        Ok(v) => v,
        Err(err) => {
            return HttpResponse::InternalServerError().body(err.to_string());
        }
    }

    let config_type = StringUtils::map_not_empty(selected_param.r#type.clone());
    let desc = StringUtils::map_not_empty(selected_param.desc.clone());
    let param = selected_param.to_confirmed_param();
    match param {
        Ok(p) => {
            let mut req = SetConfigReq::new(
                ConfigKey::new(&p.data_id, &p.group, &p.tenant),
                Arc::new(p.content.to_owned()),
            );
            req.config_type = config_type.map(|v| ConfigType::new_by_value(v.as_ref()).get_value());
            req.desc = desc.map(Arc::new);
            match appdata.config_route.set_config(req).await {
                Ok(_) => HttpResponse::Ok()
                    .content_type("text/html; charset=utf-8")
                    .body("true"),
                Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
            }
        }
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

pub(crate) async fn del_config(
    a: web::Query<ConfigWebParams>,
    payload: web::Payload,
    appdata: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    let selected_param = merge_web_param!(a.0, payload);
    match param_utils::check_tenant(&selected_param.tenant) {
        Ok(v) => v,
        Err(err) => {
            return HttpResponse::InternalServerError().body(err.to_string());
        }
    }

    match param_utils::check_param(
        &selected_param.data_id,
        &selected_param.group,
        &Some(String::from("datumId")),
        &Some(String::from("rm")),
    ) {
        Ok(v) => v,
        Err(err) => {
            return HttpResponse::InternalServerError().body(err.to_string());
        }
    }

    let param = selected_param.to_confirmed_param();
    match param {
        Ok(p) => {
            let req = DelConfigReq::new(ConfigKey::new(&p.data_id, &p.group, &p.tenant));
            match appdata.config_route.del_config(req).await {
                Ok(_) => HttpResponse::Ok()
                    .content_type("text/html; charset=utf-8")
                    .body("true"),
                Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
            }
        }
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

pub(crate) async fn get_config(
    web_param: web::Query<ConfigWebParams>,
    appdata: web::Data<Arc<AppShareData>>,
) -> impl Responder {
    if let Some(search) = web_param.search.as_ref() {
        if search == "blur" {
            let query_param = web_param.0.build_like_search_param();
            return do_search_config(query_param, appdata).await;
        } else if search == "accurate" {
            let query_param = web_param.0.build_search_param();
            return do_search_config(query_param, appdata).await;
        }
    };
    let param = web_param.to_confirmed_param();
    match param {
        Ok(p) => {
            let cmd = ConfigCmd::GET(ConfigKey::new(&p.data_id, &p.group, &p.tenant));
            match appdata.config_addr.send(cmd).await {
                Ok(res) => {
                    let r: ConfigResult = res.unwrap();
                    match r {
                        ConfigResult::Data {
                            value: v,
                            md5,
                            config_type,
                            ..
                        } => HttpResponse::Ok()
                            .content_type(
                                config_type
                                    .map(|v| ConfigType::new_by_value(&v))
                                    .unwrap_or_default()
                                    .get_media_type(),
                            )
                            .insert_header(("content-md5", md5.as_ref().to_string()))
                            .body(v.as_ref().as_bytes().to_vec()),
                        _ => HttpResponse::NotFound().body("config data not exist"),
                    }
                }
                Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
            }
        }
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

async fn do_search_config(
    query_param: ConfigQueryParam,
    appdata: web::Data<Arc<AppShareData>>,
) -> HttpResponse {
    let page_size = query_param.limit;
    let page_number = query_param.offset / query_param.limit + 1;
    let cmd = ConfigCmd::QueryPageInfo(Box::new(query_param));
    match appdata.config_addr.send(cmd).await {
        Ok(res) => {
            let r: ConfigResult = res.unwrap();
            match r {
                ConfigResult::ConfigInfoPage(total_count, list) => {
                    let page = ConfigSearchPage {
                        total_count: Some(total_count),
                        page_number: Some(page_number),
                        pages_available: Some(total_count.div_ceil(page_size)),
                        page_items: Some(list),
                    };
                    HttpResponse::Ok().json(page)
                }
                _ => HttpResponse::Ok().json(ApiResult::<()>::error(
                    ERROR_CODE_SYSTEM_ERROR.to_string(),
                    None,
                )),
            }
        }
        Err(err) => HttpResponse::Ok().json(ApiResult::<()>::error(
            ERROR_CODE_SYSTEM_ERROR.to_string(),
            Some(err.to_string()),
        )),
    }
}

#[derive(Serialize, Deserialize)]
pub struct ListenerParams {
    #[serde(rename(serialize = "Listening-Configs", deserialize = "Listening-Configs"))]
    configs: Option<String>,
}

impl ListenerParams {
    pub fn select_option(&self, o: &Self) -> Self {
        Self {
            configs: select_option_by_clone(&self.configs, &o.configs),
        }
    }

    pub fn to_items(&self) -> Vec<ListenerItem> {
        let config = self.configs.as_ref().unwrap_or(&"".to_owned()).to_owned();
        ListenerItem::decode_listener_items(&config)
    }
}

pub(super) async fn listener_config(
    _req: HttpRequest,
    a: web::Query<ListenerParams>,
    payload: web::Payload,
    config_addr: web::Data<Addr<ConfigActor>>,
) -> impl Responder {
    let body = match get_req_body(payload).await {
        Ok(v) => v,
        Err(err) => {
            return HttpResponse::InternalServerError().body(err.to_string());
        }
    };
    let b = match serde_urlencoded::from_bytes(&body) {
        Ok(v) => v,
        Err(err) => {
            return HttpResponse::InternalServerError().body(err.to_string());
        }
    };
    let list = a.select_option(&b).to_items();
    if list.is_empty() {
        //println!("listener_config error: listener item len == 0");
        return HttpResponse::NoContent()
            .content_type("text/html; charset=utf-8")
            .body("error:listener empty");
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    let current_time = Local::now().timestamp_millis();
    let mut time_out = 0;
    if let Some(_timeout) = _req.headers().get("Long-Pulling-Timeout") {
        match _timeout.to_str().unwrap().parse::<i64>() {
            Ok(v) => {
                time_out = current_time + v.clamp(10000, 120000) - 500;
            }
            Err(_) => {
                time_out = 0;
            }
        }
    }
    //println!("timeout header:{:?},time_out:{}",_req.headers().get("Long-Pulling-Timeout") ,time_out);
    let cmd = ConfigCmd::LISTENER(list, tx, time_out);
    let _ = config_addr.send(cmd).await;
    let res = rx.await.unwrap();
    let v = match res {
        ListenerResult::DATA(list) => {
            let mut data = "".to_string();
            for item in list {
                data += &item.build_key();
                data += "\x01";
            }
            let mut tmp_param = HashMap::new();
            tmp_param.insert("_", data);
            let t = serde_urlencoded::to_string(&tmp_param).unwrap();
            t[2..t.len()].to_owned() + "\n"
        }
        ListenerResult::NULL => "".to_owned(),
    };
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(v)
}
