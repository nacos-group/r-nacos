use super::core::ConfigAsyncCmd;
use super::core::{ConfigActor, ConfigCmd, ConfigKey, ConfigResult, ListenerItem, ListenerResult};
use crate::cluster::route::{SetConfigReq, DelConfigReq};
use crate::common::appdata::AppData;
use crate::utils::select_option_by_clone;
use chrono::Local;
use std::cmp::max;
use std::cmp::min;
use std::sync::Arc;

use actix_web::{web, HttpRequest, HttpResponse, Responder};

use actix::prelude::Addr;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigWebParams {
    pub data_id: Option<String>,
    pub group: Option<String>,
    pub tenant: Option<String>,
    pub content: Option<String>,
}

impl ConfigWebParams {
    pub fn select_option(&self, o: &Self) -> Self {
        Self {
            data_id: select_option_by_clone(&self.data_id, &o.data_id),
            group: select_option_by_clone(&self.group, &o.group),
            tenant: select_option_by_clone(&self.tenant, &o.tenant),
            content: select_option_by_clone(&self.content, &o.content),
        }
    }

    pub fn to_confirmed_param(&self) -> Result<ConfigWebConfirmedParam, String> {
        let mut param = ConfigWebConfirmedParam::default();
        if let Some(v) = self.data_id.as_ref() {
            if v.is_empty() {
                return Err("dataId is empty".to_owned());
            }
            param.data_id = v.to_owned();
        }
        param.group = self
            .group
            .as_ref()
            .unwrap_or(&"DEFAULT_GROUP".to_owned())
            .to_owned();
        //param.tenant= self.tenant.as_ref().unwrap_or(&"public".to_owned()).to_owned();
        param.tenant = self.tenant.as_ref().unwrap_or(&"".to_owned()).to_owned();
        if param.tenant == "public" {
            param.tenant = "".to_owned();
        }
        if let Some(v) = self.content.as_ref() {
            if !v.is_empty() {
                param.content = v.to_owned();
            }
        }
        Ok(param)
    }
}

#[derive(Debug, Default, Clone)]
pub struct ConfigWebConfirmedParam {
    pub data_id: String,
    pub group: String,
    pub tenant: String,
    pub content: String,
}

async fn add_config(
    a: web::Query<ConfigWebParams>,
    b: web::Form<ConfigWebParams>,
    appdata: web::Data<Arc<AppData>>,
) -> impl Responder {
    let param = a.select_option(&b).to_confirmed_param();
    match param {
        Ok(p) => {
            let req = SetConfigReq::new(
                ConfigKey::new(&p.data_id, &p.group, &p.tenant),
                Arc::new(p.content.to_owned())
            );
            match appdata.config_route.set_config(req).await {
                Ok(_) => {
                    HttpResponse::Ok()
                        .content_type("text/html; charset=utf-8")
                        .body("true")
                },
                Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
            }
        }
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

async fn del_config(
    a: web::Query<ConfigWebParams>,
    b: web::Form<ConfigWebParams>,
    appdata: web::Data<Arc<AppData>>,
) -> impl Responder {
    let param = a.select_option(&b).to_confirmed_param();
    match param {
        Ok(p) => {
            let req = DelConfigReq::new(
                ConfigKey::new(&p.data_id, &p.group, &p.tenant)
            );
            match appdata.config_route.del_config(req).await {
                Ok(_) => {
                    HttpResponse::Ok()
                        .content_type("text/html; charset=utf-8")
                        .body("true")
                },
                Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
            }
        }
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

async fn get_config(
    a: web::Query<ConfigWebParams>,
    config_addr: web::Data<Addr<ConfigActor>>,
) -> impl Responder {
    let param = a.to_confirmed_param();
    match param {
        Ok(p) => {
            let cmd = ConfigCmd::GET(ConfigKey::new(&p.data_id, &p.group, &p.tenant));
            match config_addr.send(cmd).await {
                Ok(res) => {
                    let r: ConfigResult = res.unwrap();
                    match r {
                        ConfigResult::DATA(v, md5) => HttpResponse::Ok()
                            .content_type("text/html; charset=utf-8")
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

async fn listener_config(
    _req: HttpRequest,
    a: web::Query<ListenerParams>,
    b: web::Form<ListenerParams>,
    config_addr: web::Data<Addr<ConfigActor>>,
) -> impl Responder {
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
                time_out = current_time + min(max(10000, v), 120000) - 500;
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

pub fn app_config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/nacos/v1/cs")
            .service(
                web::resource("/configs")
                    .route(web::get().to(get_config))
                    .route(web::post().to(add_config))
                    .route(web::put().to(add_config))
                    .route(web::delete().to(del_config)),
            )
            .service(web::resource("/configs/listener").route(web::post().to(listener_config))),
    );
}
