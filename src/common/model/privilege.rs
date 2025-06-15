use crate::common::constant::DEFAULT_NAMESPACE_ARC_STRING;
use crate::namespace::is_default_namespace;
use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;

bitflags! {
    /// Represents a set of flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct PrivilegeGroupFlags: u8 {
        /// The value `ENABLE`, at bit position `0`.
        const ENABLE = 0b00000001;
        /// The value `WHILE_LIST_IS_ALL`, at bit position `1`.
        const WHILE_LIST_IS_ALL = 0b00000010;
        /// The value `BLACK_LIST_IS_ALL`, at bit position `2`.
        const BLACK_LIST_IS_ALL= 0b00000100;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PrivilegeGroupOptionParam<T>
where
    T: Sized + std::hash::Hash + std::cmp::Eq,
{
    pub whitelist_is_all: Option<bool>,
    pub whitelist: Option<Arc<HashSet<T>>>,
    pub blacklist_is_all: Option<bool>,
    pub blacklist: Option<Arc<HashSet<T>>>,
}

impl<T> PrivilegeGroupOptionParam<T>
where
    T: Sized + std::hash::Hash + std::cmp::Eq,
{
    pub fn is_none(&self) -> bool {
        self.whitelist_is_all.is_none()
            && self.whitelist.is_none()
            && self.blacklist_is_all.is_none()
            && self.blacklist.is_none()
    }
}

///
/// 数据权限组
/// 支持分别设置黑白名单
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivilegeGroup<T>
where
    T: Sized + std::hash::Hash + std::cmp::Eq,
{
    pub enabled: bool,
    pub whitelist_is_all: bool,
    pub whitelist: Option<Arc<HashSet<T>>>,
    pub blacklist_is_all: bool,
    pub blacklist: Option<Arc<HashSet<T>>>,
}

impl<T> Default for PrivilegeGroup<T>
where
    T: Sized + std::hash::Hash + std::cmp::Eq,
{
    fn default() -> Self {
        Self::all()
    }
}

impl<T> PrivilegeGroup<T>
where
    T: Sized + std::hash::Hash + std::cmp::Eq,
{
    pub fn new(
        flags: u8,
        whitelist: Option<Arc<HashSet<T>>>,
        blacklist: Option<Arc<HashSet<T>>>,
    ) -> PrivilegeGroup<T> {
        let enabled = flags & PrivilegeGroupFlags::ENABLE.bits() > 0;
        let white_list_is_all = flags & PrivilegeGroupFlags::WHILE_LIST_IS_ALL.bits() > 0;
        let black_list_is_all = flags & PrivilegeGroupFlags::BLACK_LIST_IS_ALL.bits() > 0;
        Self {
            enabled,
            whitelist_is_all: white_list_is_all,
            blacklist_is_all: black_list_is_all,
            whitelist,
            blacklist,
        }
    }

    pub fn empty() -> Self {
        Self {
            enabled: true,
            whitelist_is_all: false,
            whitelist: None,
            blacklist_is_all: false,
            blacklist: None,
        }
    }

    pub fn all() -> Self {
        Self {
            enabled: true,
            whitelist_is_all: true,
            whitelist: None,
            blacklist_is_all: false,
            blacklist: None,
        }
    }

    pub fn is_all(&self) -> bool {
        self.enabled && self.whitelist_is_all && self.blacklist_is_empty()
    }

    fn blacklist_is_empty(&self) -> bool {
        if self.blacklist_is_all {
            return false;
        }
        if let Some(blacklist) = &self.blacklist {
            blacklist.is_empty()
        } else {
            true
        }
    }

    pub fn get_flags(&self) -> u8 {
        let mut v = 0;
        if self.enabled {
            v |= PrivilegeGroupFlags::ENABLE.bits();
        }
        if self.whitelist_is_all {
            v |= PrivilegeGroupFlags::WHILE_LIST_IS_ALL.bits();
        }
        if self.blacklist_is_all {
            v |= PrivilegeGroupFlags::BLACK_LIST_IS_ALL.bits();
        }
        v
    }

    pub fn set_flags(&mut self, flags: u8) {
        self.enabled = flags & PrivilegeGroupFlags::ENABLE.bits() > 0;
        self.whitelist_is_all = flags & PrivilegeGroupFlags::WHILE_LIST_IS_ALL.bits() > 0;
        self.blacklist_is_all = flags & PrivilegeGroupFlags::BLACK_LIST_IS_ALL.bits() > 0;
    }

    pub fn check_permission(&self, key: &T) -> bool {
        self.at_whitelist(key) && !self.at_blacklist(key)
    }

    pub fn check_option_value_permission(&self, key: &Option<T>, empty_default: bool) -> bool {
        if let Some(key) = key {
            self.at_whitelist(key) && !self.at_blacklist(key)
        } else {
            empty_default
        }
    }

    fn at_whitelist(&self, key: &T) -> bool {
        if self.whitelist_is_all {
            return true;
        }
        if let Some(list) = &self.whitelist {
            list.contains(key)
        } else {
            false
        }
    }

    fn at_blacklist(&self, key: &T) -> bool {
        if self.blacklist_is_all {
            return true;
        }
        if let Some(list) = &self.blacklist {
            list.contains(key)
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NamespacePrivilegeGroup(pub PrivilegeGroup<Arc<String>>);

impl NamespacePrivilegeGroup {
    pub fn new(inner: PrivilegeGroup<Arc<String>>) -> Self {
        Self(inner)
    }

    pub fn check_permission(&self, key: &Arc<String>) -> bool {
        if is_default_namespace(key.as_str()) {
            self.0.check_permission(&DEFAULT_NAMESPACE_ARC_STRING)
        } else {
            self.0.check_permission(key)
        }
    }

    pub fn check_option_value_permission(
        &self,
        key: &Option<Arc<String>>,
        empty_default: bool,
    ) -> bool {
        if let Some(key) = key {
            self.check_permission(key)
        } else {
            empty_default
        }
    }

    pub fn is_all(&self) -> bool {
        self.0.is_all()
    }
}
