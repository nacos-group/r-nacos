use std::sync::Arc;

lazy_static::lazy_static! {
    pub(crate) static ref CONFIG_TYPE_TEXT: Arc<String> =  Arc::new("text".to_string());
    pub(crate) static ref CONFIG_TYPE_JSON: Arc<String> =  Arc::new("json".to_string());
    pub(crate) static ref CONFIG_TYPE_XML: Arc<String> =  Arc::new("xml".to_string());
    pub(crate) static ref CONFIG_TYPE_YAML: Arc<String> =  Arc::new("yaml".to_string());
    pub(crate) static ref CONFIG_TYPE_HTML: Arc<String> =  Arc::new("html".to_string());
    pub(crate) static ref CONFIG_TYPE_PROPERTIES: Arc<String> =  Arc::new("properties".to_string());
    pub(crate) static ref CONFIG_TYPE_TOML: Arc<String> =  Arc::new("toml".to_string());
}

//html media type
pub(crate) const MEDIA_TYPE_TEXT_PLAIN: &'static str = "text/plain;charset=UTF-8";
pub(crate) const MEDIA_TYPE_TEXT_HTML: &'static str = "text/html;charset=UTF-8";
pub(crate) const MEDIA_TYPE_APPLICATION_JSON: &'static str = "application/json;charset=UTF-8";
pub(crate) const MEDIA_TYPE_APPLICATION_XML: &'static str = "application/xml;charset=UTF-8";

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum ConfigType {
    Text,
    Json,
    Xml,
    Yaml,
    Html,
    Properties,
    Toml,
}

impl Default for ConfigType {
    fn default() -> Self {
        Self::Text
    }
}

impl ConfigType {
    ///
    /// 根据类型值获取类型
    ///
    pub fn new_by_value(v: &str) -> Self {
        match v {
            "json" => Self::Json,
            "xml" => Self::Xml,
            "yaml" => Self::Yaml,
            "html" => Self::Html,
            "toml" => Self::Toml,
            "properties" => Self::Properties,
            _ => Self::Text,
        }
    }

    ///
    /// 获取类型对应值，储存到配置中心内存前统一使用此函数获取的值；
    /// 多个配置的类型值使用同一块地址，可节省内存；
    pub fn get_value(&self) -> Arc<String> {
        match self {
            ConfigType::Text => CONFIG_TYPE_TEXT.clone(),
            ConfigType::Json => CONFIG_TYPE_JSON.clone(),
            ConfigType::Xml => CONFIG_TYPE_XML.clone(),
            ConfigType::Yaml => CONFIG_TYPE_YAML.clone(),
            ConfigType::Html => CONFIG_TYPE_HTML.clone(),
            ConfigType::Properties => CONFIG_TYPE_PROPERTIES.clone(),
            ConfigType::Toml => CONFIG_TYPE_TOML.clone(),
        }
    }

    ///
    /// 获取类型对应媒介类型
    ///
    pub fn get_media_type(&self) -> &'static str {
        match self {
            ConfigType::Text => MEDIA_TYPE_TEXT_PLAIN,
            ConfigType::Json => MEDIA_TYPE_APPLICATION_JSON,
            ConfigType::Xml => MEDIA_TYPE_APPLICATION_XML,
            ConfigType::Yaml => MEDIA_TYPE_TEXT_PLAIN,
            ConfigType::Html => MEDIA_TYPE_TEXT_HTML,
            ConfigType::Properties => MEDIA_TYPE_TEXT_PLAIN,
            ConfigType::Toml => MEDIA_TYPE_TEXT_PLAIN,
        }
    }
}
