use self::api_model::BaseResponse;
use crate::common::model::privilege::NamespacePrivilegeGroup;
use crate::common::model::{ClientVersion, TokenSession};
use async_trait::async_trait;
use std::fmt::Display;
use std::{collections::HashMap, sync::Arc};

pub mod api_model;
pub mod bistream_conn;
pub mod bistream_manage;
pub mod handler;
pub mod metrics;
pub mod nacos_proto;
pub mod server;

#[derive(Default)]
pub struct RequestMeta {
    pub connection_id: Arc<String>,
    pub client_ip: String,
    pub labels: HashMap<String, String>,
    pub token_session: Option<Arc<TokenSession>>,
    pub cluster_token_is_valid: bool,
    pub client_version: Arc<ClientVersion>,
}

impl RequestMeta {
    /// 命名空间数据权限组; 无会话(鉴权关闭)时为全量权限
    pub fn namespace_privilege(&self) -> NamespacePrivilegeGroup {
        match &self.token_session {
            Some(session) => session.namespace_privilege_group(),
            None => NamespacePrivilegeGroup::default(),
        }
    }

    /// 是否可编辑; 无会话(鉴权关闭)时不限制, 返回 true
    pub fn can_edit(&self) -> bool {
        match &self.token_session {
            Some(session) => session.can_edit(),
            None => true,
        }
    }
}

pub struct HandlerResult {
    pub success: bool,
    pub payload: nacos_proto::Payload,
    pub message: Option<String>,
}
impl HandlerResult {
    pub fn success(payload: nacos_proto::Payload) -> Self {
        Self {
            success: true,
            message: None,
            payload,
        }
    }

    pub fn error(code: u16, message: String) -> Self {
        let payload = PayloadUtils::build_error_payload(code, message.clone());
        Self {
            success: false,
            message: Some(message),
            payload,
        }
    }

    pub fn error_mark(payload: nacos_proto::Payload) -> Self {
        Self {
            success: false,
            message: None,
            payload,
        }
    }

    pub fn error_with_message(payload: nacos_proto::Payload, message: String) -> Self {
        Self {
            success: false,
            message: Some(message),
            payload,
        }
    }
}

pub enum HandleLogArgs {
    /// 日志参数内容为空
    None,
    /// 忽略,不打印日志
    Ignore,
    /// 打印日志
    Arg(String),
}

impl Display for HandleLogArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            HandleLogArgs::None => "".to_string(),
            HandleLogArgs::Ignore => "".to_string(),
            HandleLogArgs::Arg(arg) => arg.to_string(),
        };
        write!(f, "{}", str)
    }
}

impl HandleLogArgs {
    pub fn enable_log(&self) -> bool {
        match self {
            HandleLogArgs::None => true,
            HandleLogArgs::Ignore => false,
            HandleLogArgs::Arg(_) => true,
        }
    }
}

#[async_trait]
pub trait PayloadHandler {
    async fn handle(
        &self,
        request_payload: nacos_proto::Payload,
        request_meta: RequestMeta,
    ) -> anyhow::Result<HandlerResult>;

    /// 获取请求日志参数
    fn get_log_args(
        &self,
        _request_payload: &nacos_proto::Payload,
        _request_meta: &RequestMeta,
    ) -> HandleLogArgs {
        HandleLogArgs::None
    }
}

pub struct PayloadUtils;

impl PayloadUtils {
    pub fn new_metadata(
        r#type: &str,
        client_ip: &str,
        headers: HashMap<String, String>,
    ) -> nacos_proto::Metadata {
        nacos_proto::Metadata {
            r#type: r#type.to_owned(),
            client_ip: client_ip.to_owned(),
            headers,
        }
    }

    pub fn build_error_payload(error_code: u16, error_msg: String) -> nacos_proto::Payload {
        let error_val = BaseResponse::build_error_response(error_code, error_msg).to_json_string();
        Self::build_payload("ErrorResponse", error_val)
    }

    pub fn build_payload(url: &str, val: String) -> nacos_proto::Payload {
        Self::build_full_payload(url, val, "", Default::default())
    }

    pub fn build_full_payload(
        url: &str,
        val: String,
        client_ip: &str,
        headers: HashMap<String, String>,
    ) -> nacos_proto::Payload {
        let body = nacos_proto::Any {
            type_url: "".into(),
            value: val.into_bytes(),
        };
        let meta = Self::new_metadata(url, client_ip, headers);
        nacos_proto::Payload {
            body: Some(body),
            metadata: Some(meta),
        }
    }

    pub fn get_payload_header(payload: &nacos_proto::Payload) -> String {
        let mut str = String::default();
        if let Some(meta) = &payload.metadata {
            str.push_str(&format!("type:{},", meta.r#type));
            str.push_str(&format!("client_ip:{},", meta.client_ip));
            str.push_str(&format!("header:{:?},", meta.headers));
        }
        str
    }

    pub fn get_payload_string(payload: &nacos_proto::Payload) -> String {
        let mut str = String::default();
        if let Some(meta) = &payload.metadata {
            str.push_str(&format!("type:{},\n\t", meta.r#type));
            str.push_str(&format!("client_ip:{},\n\t", meta.client_ip));
            str.push_str(&format!("header:{:?},\n\t", meta.headers));
        }
        if let Some(body) = &payload.body {
            let new_value = body.clone();
            let value_str = String::from_utf8(new_value.value).unwrap();
            str.push_str(&format!("body:{}", value_str));
        }
        str
    }

    pub fn get_payload_type(payload: &nacos_proto::Payload) -> Option<&String> {
        if let Some(meta) = &payload.metadata {
            return Some(&meta.r#type);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::model::privilege::PrivilegeGroup;
    use std::collections::HashSet;

    fn build_meta(
        roles: Vec<&str>,
        namespace_privilege: Option<PrivilegeGroup<Arc<String>>>,
    ) -> RequestMeta {
        RequestMeta {
            token_session: Some(Arc::new(TokenSession {
                username: Arc::new("test_user".to_owned()),
                roles: roles.into_iter().map(|r| Arc::new(r.to_owned())).collect(),
                namespace_privilege,
                extend_infos: HashMap::new(),
            })),
            ..Default::default()
        }
    }

    #[test]
    fn test_request_meta_privilege_helper() {
        // 无会话(鉴权关闭): 全量权限 + 可编辑
        let meta = RequestMeta::default();
        assert!(meta.namespace_privilege().is_all());
        assert!(meta.can_edit());

        // 有会话 Visitor + 全量权限: 不可编辑
        let meta = build_meta(vec!["2"], Some(PrivilegeGroup::all()));
        assert!(meta.namespace_privilege().is_all());
        assert!(!meta.can_edit());

        // 有会话 Developer + 无权限(empty): 不可访问任何命名空间, 可编辑
        let meta = build_meta(vec!["1"], Some(PrivilegeGroup::empty()));
        let group = meta.namespace_privilege();
        assert!(!group.is_all());
        assert!(!group.check_permission(&Arc::new("public".to_owned())));
        assert!(meta.can_edit());

        // 有会话 Developer + 白名单空串(public): 可编辑且仅 public 可访问
        let mut whitelist = HashSet::new();
        whitelist.insert(Arc::new("".to_owned()));
        let group = PrivilegeGroup {
            enabled: true,
            whitelist_is_all: false,
            whitelist: Some(Arc::new(whitelist)),
            blacklist_is_all: false,
            blacklist: None,
        };
        let meta = build_meta(vec!["1"], Some(group));
        let group = meta.namespace_privilege();
        assert!(group.check_permission(&Arc::new("public".to_owned())));
        assert!(!group.check_permission(&Arc::new("ns-x".to_owned())));
        assert!(meta.can_edit());
    }
}
