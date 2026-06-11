// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDiff {
    pub name_changed: bool,
    pub description_changed: bool,
    pub icon_changed: bool,
    pub tags_changed: bool,
    pub nodes_changed: bool,
    pub edges_changed: bool,
    pub variables_changed: bool,
    pub input_schema_changed: bool,
    pub output_schema_changed: bool,
    pub trigger_config_changed: bool,
    pub error_config_changed: bool,
}

impl VersionDiff {
    pub fn has_changes(&self) -> bool {
        self.name_changed
            || self.description_changed
            || self.icon_changed
            || self.tags_changed
            || self.nodes_changed
            || self.edges_changed
            || self.variables_changed
            || self.input_schema_changed
            || self.output_schema_changed
            || self.trigger_config_changed
            || self.error_config_changed
    }

    pub fn changed_fields(&self) -> Vec<&'static str> {
        let mut fields = Vec::new();
        if self.name_changed {
            fields.push("name");
        }
        if self.description_changed {
            fields.push("description");
        }
        if self.icon_changed {
            fields.push("icon");
        }
        if self.tags_changed {
            fields.push("tags");
        }
        if self.nodes_changed {
            fields.push("nodes");
        }
        if self.edges_changed {
            fields.push("edges");
        }
        if self.variables_changed {
            fields.push("variables");
        }
        if self.input_schema_changed {
            fields.push("input_schema");
        }
        if self.output_schema_changed {
            fields.push("output_schema");
        }
        if self.trigger_config_changed {
            fields.push("trigger_config");
        }
        if self.error_config_changed {
            fields.push("error_config");
        }
        fields
    }
}

pub struct WorkflowVersionComparator;

impl WorkflowVersionComparator {
    pub fn compare(
        v1: &axagent_harness::workflow_types::WorkflowTemplateVersionData,
        v2: &axagent_harness::workflow_types::WorkflowTemplateVersionData,
    ) -> VersionDiff {
        // 标量字段直接比较；复杂结构体字段（nodes/edges/variables/各 *config）
        // 走 JSON 序列化比较，语义等价于原 Model 把这些字段存为 JSON 字符串时的字符串比较。
        VersionDiff {
            name_changed: v1.name != v2.name,
            description_changed: v1.description != v2.description,
            icon_changed: v1.icon != v2.icon,
            tags_changed: v1.tags != v2.tags,
            nodes_changed: json_ne(&v1.nodes, &v2.nodes),
            edges_changed: json_ne(&v1.edges, &v2.edges),
            variables_changed: json_ne(&v1.variables, &v2.variables),
            input_schema_changed: json_opt_ne(&v1.input_schema, &v2.input_schema),
            output_schema_changed: json_opt_ne(&v1.output_schema, &v2.output_schema),
            trigger_config_changed: json_opt_ne(&v1.trigger_config, &v2.trigger_config),
            error_config_changed: json_opt_ne(&v1.error_config, &v2.error_config),
        }
    }
}

fn json_ne<T: serde::Serialize>(a: &T, b: &T) -> bool {
    serde_json::to_string(a).ok() != serde_json::to_string(b).ok()
}

fn json_opt_ne<T: serde::Serialize>(a: &Option<T>, b: &Option<T>) -> bool {
    serde_json::to_string(a).ok() != serde_json::to_string(b).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_diff_has_changes() {
        let diff = VersionDiff {
            name_changed: true,
            description_changed: false,
            icon_changed: false,
            tags_changed: false,
            nodes_changed: false,
            edges_changed: false,
            variables_changed: false,
            input_schema_changed: false,
            output_schema_changed: false,
            trigger_config_changed: false,
            error_config_changed: false,
        };
        assert!(diff.has_changes());
        assert_eq!(diff.changed_fields(), vec!["name"]);
    }

    #[test]
    fn test_version_diff_no_changes() {
        let diff = VersionDiff {
            name_changed: false,
            description_changed: false,
            icon_changed: false,
            tags_changed: false,
            nodes_changed: false,
            edges_changed: false,
            variables_changed: false,
            input_schema_changed: false,
            output_schema_changed: false,
            trigger_config_changed: false,
            error_config_changed: false,
        };
        assert!(!diff.has_changes());
        assert!(diff.changed_fields().is_empty());
    }
}
