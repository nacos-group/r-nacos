use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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

///
/// 数据权限组
/// 支持分别设置黑白名单
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PrivilegeGroup<T>
where
    T: Sized + std::hash::Hash + std::cmp::Eq,
{
    pub enabled: bool,
    pub white_list_is_all: bool,
    pub whitelist: Option<HashSet<T>>,
    pub black_list_is_all: bool,
    pub blacklist: Option<HashSet<T>>,
}

impl<T> PrivilegeGroup<T>
where
    T: Sized + std::hash::Hash + std::cmp::Eq,
{
    pub fn new(
        flags: u8,
        whitelist: Option<HashSet<T>>,
        blacklist: Option<HashSet<T>>,
    ) -> PrivilegeGroup<T> {
        let enabled = flags & PrivilegeGroupFlags::ENABLE.bits() > 0;
        let white_list_is_all = flags & PrivilegeGroupFlags::WHILE_LIST_IS_ALL.bits() > 0;
        let black_list_is_all = flags & PrivilegeGroupFlags::BLACK_LIST_IS_ALL.bits() > 0;
        Self {
            enabled,
            white_list_is_all,
            black_list_is_all,
            whitelist,
            blacklist,
        }
    }

    pub fn empty() -> Self {
        Self {
            enabled: true,
            white_list_is_all: false,
            whitelist: None,
            black_list_is_all: false,
            blacklist: None,
        }
    }

    pub fn all() -> Self {
        Self {
            enabled: true,
            white_list_is_all: true,
            whitelist: None,
            black_list_is_all: false,
            blacklist: None,
        }
    }

    pub fn get_flags(&self) -> u8 {
        let mut v = 0;
        if self.enabled {
            v |= PrivilegeGroupFlags::ENABLE.bits();
        }
        if self.white_list_is_all {
            v |= PrivilegeGroupFlags::WHILE_LIST_IS_ALL.bits();
        }
        if self.black_list_is_all {
            v |= PrivilegeGroupFlags::BLACK_LIST_IS_ALL.bits();
        }
        v
    }

    pub fn set_flags(&mut self, flags: u8) {
        self.enabled = flags & PrivilegeGroupFlags::ENABLE.bits() > 0;
        self.white_list_is_all = flags & PrivilegeGroupFlags::WHILE_LIST_IS_ALL.bits() > 0;
        self.black_list_is_all = flags & PrivilegeGroupFlags::BLACK_LIST_IS_ALL.bits() > 0;
    }
}
