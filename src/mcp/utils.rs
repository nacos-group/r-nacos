use crate::mcp::model::mcp::McpServerValue;
use crate::mcp::model::tools::ToolKey;
use std::collections::HashMap;

pub struct ToolSpecUtils;

impl ToolSpecUtils {
    pub fn update_server_ref_to_map(
        ref_map: &mut HashMap<ToolKey, HashMap<u64, i64>>,
        server_value: &McpServerValue,
    ) {
        for mcp_tool in &server_value.tools {
            Self::add_tool_ref_to_map(ref_map, &mcp_tool.tool_key, mcp_tool.tool_version, 1);
        }
    }

    pub fn add_tool_ref_to_map(
        ref_map: &mut HashMap<ToolKey, HashMap<u64, i64>>,
        tool_key: &ToolKey,
        version: u64,
        count: i64,
    ) {
        if let Some(tool_spec_map) = ref_map.get_mut(&tool_key) {
            let value = if let Some(value) = tool_spec_map.get(&version) {
                *value + count
            } else {
                count
            };
            if value != 0 {
                tool_spec_map.insert(version, value);
            }
        } else {
            let mut tool_spec_map = HashMap::new();
            tool_spec_map.insert(version, count);
            ref_map.insert(tool_key.clone(), tool_spec_map);
        }
    }

    pub fn merge_ref_map(
        target_ref_map: &mut HashMap<ToolKey, HashMap<u64, i64>>,
        add_ref_map: &HashMap<ToolKey, HashMap<u64, i64>>,
    ) {
        for (tool_key, version_map) in add_ref_map {
            for (version, count) in version_map {
                Self::add_tool_ref_to_map(target_ref_map, tool_key, *version, *count);
            }
        }
    }

    pub fn remove_ref_map(
        target_ref_map: &mut HashMap<ToolKey, HashMap<u64, i64>>,
        add_ref_map: &HashMap<ToolKey, HashMap<u64, i64>>,
    ) {
        for (tool_key, version_map) in add_ref_map {
            for (version, count) in version_map {
                Self::add_tool_ref_to_map(target_ref_map, tool_key, *version, 0 - *count);
            }
        }
    }
}
