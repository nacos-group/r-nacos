pub mod client_version;
pub mod privilege;

use std::{collections::HashMap, sync::Arc};

pub use crate::common::model::client_version::ClientVersion;
use crate::common::model::privilege::PrivilegeGroup;
use crate::user::permission::UserRole;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ApiResultOld<T>
where
    T: Sized + Default,
{
    pub data: Option<T>,
    pub success: bool,
    pub code: Option<String>,
    pub msg: Option<String>,
}

impl<T> ApiResultOld<T>
where
    T: Sized + Default,
{
    pub fn success(data: Option<T>) -> Self {
        Self {
            data,
            success: true,
            code: None,
            msg: None,
        }
    }

    pub fn error(code: String, msg: Option<String>) -> Self {
        Self {
            data: None,
            success: false,
            code: Some(code),
            msg,
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct PageResultOld<T> {
    pub size: usize,
    pub list: Vec<T>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ApiResult<T>
where
    T: Sized + Default,
{
    pub data: Option<T>,
    pub success: bool,
    pub code: Option<String>,
    pub message: Option<String>,
}

impl<T> crate::common::model::ApiResult<T>
where
    T: Sized + Default,
{
    pub fn success(data: Option<T>) -> Self {
        Self {
            data,
            success: true,
            code: None,
            message: None,
        }
    }

    pub fn error(code: String, message: Option<String>) -> Self {
        Self {
            data: None,
            success: false,
            code: Some(code),
            message,
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageResult<T> {
    pub total_count: usize,
    pub list: Vec<T>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct UserSession {
    pub username: Arc<String>,
    pub nickname: Option<String>,
    pub roles: Vec<Arc<String>>,
    pub namespace_privilege: Option<PrivilegeGroup<Arc<String>>>,
    pub extend_infos: HashMap<String, String>,
    /// 时间戳，单位秒
    pub refresh_time: u32,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct TokenSession {
    pub username: Arc<String>,
    pub roles: Vec<Arc<String>>,
    /// 命名空间数据权限; 历史缓存数据缺失该字段时反序列化为 None(视为全量权限)
    pub namespace_privilege: Option<PrivilegeGroup<Arc<String>>>,
    pub extend_infos: HashMap<String, String>,
}

impl TokenSession {
    /// 是否可编辑(增、删、改)数据: roles 包含 Developer 或 Manager 角色
    pub fn can_edit(&self) -> bool {
        self.roles
            .iter()
            .any(|r| UserRole::is_edit_role(r.as_str()))
    }

    /// 命名空间数据权限组; None(历史数据)视为全量权限
    pub fn namespace_privilege_group(
        &self,
    ) -> crate::common::model::privilege::NamespacePrivilegeGroup {
        self.namespace_privilege
            .clone()
            .map(crate::common::model::privilege::NamespacePrivilegeGroup::new)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_session(
        roles: Vec<&str>,
        namespace_privilege: Option<PrivilegeGroup<Arc<String>>>,
    ) -> TokenSession {
        TokenSession {
            username: Arc::new("test_user".to_owned()),
            roles: roles.into_iter().map(|r| Arc::new(r.to_owned())).collect(),
            namespace_privilege,
            extend_infos: HashMap::new(),
        }
    }

    #[test]
    fn test_can_edit_role_matrix() {
        // Manager 可编辑
        assert!(build_session(vec!["0"], None).can_edit());
        // Developer 可编辑
        assert!(build_session(vec!["1"], None).can_edit());
        // Visitor 不可编辑
        assert!(!build_session(vec!["2"], None).can_edit());
        // 多角色包含 Developer 或 Manager 即可编辑
        assert!(build_session(vec!["2", "1"], None).can_edit());
        assert!(build_session(vec!["2", "0"], None).can_edit());
        // 未知角色不可编辑
        assert!(!build_session(vec!["9"], None).can_edit());
        // 无角色不可编辑
        assert!(!build_session(vec![], None).can_edit());
    }

    #[test]
    fn test_namespace_privilege_group_and_history_json_compatible() {
        // 无权限字段历史 JSON(存量 ApiTokenSession 缓存)反序列化为 None, 且权限组为全量
        let json = r#"{"username":"dev1","roles":["1"],"extend_infos":{}}"#;
        let session: TokenSession = serde_json::from_str(json).unwrap();
        assert!(session.namespace_privilege.is_none());
        let group = session.namespace_privilege_group();
        assert!(group.is_all());

        // Some(all) 权限组
        let session = build_session(vec!["1"], Some(PrivilegeGroup::all()));
        assert!(session.namespace_privilege_group().is_all());

        // Some(empty) 表示无任何命名空间权限
        let session = build_session(vec!["1"], Some(PrivilegeGroup::empty()));
        assert!(!session.namespace_privilege_group().is_all());
        assert!(!session
            .namespace_privilege_group()
            .check_permission(&Arc::new("public".to_owned())));

        // 白名单权限组校验(默认命名空间在权限组中存储为空串)
        let mut whitelist = std::collections::HashSet::new();
        whitelist.insert(Arc::new("".to_owned()));
        let group = PrivilegeGroup {
            enabled: true,
            whitelist_is_all: false,
            whitelist: Some(Arc::new(whitelist)),
            blacklist_is_all: false,
            blacklist: None,
        };
        let session = build_session(vec!["1"], Some(group));
        let ns_group = session.namespace_privilege_group();
        assert!(!ns_group.is_all());
        // "public" 与空串都按默认命名空间归一化校验
        assert!(ns_group.check_permission(&Arc::new("public".to_owned())));
        assert!(ns_group.check_permission(&Arc::new("".to_owned())));
        assert!(!ns_group.check_permission(&Arc::new("ns-x".to_owned())));
    }
}
