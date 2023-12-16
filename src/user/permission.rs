/// 权限控制
/// 用户-> 角色 -> 功能模块 -> 权限资源； 从前到后都是一对多；
/// 权限资源分为两类：
/// 1）web资源，由前端控制页面是否支持访问； 
/// 2）http请求路径，由后端拦截器控制否支持请求；

use std::{sync::Arc, collections::HashSet, hash::Hash};

use crate::common::constant::{EMPTY_STR, HTTP_METHOD_GET, HTTP_METHOD_ALL};

pub enum Resource {
    WebResource(&'static str),
    Path( &'static str, &'static str),
}

#[derive(Debug,PartialEq, Eq, Hash)]
pub struct PathResource{
    pub path: &'static str,
    pub method: &'static str,
}

impl PathResource {
    pub fn match_url(&self,path:&str,method:&str) -> bool {
        let match_method = self.is_match_all_method() || self.method == method;
        if path.is_empty() {
            match_method && (self.is_match_all_path() || self.path=="/")
        }
        else{
            match_method && (self.is_match_all_path() || self.path==path)
        }
    }

    pub fn is_match_all_path(&self) -> bool {
        self.path==EMPTY_STR
    }
    pub fn is_match_all_method(&self) -> bool {
        self.method==HTTP_METHOD_ALL 
    }
}
pub struct ModuleResource{
    pub web_resources: HashSet<&'static str>,
    pub path_resources: HashSet<PathResource>,
}

impl ModuleResource {
    pub fn new(resources:Vec<Resource>) -> Self {
        let mut web_resources = HashSet::new();
        let mut path_resources = HashSet::new();
        for item in resources {
            match item {
                Resource::WebResource(r) => {
                    web_resources.insert(r);
                },
                Resource::Path(path, method) => {
                    path_resources.insert(PathResource{
                        path,
                        method
                    });
                },
            }
        }
        Self {
            web_resources,
            path_resources
        }
    }

    pub fn match_url(&self,path:&str,method:&str) -> bool {
        for item in &self.path_resources {
            if item.match_url(path, method){
                return true
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
    static ref M_BASE: Arc<ModuleResource> = Arc::new(ModuleResource::new(vec![
        //WebResource
        R::WebResource("/404"),
        R::WebResource("/p/login"),
        //path
        R::Path("/404",HTTP_METHOD_ALL),
        R::Path("/p/login",HTTP_METHOD_ALL),
        R::Path("/nacos/v1/console/login/login",HTTP_METHOD_ALL),
        R::Path("/nacos/v1/console/login/captcha",HTTP_METHOD_ALL),
        R::Path("/nacos/v1/console/namespaces",HTTP_METHOD_GET),
        R::Path("/nacos/v1/console/user/info",HTTP_METHOD_GET),
    ]));

    static ref M_CLUSTER_VISITOR: Arc<ModuleResource> = Arc::new(ModuleResource::new(vec![
        //WebResource
        R::WebResource("/manage/cluster"),
        //path
        R::Path("/manage/cluster",HTTP_METHOD_GET),
        R::Path("/nacos/v1/console/cluster/cluster_node_list",HTTP_METHOD_GET),
    ]));

    static ref M_NAMESPACE_VISITOR: Arc<ModuleResource> = Arc::new(ModuleResource::new(vec![
        //WebResource
        R::WebResource("/manage/namespace"),
        //path
        R::Path("/manage/namespace",HTTP_METHOD_GET),
        R::Path("/nacos/v1/console/namespaces",HTTP_METHOD_GET),
    ]));

    static ref M_NAMESPACE_MANAGE: Arc<ModuleResource> = Arc::new(ModuleResource::new(vec![
        //WebResource
        R::WebResource("/manage/namespace"),
        R::WebResource("NAMESPACE_UPDATE"),
        //path
        R::Path("/manage/namespace",HTTP_METHOD_GET),
        R::Path("/nacos/v1/console/namespaces",HTTP_METHOD_ALL),
    ]));

    static ref M_USER_MANAGE: Arc<ModuleResource> = Arc::new(ModuleResource::new(vec![
        //WebResource
        R::WebResource("/manage/user"),
        R::WebResource("USER_UPDATE"),
        //path
        R::Path("/manage/user",HTTP_METHOD_GET),
        R::Path("/nacos/v1/console/user/list",HTTP_METHOD_GET),
        R::Path("/nacos/v1/console/user/info",HTTP_METHOD_GET),
        R::Path("/nacos/v1/console/user/add",HTTP_METHOD_ALL),
        R::Path("/nacos/v1/console/user/update",HTTP_METHOD_ALL),
        R::Path("/nacos/v1/console/user/remove",HTTP_METHOD_ALL),
    ]));

    static ref M_CONFIG_VISITOR: Arc<ModuleResource> = Arc::new(ModuleResource::new(vec![
        //WebResource
        R::WebResource("/manage/configs"),
        R::WebResource("/manage/config/history"),
        //path
        R::Path("/manage/configs",HTTP_METHOD_GET),
        R::Path("/nacos/v1/console/configs",HTTP_METHOD_GET),
        R::Path("/nacos/v1/console/config/download",HTTP_METHOD_GET),
        R::Path("/nacos/v1/cs/configs",HTTP_METHOD_GET),
        //config history
        R::Path("/manage/config/history",HTTP_METHOD_GET),
        R::Path("/nacos/v1/console/config/history",HTTP_METHOD_GET),
    ]));

    static ref M_CONFIG_MANAGE: Arc<ModuleResource> = Arc::new(ModuleResource::new(vec![
        //WebResource
        R::WebResource("/manage/configs"),
        R::WebResource("/manage/config/history"),
        R::WebResource("CONFIG_UPDATE"),
        //path 
        R::Path("/nacos/v1/console/namespaces",HTTP_METHOD_ALL),
        R::Path("/manage/configs",HTTP_METHOD_ALL),
        R::Path("/nacos/v1/console/configs",HTTP_METHOD_GET),
        R::Path("/nacos/v1/console/config/download",HTTP_METHOD_GET),
        R::Path("/nacos/v1/console/config/import",HTTP_METHOD_ALL),
        R::Path("/nacos/v1/cs/configs",HTTP_METHOD_ALL),
        //config history
        R::Path("/manage/config/history",HTTP_METHOD_GET),
        R::Path("/nacos/v1/console/config/history",HTTP_METHOD_GET),
    ]));

    static ref M_NAMING_VISITOR: Arc<ModuleResource> = Arc::new(ModuleResource::new(vec![
        //WebResource
        R::WebResource("/manage/service"),
        R::WebResource("/manage/service/instance"),
        //path
        R::Path("/manage/service",HTTP_METHOD_GET),
        R::Path("/nacos/v1/ns/catalog/services",HTTP_METHOD_GET),
        R::Path("/nacos/v1/ns/service",HTTP_METHOD_GET),
        //instance 
        R::Path("/manage/service/instance",HTTP_METHOD_GET),
        R::Path("/nacos/v1/console/instances",HTTP_METHOD_GET),
        R::Path("/nacos/v1/ns/instance",HTTP_METHOD_GET),
    ]));

    static ref M_NAMING_MANAGE: Arc<ModuleResource> = Arc::new(ModuleResource::new(vec![
        //WebResource
        R::WebResource("/manage/service"),
        R::WebResource("/manage/service/instance"),
        //path
        R::Path("/manage/service",HTTP_METHOD_GET),
        R::Path("/nacos/v1/ns/catalog/services",HTTP_METHOD_GET),
        R::Path("/nacos/v1/ns/service",HTTP_METHOD_ALL),
        //instance 
        R::Path("/manage/service/instance",HTTP_METHOD_GET),
        R::Path("/nacos/v1/console/instances",HTTP_METHOD_GET),
        R::Path("/nacos/v1/ns/instance",HTTP_METHOD_ALL),
    ]));
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

    /* 
    pub fn get_web_resource(role_value:&str) -> Vec<Arc<String>> {
        todo!()
    }
    */
}

