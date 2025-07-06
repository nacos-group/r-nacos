pub mod param_utils {
    use anyhow::Ok;

    use super::property_util;

    const VALID_CHARS: [char; 4] = ['_', '-', '.', ':'];
    const TENANT_MAX_LEN: usize = 128;

    pub fn check_tenant(tenant: &Option<String>) -> anyhow::Result<()> {
        if let Some(t) = tenant {
            if !t.is_empty() {
                if !is_valid(t.trim()) {
                    return Err(anyhow::anyhow!("invalid tenant"));
                }
                if t.len() > TENANT_MAX_LEN {
                    return Err(anyhow::anyhow!("Too long tenant, over 128"));
                }
            }
        }
        Ok(())
    }
    pub fn check_param(
        data_id: &Option<String>,
        group: &Option<String>,
        datum_id: &Option<String>,
        content: &Option<String>,
    ) -> anyhow::Result<()> {
        match data_id {
            Some(data_id) => {
                if data_id.is_empty() || !is_valid(data_id.trim()) {
                    return Err(anyhow::anyhow!("invalid dataId : {}", data_id));
                }
            }
            None => return Err(anyhow::anyhow!("invalid dataId : ")),
        }
        match group {
            Some(group) => {
                if group.is_empty() || !is_valid(group) {
                    return Err(anyhow::anyhow!("invalid group : {}", group));
                }
            }
            None => return Err(anyhow::anyhow!("invalid group : ")),
        }
        match datum_id {
            Some(datum_id) => {
                if datum_id.is_empty() || !is_valid(datum_id) {
                    return Err(anyhow::anyhow!("invalid datumId : {}", datum_id));
                }
            }
            None => return Err(anyhow::anyhow!("invalid datumId : ")),
        }
        match content {
            Some(content) => {
                if content.is_empty() {
                    return Err(anyhow::anyhow!("content is blank : {}", content));
                } else if content.len() > property_util::get_max_content() {
                    return Err(anyhow::anyhow!(
                        "invalid content, over {}",
                        property_util::get_max_content()
                    ));
                }
            }
            None => return Err(anyhow::anyhow!("content is blank : ")),
        }

        Ok(())
    }

    fn is_valid_char(ch: char) -> bool {
        VALID_CHARS.contains(&ch)
    }

    pub fn is_valid(param: &str) -> bool {
        if param.is_empty() {
            return false;
        }
        for ch in param.chars() {
            if !ch.is_alphanumeric() && !is_valid_char(ch) {
                return false;
            }
        }
        true
    }
}
pub mod property_util {
    use crate::common::AppSysConfig;
    pub struct ConfigProperty {
        max_content: usize,
    }

    impl Default for ConfigProperty {
        fn default() -> Self {
            ConfigProperty::new()
        }
    }
    impl ConfigProperty {
        pub fn new() -> Self {
            let sys_config = AppSysConfig::init_from_env();
            let max_content = sys_config.config_max_content;
            Self { max_content }
        }
    }
    pub fn get_max_content() -> usize {
        ConfigProperty::new().max_content
    }
}
