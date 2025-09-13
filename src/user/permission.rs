/// 权限控制
/// 用户-> 角色 -> 功能模块 -> 权限资源； 从前到后都是一对多；
/// 权限资源分为两类：
/// 1）web资源，由前端控制页面是否支持访问；
/// 2）http请求路径，由后端拦截器控制否支持请求；
use std::{collections::HashSet, hash::Hash, sync::Arc};

use crate::common::constant::{EMPTY_STR, HTTP_METHOD_ALL, HTTP_METHOD_GET, HTTP_METHOD_POST};

pub enum Resource {
    WebResource(&'static str),
    Path(&'static str, &'static str),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct PathResource {
    pub path: &'static str,
    pub method: &'static str,
}

impl PathResource {
    pub fn match_url(&self, path: &str, method: &str) -> bool {
        let match_method = self.is_match_all_method() || self.method == method;
        if path.is_empty() {
            match_method && (self.is_match_all_path() || self.path == "/")
        } else {
            match_method && (self.is_match_all_path() || self.path == path)
        }
    }

    pub fn is_match_all_path(&self) -> bool {
        self.path == EMPTY_STR
    }
    pub fn is_match_all_method(&self) -> bool {
        self.method == HTTP_METHOD_ALL
    }
}
pub struct ModuleResource {
    pub web_resources: HashSet<&'static str>,
    pub path_resources: HashSet<PathResource>,
}

impl ModuleResource {
    pub fn new(resources: Vec<Resource>) -> Self {
        let mut web_resources = HashSet::new();
        let mut path_resources = HashSet::new();
        for item in resources {
            match item {
                Resource::WebResource(r) => {
                    web_resources.insert(r);
                }
                Resource::Path(path, method) => {
                    path_resources.insert(PathResource { path, method });
                }
            }
        }
        Self {
            web_resources,
            path_resources,
        }
    }

    pub fn match_url(&self, path: &str, method: &str) -> bool {
        for item in &self.path_resources {
            if item.match_url(path, method) {
                return true;
            }
        }
        false
    }
}

pub struct GroupResource {
    pub web_resources: HashSet<&'static str>,
    pub path_resources: HashSet<PathResource>,
}

impl GroupResource {
    pub fn new(module_resources: Vec<&ModuleResource>) -> Self {
        let mut web_resources = HashSet::new();
        let mut path_resources = HashSet::new();
        for module in module_resources {
            for item in &module.web_resources {
                web_resources.insert(*item);
            }
            for item in &module.path_resources {
                path_resources.insert(PathResource { ..(*item) });
            }
        }
        Self {
            web_resources,
            path_resources,
        }
    }

    pub fn match_url(&self, path: &str, method: &str) -> bool {
        for item in &self.path_resources {
            if item.match_url(path, method) {
                return true;
            }
        }
        false
    }
}

type R = Resource;

lazy_static::lazy_static! {
    pub(crate) static ref USER_ROLE_MANAGER: Arc<String> =  Arc::new("0".to_string());
    pub(crate) static ref USER_ROLE_DEVELOPER: Arc<String> =  Arc::new("1".to_string());
    pub(crate) static ref USER_ROLE_VISITOR: Arc<String> =  Arc::new("2".to_string());
    pub(crate) static ref ALL_ROLES: Vec<Arc<String>> = vec![USER_ROLE_MANAGER.clone(),USER_ROLE_DEVELOPER.clone(),USER_ROLE_VISITOR.clone()];
    static ref M_BASE: ModuleResource = ModuleResource::new(vec![
        //WebResource
        R::WebResource("/"),
        R::WebResource("/404"),
        R::WebResource("/nopermission"),
        R::WebResource("/p/login"),
        R::WebResource("/manage/about"),
        R::WebResource("/rnacos"),
        R::WebResource("/rnacos/"),
        R::WebResource("/rnacos/404"),
        R::WebResource("/rnacos/nopermission"),
        R::WebResource("/rnacos/p/login"),
        R::WebResource("/rnacos/manage/about"),
        //path
        R::Path("/",HTTP_METHOD_GET),
        R::Path("/404",HTTP_METHOD_GET),
        R::Path("/nopermission",HTTP_METHOD_GET),
        R::Path("/p/login",HTTP_METHOD_GET),
        R::Path("/manage/about",HTTP_METHOD_GET),
        R::Path("/rnacos",HTTP_METHOD_GET),
        R::Path("/rnacos/",HTTP_METHOD_GET),
        R::Path("/rnacos/404",HTTP_METHOD_GET),
        R::Path("/rnacos/nopermission",HTTP_METHOD_GET),
        R::Path("/rnacos/p/login",HTTP_METHOD_GET),
        R::Path("/rnacos/manage/about",HTTP_METHOD_GET),

        R::Path("/rnacos/api/console/login/login",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/login/captcha",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/login/logout",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/namespaces",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/user/info",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/user/web_resources",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/user/reset_password",HTTP_METHOD_ALL),

        R::Path("/rnacos/api/console/v2/login/login",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/v2/login/captcha",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/v2/login/logout",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/v2/user/info",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/user/web_resources",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/user/reset_password",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/v2/namespaces/list",HTTP_METHOD_GET),

    ]);

    static ref M_CLUSTER_VISITOR: ModuleResource = ModuleResource::new(vec![
        //WebResource
        R::WebResource("/manage/cluster"),
        R::WebResource("/rnacos/manage/cluster"),
        //path
        R::Path("/rnacos/manage/cluster",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/cluster/cluster_node_list",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/cluster/cluster_node_list",HTTP_METHOD_GET),
    ]);

    static ref M_NAMESPACE_VISITOR: ModuleResource = ModuleResource::new(vec![
        //WebResource
        R::WebResource("/manage/namespace"),
        R::WebResource("/rnacos/manage/namespace"),
        //path
        R::Path("/rnacos/manage/namespace",HTTP_METHOD_GET),
        //R::Path("/rnacos/api/console/namespaces",HTTP_METHOD_GET),
    ]);

    static ref M_NAMESPACE_MANAGE: ModuleResource = ModuleResource::new(vec![
        //WebResource
        R::WebResource("/manage/namespace"),
        R::WebResource("/rnacos/manage/namespace"),
        R::WebResource("NAMESPACE_UPDATE"),
        //path
        R::Path("/rnacos/manage/namespace",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/namespaces",HTTP_METHOD_ALL),

        R::Path("/rnacos/api/console/v2/namespaces/add",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/v2/namespaces/update",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/v2/namespaces/remove",HTTP_METHOD_ALL),
    ]);

    static ref M_USER_MANAGE: ModuleResource = ModuleResource::new(vec![
        //WebResource
        R::WebResource("/manage/user"),
        R::WebResource("/rnacos/manage/user"),
        R::WebResource("USER_UPDATE"),
        //path
        R::Path("/rnacos/manage/user",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/user/list",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/user/info",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/user/add",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/user/update",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/user/remove",HTTP_METHOD_ALL),

        R::Path("/rnacos/api/console/v2/user/list",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/user/info",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/user/add",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/v2/user/update",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/v2/user/remove",HTTP_METHOD_ALL),
    ]);

    static ref M_CONFIG_VISITOR: ModuleResource = ModuleResource::new(vec![
        //WebResource
        R::WebResource("/manage/configs"),
        R::WebResource("/manage/config/history"),
        R::WebResource("/rnacos/manage/configs"),
        R::WebResource("/rnacos/manage/config/history"),
        //path
        R::Path("/rnacos/manage/configs",HTTP_METHOD_GET),
        R::Path("/rnacos/manage/config/history",HTTP_METHOD_GET),

        R::Path("/rnacos/api/console/configs",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/download",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/cs/configs",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/config/history",HTTP_METHOD_GET),

        R::Path("/rnacos/api/console/v2/config/list",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/config/download",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/config/info",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/config/history",HTTP_METHOD_GET),
    ]);

    static ref M_CONFIG_MANAGE: ModuleResource = ModuleResource::new(vec![
        //WebResource
        R::WebResource("/manage/configs"),
        R::WebResource("/manage/config/history"),
        R::WebResource("/rnacos/manage/configs"),
        R::WebResource("/rnacos/manage/config/history"),
        R::WebResource("CONFIG_UPDATE"),
        //path
        R::Path("/rnacos/manage/configs",HTTP_METHOD_ALL),
        R::Path("/rnacos/manage/config/history",HTTP_METHOD_GET),

        R::Path("/rnacos/api/console/configs",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/config/download",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/config/download",HTTP_METHOD_POST),
        R::Path("/rnacos/api/console/config/import",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/cs/configs",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/config/history",HTTP_METHOD_GET),

        R::Path("/rnacos/api/console/v2/config/list",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/config/download",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/config/info",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/config/history",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/config/import",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/v2/config/add",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/v2/config/update",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/v2/config/remove",HTTP_METHOD_ALL),
    ]);

    static ref M_NAMING_VISITOR: ModuleResource = ModuleResource::new(vec![
        //WebResource
        R::WebResource("/manage/service"),
        R::WebResource("/manage/service/instance"),
        R::WebResource("/manage/subscriber"),
        R::WebResource("/rnacos/manage/service"),
        R::WebResource("/rnacos/manage/service/instance"),
        R::WebResource("/rnacos/manage/subscriber"),
        //path
        R::Path("/rnacos/manage/service",HTTP_METHOD_GET),
        R::Path("/rnacos/manage/service/instance",HTTP_METHOD_GET),

        R::Path("/rnacos/api/console/ns/services",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/ns/service",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/ns/service/subscribers",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/instances",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/ns/instance",HTTP_METHOD_GET),

        R::Path("/rnacos/api/console/v2/service/list",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/service/subscriber/list",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/instance/list",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/instance/info",HTTP_METHOD_GET),
        R::Path("/rnacos/manage/subscriber", HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/cluster/cluster_node_list",HTTP_METHOD_GET),
    ]);

    static ref M_NAMING_MANAGE: ModuleResource = ModuleResource::new(vec![
        //WebResource
        R::WebResource("/manage/service"),
        R::WebResource("/manage/service/instance"),
        R::WebResource("/manage/subscriber"),
        R::WebResource("/rnacos/manage/service"),
        R::WebResource("/rnacos/manage/service/instance"),
        R::WebResource("/rnacos/manage/subscriber"),
        R::WebResource("SERVICE_UPDATE"),
        //path
        R::Path("/rnacos/manage/service",HTTP_METHOD_GET),
        R::Path("/rnacos/manage/service/instance",HTTP_METHOD_GET),
        R::Path("/rnacos/manage/subscriber",HTTP_METHOD_GET),

        R::Path("/rnacos/api/console/ns/services",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/ns/service",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/ns/service/subscribers",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/instances",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/ns/instance",HTTP_METHOD_ALL),

        R::Path("/rnacos/api/console/v2/service/list",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/service/subscriber/list",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/service/add",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/v2/service/update",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/v2/service/remove",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/v2/instance/list",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/instance/info",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/instance/add",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/v2/instance/update",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/v2/instance/remove",HTTP_METHOD_ALL),
    ]);

    static ref M_METRICS_VISITOR: ModuleResource = ModuleResource::new(vec![
        //WebResource
        R::WebResource("/manage/appmonitor"),
        R::WebResource("/rnacos/manage/appmonitor"),
        //path
        R::Path("/rnacos/manage/appmonitor",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/metrics/timeline",HTTP_METHOD_ALL),
        R::Path("/rnacos/api/console/cluster/cluster_node_list",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/cluster/cluster_node_list",HTTP_METHOD_GET),
    ]);

    static ref M_TRASFER_DATE_MANAGE: ModuleResource = ModuleResource::new(vec![
        //WebResource
        R::WebResource("/manage/transfer"),
        R::WebResource("/rnacos/manage/transfer"),
        R::WebResource("/rnacos/api/console/transfer/export"),
        R::WebResource("/rnacos/api/console/transfer/import"),
        //path
        R::Path("/rnacos/manage/transfer",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/transfer/export",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/transfer/import",HTTP_METHOD_ALL),
    ]);

    static ref M_MCP_TOOL_SPEC_VISITOR: ModuleResource = ModuleResource::new(vec![
        //WebResource
        R::WebResource("/manage/mcptoolspec"),
        R::WebResource("/manage/mcptoolspec/detail"),
        R::WebResource("/rnacos/manage/mcptoolspec"),
        R::WebResource("/rnacos/manage/mcptoolspec/detail"),
        //path
        R::Path("/rnacos/manage/mcptoolspec",HTTP_METHOD_GET),
        R::Path("/rnacos/manage/mcptoolspec/detail",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/mcp/toolspec/list",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/mcp/toolspec/info",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/mcp/toolspec/download",HTTP_METHOD_GET),
    ]);

    static ref M_MCP_TOOL_SPEC_MANAGE: ModuleResource = ModuleResource::new(vec![
        //WebResource
        R::WebResource("/manage/mcptoolspec"),
        R::WebResource("/manage/mcptoolspec/detail"),
        R::WebResource("/rnacos/manage/mcptoolspec"),
        R::WebResource("/rnacos/manage/mcptoolspec/detail"),
        R::WebResource("MCP_TOOL_SPEC_UPDATE"),
        //path
        R::Path("/rnacos/manage/mcptoolspec",HTTP_METHOD_GET),
        R::Path("/rnacos/manage/mcptoolspec/detail",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/mcp/toolspec/list",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/mcp/toolspec/info",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/mcp/toolspec/add",HTTP_METHOD_POST),
        R::Path("/rnacos/api/console/v2/mcp/toolspec/update",HTTP_METHOD_POST),
        R::Path("/rnacos/api/console/v2/mcp/toolspec/batch_update",HTTP_METHOD_POST),
        R::Path("/rnacos/api/console/v2/mcp/toolspec/remove",HTTP_METHOD_POST),
        R::Path("/rnacos/api/console/v2/mcp/toolspec/download",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/mcp/toolspec/import",HTTP_METHOD_POST),
    ]);

    static ref M_MCP_SERVER_VISITOR: ModuleResource = ModuleResource::new(vec![
        //WebResource
        R::WebResource("/manage/mcpserver"),
        R::WebResource("/rnacos/manage/mcpserver"),
        //path
        R::Path("/rnacos/manage/mcpserver",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/mcp/server/list",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/mcp/server/info",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/mcp/server/history",HTTP_METHOD_GET),
    ]);

    static ref M_MCP_SERVER_MANAGE: ModuleResource = ModuleResource::new(vec![
        //WebResource
        R::WebResource("/manage/mcpserver"),
        R::WebResource("/rnacos/manage/mcpserver"),
        R::WebResource("MCP_SERVER_UPDATE"),
        //path
        R::Path("/rnacos/manage/mcpserver",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/mcp/server/list",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/mcp/server/info",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/mcp/server/history",HTTP_METHOD_GET),
        R::Path("/rnacos/api/console/v2/mcp/server/add",HTTP_METHOD_POST),
        R::Path("/rnacos/api/console/v2/mcp/server/update",HTTP_METHOD_POST),
        R::Path("/rnacos/api/console/v2/mcp/server/remove",HTTP_METHOD_POST),
        R::Path("/rnacos/api/console/v2/mcp/server/publish",HTTP_METHOD_POST),
        R::Path("/rnacos/api/console/v2/mcp/server/publish/history",HTTP_METHOD_POST),
    ]);

    static ref R_VISITOR: Arc<GroupResource> = Arc::new(GroupResource::new(vec![
        &M_BASE,
        //&M_CLUSTER_VISITOR,
        //&M_NAMESPACE_VISITOR,
        &M_CONFIG_VISITOR,
        &M_NAMING_VISITOR,
        &M_MCP_TOOL_SPEC_VISITOR,
        &M_MCP_SERVER_VISITOR,
    ]));

    static ref R_DEVELOPER: Arc<GroupResource> = Arc::new(GroupResource::new(vec![
        &M_BASE,
        &M_CLUSTER_VISITOR,
        &M_NAMESPACE_MANAGE,
        &M_CONFIG_MANAGE,
        &M_NAMING_MANAGE,
        &M_METRICS_VISITOR,
        &M_MCP_TOOL_SPEC_MANAGE,
        &M_MCP_SERVER_MANAGE,
    ]));

    static ref R_MANAGER: Arc<GroupResource> = Arc::new(GroupResource::new(vec![
        &M_BASE,
        &M_CLUSTER_VISITOR,
        &M_NAMESPACE_MANAGE,
        &M_CONFIG_MANAGE,
        &M_NAMING_MANAGE,
        &M_USER_MANAGE,
        &M_METRICS_VISITOR,
        &M_TRASFER_DATE_MANAGE,
        &M_MCP_TOOL_SPEC_MANAGE,
        &M_MCP_SERVER_MANAGE,
    ]));

}

#[derive(Debug)]
pub enum UserRole {
    Visitor,
    Developer,
    Manager,
    OldConsole,
    None,
}
const MANAGER_VALUE: &str = "0";
const DEVELOPER_VALUE: &str = "1";
const VISITOR_VALUE: &str = "2";
const NONE_VALUE: &str = "";

impl UserRole {
    pub fn new(role_value: &str) -> Self {
        match role_value {
            MANAGER_VALUE => Self::Manager,
            DEVELOPER_VALUE => Self::Developer,
            VISITOR_VALUE => Self::Visitor,
            _ => Self::None,
        }
    }

    pub fn to_role_value(&self) -> &str {
        match self {
            Self::Manager => MANAGER_VALUE,
            Self::Developer => DEVELOPER_VALUE,
            Self::Visitor => VISITOR_VALUE,
            _ => NONE_VALUE,
        }
    }

    pub fn get_resources(&self) -> Vec<&GroupResource> {
        match &self {
            UserRole::Visitor => vec![R_VISITOR.as_ref()],
            UserRole::Developer => vec![R_DEVELOPER.as_ref()],
            UserRole::Manager => vec![R_MANAGER.as_ref()],
            //旧控制台使用开发者权限
            UserRole::OldConsole => vec![R_DEVELOPER.as_ref()],
            UserRole::None => vec![],
        }
    }

    pub fn match_url(&self, path: &str, method: &str) -> bool {
        for item in self.get_resources() {
            if item.match_url(path, method) {
                return true;
            }
        }
        false
    }

    pub fn match_url_by_roles(role_values: &Vec<Arc<String>>, path: &str, method: &str) -> bool {
        for item in role_values {
            if Self::new(item.as_str()).match_url(path, method) {
                return true;
            }
        }
        false
    }

    pub fn get_web_resources(&self) -> Vec<&'static str> {
        //log::info!("get_web_resources {:?}", &self);
        let resources = self.get_resources();
        if resources.len() == 1 {
            return resources
                .first()
                .unwrap()
                .web_resources
                .iter()
                .copied()
                .collect();
        }
        let mut set = HashSet::new();
        for resource in resources {
            for item in &resource.web_resources {
                set.insert(*item);
            }
        }
        set.into_iter().collect()
    }

    pub fn get_web_resources_by_roles(role_values: Vec<&str>) -> Vec<&'static str> {
        //log::info!("get_web_resources_by_roles {:?}", &role_values);
        let roles: Vec<Self> = role_values.into_iter().map(Self::new).collect();
        if roles.len() == 1 {
            return roles.first().unwrap().get_web_resources();
        }
        let mut set = HashSet::new();
        for role in roles {
            for resource in role.get_resources() {
                for item in &resource.web_resources {
                    set.insert(*item);
                }
            }
        }
        set.into_iter().collect()
    }
}

pub struct UserRoleHelper;

impl UserRoleHelper {
    pub fn get_all_roles() -> Vec<Arc<String>> {
        ALL_ROLES.clone()
    }

    pub fn get_role(role_value: &str) -> Arc<String> {
        for item in ALL_ROLES.iter() {
            if role_value == item.as_str() {
                return item.clone();
            }
        }
        Arc::new(role_value.to_owned())
    }

    pub fn get_role_by_name(role_name: &str, default: Arc<String>) -> Arc<String> {
        match role_name {
            "VISITOR" => USER_ROLE_VISITOR.clone(),
            "DEVELOPER" => USER_ROLE_DEVELOPER.clone(),
            "ADMIN" => USER_ROLE_MANAGER.clone(),
            "MANAGER" => USER_ROLE_MANAGER.clone(),
            _ => default,
        }
    }
}
