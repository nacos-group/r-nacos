use regex::Regex;
use std::collections::HashSet;
use std::sync::Arc;

pub struct StringUtils;

impl StringUtils {
    pub fn is_empty(s: &str) -> bool {
        s.is_empty()
    }

    pub fn eq(a: &str, b: &str) -> bool {
        a == b
    }

    pub fn like(a: &str, b: &str) -> Option<usize> {
        a.rfind(b)
    }

    pub fn is_option_empty_arc(v: &Option<Arc<String>>) -> bool {
        if let Some(v) = &v {
            v.is_empty()
        } else {
            true
        }
    }

    pub fn is_option_empty(v: &Option<String>) -> bool {
        if let Some(v) = &v {
            v.is_empty()
        } else {
            true
        }
    }

    ///
    /// 空字符串转为None
    pub fn map_not_empty(v: Option<String>) -> Option<String> {
        if let Some(v) = &v {
            if v.is_empty() {
                return None;
            }
        }
        v
    }

    pub fn extract_ldap_value_cn(ldap_str: &str) -> Option<String> {
        let re = Regex::new(r"(?:^|,)cn=([^,]+)").unwrap();
        re.captures(ldap_str)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
    }

    pub fn split_to_hashset(input: &str) -> HashSet<String> {
        input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()) // 过滤空字符串
            .collect() // HashSet自动去重
    }
}
