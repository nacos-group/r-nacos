use crate::metrics::timeline::model::TimelineQueryParam;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineQueryRequest {
    pub start_time: Option<u64>,
    // LEAST | MINUTE
    pub timeline_group_name: Option<String>,
    //pub query_all_key: bool,
    // 用逗号分割
    pub string_key: Option<String>,
    // string_key与keys只取其中一个
    pub keys: Option<Vec<String>>,
    pub node_id: Option<u64>,
}

impl From<TimelineQueryRequest> for TimelineQueryParam {
    fn from(value: TimelineQueryRequest) -> Self {
        let mut keys = value.keys.unwrap_or_default();
        if keys.is_empty() {
            keys = value
                .string_key
                .unwrap_or_default()
                .split(',')
                .filter(|e| !e.is_empty())
                .map(|e| e.to_string())
                .collect();
        }
        Self {
            start_time: value.start_time.unwrap_or_default(),
            timeline_group_name: value.timeline_group_name.unwrap_or_default(),
            keys,
            node_id: value.node_id.unwrap_or_default(),
        }
    }
}
