use serde::{Deserialize, Serialize};

const VALID_NODE_TYPES: &[&str] = &[
    "trigger",
    "agent",
    "llm",
    "condition",
    "parallel",
    "loop",
    "merge",
    "delay",
    "tool",
    "code",
    "atomicSkill",
    "end",
];

const VALID_EDGE_TYPES: &[&str] = &[
    "direct",
    "conditionTrue",
    "conditionFalse",
    "loopBack",
    "parallelBranch",
    "merge",
    "error",
];

const VALID_ENTRY_TYPES: &[&str] = &["builtin", "mcp", "local", "plugin"];

const VALID_CATEGORIES: &[&str] = &[
    "data_processing",
    "web_scraping",
    "file_operation",
    "api_integration",
    "text_processing",
    "automation",
    "monitoring",
    "integration",
    "other",
];

const VALID_LOOP_TYPES: &[&str] = &["forEach", "while", "doWhile", "until"];

const VALID_COMPARE_OPERATORS: &[&str] = &[
    "eq", "ne", "gt", "lt", "gte", "lte", "contains", "notContains",
    "startsWith", "endsWith", "regexMatch", "isEmpty", "isNotEmpty",
];

const VALID_LOGICAL_OPERATORS: &[&str] = &["and", "or"];

const VALID_TRIGGER_TYPES: &[&str] = &["manual", "schedule", "webhook", "event"];

const VALID_AGENT_ROLES: &[&str] = &[
    "researcher", "planner", "developer", "reviewer", "synthesizer", "executor",
];

const VALID_OUTPUT_MODES: &[&str] = &["json", "text", "artifact"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub severity: IssueSeverity,
    pub node_id: Option<String>,
    pub field: Option<String>,
    pub message: String,
    pub original_value: Option<String>,
    pub corrected_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub issues: Vec<ValidationIssue>,
    pub corrected_workflow: Option<serde_json::Value>,
}

pub struct WorkflowValidator;

impl WorkflowValidator {
    pub fn validate(workflow_json: &serde_json::Value) -> ValidationResult {
        let mut issues = Vec::new();
        let mut corrected = workflow_json.clone();

        if let Some(nodes) = corrected.get_mut("nodes").and_then(|n| n.as_array_mut()) {
            for node in nodes.iter_mut() {
                let node_issues = Self::validate_node(node);
                issues.extend(node_issues);
            }
        }

        if let Some(edges) = corrected.get_mut("edges").and_then(|e| e.as_array_mut()) {
            for edge in edges.iter_mut() {
                let edge_issues = Self::validate_edge(edge);
                issues.extend(edge_issues);
            }
        }

        if let Some(skills) = corrected.get_mut("atomic_skills").and_then(|s| s.as_array_mut()) {
            for skill in skills.iter_mut() {
                let skill_issues = Self::validate_atomic_skill(skill);
                issues.extend(skill_issues);
            }
        }

        let has_errors = issues.iter().any(|i| i.severity == IssueSeverity::Error);
        let is_valid = !has_errors;

        ValidationResult {
            is_valid,
            issues: issues.clone(),
            corrected_workflow: if issues.iter().any(|i| i.corrected_value.is_some()) {
                Some(corrected)
            } else {
                None
            },
        }
    }

    fn validate_node(node: &mut serde_json::Value) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        let node_id = node.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
        let node_type_opt = node.get("type").and_then(|v| v.as_str()).map(|s| s.to_string());
        let has_title = node.get("title").and_then(|v| v.as_str()).is_some();

        if let Some(node_type) = node_type_opt {
            if !VALID_NODE_TYPES.contains(&node_type.as_str()) {
                let correction = Self::find_closest_node_type(&node_type);
                issues.push(ValidationIssue {
                    severity: IssueSeverity::Error,
                    node_id: node_id.clone(),
                    field: Some("type".to_string()),
                    message: format!("未知节点类型 '{}'，将降级为 '{}'", node_type, correction),
                    original_value: Some(node_type.clone()),
                    corrected_value: Some(correction.to_string()),
                });
                if let Some(map) = node.as_object_mut() {
                    map.insert("type".to_string(), serde_json::Value::String(correction.to_string()));
                }
            }

            if let Some(map) = node.as_object_mut() {
                if let Some(config) = map.get_mut("config") {
                    if let Some(config_obj) = config.as_object_mut() {
                        issues.extend(Self::validate_node_config(&node_type, config_obj, node_id.as_deref()));
                    }
                }
            }
        } else {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Error,
                node_id: node_id.clone(),
                field: Some("type".to_string()),
                message: "节点缺少 'type' 字段".to_string(),
                original_value: None,
                corrected_value: None,
            });
        }

        if node.get("id").and_then(|v| v.as_str()).is_none() {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Error,
                node_id: None,
                field: Some("id".to_string()),
                message: "节点缺少 'id' 字段".to_string(),
                original_value: None,
                corrected_value: None,
            });
        }

        if !has_title {
            let default_title = node_id.clone().unwrap_or_else(|| "未命名节点".to_string());
            issues.push(ValidationIssue {
                severity: IssueSeverity::Warning,
                node_id: node_id.clone(),
                field: Some("title".to_string()),
                message: format!("节点缺少 'title'，使用默认值 '{}'", default_title),
                original_value: None,
                corrected_value: Some(default_title.clone()),
            });
            if let Some(map) = node.as_object_mut() {
                map.insert("title".to_string(), serde_json::Value::String(default_title));
            }
        }

        issues
    }

    fn validate_node_config(node_type: &str, config: &mut serde_json::Map<String, serde_json::Value>, node_id: Option<&str>) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        match node_type {
            "trigger" => {
                if let Some(trigger_type) = config.get("type").and_then(|v| v.as_str()) {
                    if !VALID_TRIGGER_TYPES.contains(&trigger_type) {
                        issues.push(ValidationIssue {
                            severity: IssueSeverity::Warning,
                            node_id: node_id.map(|s| s.to_string()),
                            field: Some("config.type".to_string()),
                            message: format!("无效触发类型 '{}'，降级为 'manual'", trigger_type),
                            original_value: Some(trigger_type.to_string()),
                            corrected_value: Some("manual".to_string()),
                        });
                        config.insert("type".to_string(), serde_json::Value::String("manual".to_string()));
                    }
                }
            }
            "agent" => {
                if let Some(role) = config.get("role").and_then(|v| v.as_str()) {
                    if !VALID_AGENT_ROLES.contains(&role) {
                        issues.push(ValidationIssue {
                            severity: IssueSeverity::Warning,
                            node_id: node_id.map(|s| s.to_string()),
                            field: Some("config.role".to_string()),
                            message: format!("无效 Agent 角色 '{}'，降级为 'developer'", role),
                            original_value: Some(role.to_string()),
                            corrected_value: Some("developer".to_string()),
                        });
                        config.insert("role".to_string(), serde_json::Value::String("developer".to_string()));
                    }
                }
                if let Some(output_mode) = config.get("output_mode").and_then(|v| v.as_str()) {
                    if !VALID_OUTPUT_MODES.contains(&output_mode) {
                        issues.push(ValidationIssue {
                            severity: IssueSeverity::Warning,
                            node_id: node_id.map(|s| s.to_string()),
                            field: Some("config.output_mode".to_string()),
                            message: format!("无效输出模式 '{}'，降级为 'text'", output_mode),
                            original_value: Some(output_mode.to_string()),
                            corrected_value: Some("text".to_string()),
                        });
                        config.insert("output_mode".to_string(), serde_json::Value::String("text".to_string()));
                    }
                }
            }
            "condition" => {
                if let Some(conditions) = config.get_mut("conditions") {
                    if let Some(conditions_arr) = conditions.as_array_mut() {
                        for cond in conditions_arr.iter_mut() {
                            if let Some(cond_obj) = cond.as_object_mut() {
                                if let Some(operator) = cond_obj.get("operator").and_then(|v| v.as_str()) {
                                    if !VALID_COMPARE_OPERATORS.contains(&operator) {
                                        issues.push(ValidationIssue {
                                            severity: IssueSeverity::Warning,
                                            node_id: node_id.map(|s| s.to_string()),
                                            field: Some("config.conditions[].operator".to_string()),
                                            message: format!("无效比较操作符 '{}'，降级为 'isNotEmpty'", operator),
                                            original_value: Some(operator.to_string()),
                                            corrected_value: Some("isNotEmpty".to_string()),
                                        });
                                        cond_obj.insert("operator".to_string(), serde_json::Value::String("isNotEmpty".to_string()));
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(logical_op) = config.get("logical_op").and_then(|v| v.as_str()) {
                    if !VALID_LOGICAL_OPERATORS.contains(&logical_op) {
                        issues.push(ValidationIssue {
                            severity: IssueSeverity::Warning,
                            node_id: node_id.map(|s| s.to_string()),
                            field: Some("config.logical_op".to_string()),
                            message: format!("无效逻辑操作符 '{}'，降级为 'and'", logical_op),
                            original_value: Some(logical_op.to_string()),
                            corrected_value: Some("and".to_string()),
                        });
                        config.insert("logical_op".to_string(), serde_json::Value::String("and".to_string()));
                    }
                }
            }
            "loop" => {
                if let Some(loop_type) = config.get("loop_type").and_then(|v| v.as_str()) {
                    if !VALID_LOOP_TYPES.contains(&loop_type) {
                        issues.push(ValidationIssue {
                            severity: IssueSeverity::Warning,
                            node_id: node_id.map(|s| s.to_string()),
                            field: Some("config.loop_type".to_string()),
                            message: format!("无效循环类型 '{}'，降级为 'forEach'", loop_type),
                            original_value: Some(loop_type.to_string()),
                            corrected_value: Some("forEach".to_string()),
                        });
                        config.insert("loop_type".to_string(), serde_json::Value::String("forEach".to_string()));
                    }
                }
            }
            _ => {}
        }

        issues
    }

    fn validate_edge(edge: &mut serde_json::Value) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let edge_id = edge.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
        let edge_type = edge.get("edge_type").and_then(|v| v.as_str());

        if let Some(edge_type) = edge_type {
            if !VALID_EDGE_TYPES.contains(&edge_type) {
                issues.push(ValidationIssue {
                    severity: IssueSeverity::Warning,
                    node_id: None,
                    field: Some("edge_type".to_string()),
                    message: format!("无效边类型 '{}'，降级为 'direct'", edge_type),
                    original_value: Some(edge_type.to_string()),
                    corrected_value: Some("direct".to_string()),
                });
                if let Some(map) = edge.as_object_mut() {
                    map.insert("edge_type".to_string(), serde_json::Value::String("direct".to_string()));
                }
            }
        }

        if !edge.get("source").and_then(|v| v.as_str()).is_some() {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Error,
                node_id: edge_id.clone(),
                field: Some("source".to_string()),
                message: "边缺少 'source' 字段".to_string(),
                original_value: None,
                corrected_value: None,
            });
        }

        if !edge.get("target").and_then(|v| v.as_str()).is_some() {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Error,
                node_id: edge_id.clone(),
                field: Some("target".to_string()),
                message: "边缺少 'target' 字段".to_string(),
                original_value: None,
                corrected_value: None,
            });
        }

        issues
    }

    fn validate_atomic_skill(skill: &mut serde_json::Value) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let skill_name = skill.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());

        if let Some(entry_type) = skill.get("entry_type").and_then(|v| v.as_str()) {
            if !VALID_ENTRY_TYPES.contains(&entry_type) {
                issues.push(ValidationIssue {
                    severity: IssueSeverity::Warning,
                    node_id: skill_name.clone(),
                    field: Some("entry_type".to_string()),
                    message: format!("无效入口类型 '{}'，降级为 'local'", entry_type),
                    original_value: Some(entry_type.to_string()),
                    corrected_value: Some("local".to_string()),
                });
                if let Some(map) = skill.as_object_mut() {
                    map.insert("entry_type".to_string(), serde_json::Value::String("local".to_string()));
                }
            }
        }

        if let Some(category) = skill.get("category").and_then(|v| v.as_str()) {
            if !VALID_CATEGORIES.contains(&category) {
                issues.push(ValidationIssue {
                    severity: IssueSeverity::Warning,
                    node_id: skill_name.clone(),
                    field: Some("category".to_string()),
                    message: format!("无效分类 '{}'，降级为 'other'", category),
                    original_value: Some(category.to_string()),
                    corrected_value: Some("other".to_string()),
                });
                if let Some(map) = skill.as_object_mut() {
                    map.insert("category".to_string(), serde_json::Value::String("other".to_string()));
                }
            }
        }

        if skill_name.is_none() {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Error,
                node_id: None,
                field: Some("name".to_string()),
                message: "原子技能缺少 'name' 字段".to_string(),
                original_value: None,
                corrected_value: None,
            });
        }

        if !skill.get("entry_ref").and_then(|v| v.as_str()).is_some() {
            if let Some(name) = &skill_name {
                issues.push(ValidationIssue {
                    severity: IssueSeverity::Warning,
                    node_id: skill_name.clone(),
                    field: Some("entry_ref".to_string()),
                    message: format!("原子技能缺少 'entry_ref'，使用 name '{}' 作为默认值", name),
                    original_value: None,
                    corrected_value: Some(name.clone()),
                });
                if let Some(map) = skill.as_object_mut() {
                    map.insert("entry_ref".to_string(), serde_json::Value::String(name.clone()));
                }
            }
        }

        issues
    }

    fn find_closest_node_type(input: &str) -> &'static str {
        let input_lower = input.to_lowercase();

        if input_lower.contains("trigger") || input_lower.contains("start") {
            "trigger"
        } else if input_lower.contains("end") || input_lower.contains("finish") || input_lower.contains("output") {
            "end"
        } else if input_lower.contains("condition") || input_lower.contains("branch") || input_lower.contains("if") {
            "condition"
        } else if input_lower.contains("parallel") || input_lower.contains("concurrent") || input_lower.contains("branch") {
            "parallel"
        } else if input_lower.contains("loop") || input_lower.contains("repeat") || input_lower.contains("iterate") {
            "loop"
        } else if input_lower.contains("merge") || input_lower.contains("join") || input_lower.contains("combine") {
            "merge"
        } else if input_lower.contains("delay") || input_lower.contains("wait") || input_lower.contains("sleep") {
            "delay"
        } else if input_lower.contains("agent") {
            "agent"
        } else if input_lower.contains("llm") || input_lower.contains("model") || input_lower.contains("ai") {
            "llm"
        } else if input_lower.contains("tool") || input_lower.contains("action") || input_lower.contains("function") {
            "tool"
        } else if input_lower.contains("code") || input_lower.contains("script") || input_lower.contains("python") || input_lower.contains("javascript") {
            "code"
        } else if input_lower.contains("skill") || input_lower.contains("atomic") {
            "atomicSkill"
        } else {
            "tool"
        }
    }

    pub fn apply_corrections(workflow_json: serde_json::Value) -> (serde_json::Value, Vec<ValidationIssue>) {
        let result = Self::validate(&workflow_json);
        (
            result.corrected_workflow.unwrap_or(workflow_json),
            result.issues,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_unknown_node_type() {
        let workflow = serde_json::json!({
            "nodes": [
                {
                    "id": "node1",
                    "type": "emailSender",
                    "title": "发送邮件",
                    "position": { "x": 0, "y": 0 },
                    "config": {}
                }
            ],
            "edges": []
        });

        let result = WorkflowValidator::validate(&workflow);
        assert!(!result.is_valid);
        assert!(!result.issues.is_empty());

        if let Some(corrected) = result.corrected_workflow {
            assert_eq!(corrected["nodes"][0]["type"], "tool");
        }
    }

    #[test]
    fn test_validate_unknown_edge_type() {
        let workflow = serde_json::json!({
            "nodes": [
                { "id": "node1", "type": "trigger", "title": "开始", "position": { "x": 0, "y": 0 }, "config": {} },
                { "id": "node2", "type": "end", "title": "结束", "position": { "x": 100, "y": 0 }, "config": {} }
            ],
            "edges": [
                { "id": "edge1", "source": "node1", "target": "node2", "edge_type": "unknownType" }
            ]
        });

        let result = WorkflowValidator::validate(&workflow);
        assert!(!result.issues.is_empty());
    }
}