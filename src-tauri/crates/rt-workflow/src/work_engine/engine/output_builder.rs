// SPDX-License-Identifier: AGPL-3.0-only

//! Workflow output construction: end-node extraction, schema filtering, validation.

use std::collections::{HashMap, HashSet};

use axagent_harness::workflow_types::{JsonSchema, WorkflowEdge, WorkflowNode};

// ── 辅助函数（run_workflow 尾部使用）──

/// 扫描所有 EndNode，提取其 output_var 指向的节点输出作为聚合结果。
///
/// BE-I2 修复：当某个 EndNode 的 `output_var` 未写入任何结果时（例如 LLM 生成
/// 缺 EndNode 后由 helpers.rs 自动补充的 EndNode 指向 `final_output`，但没有任何
/// 节点把输出写入该变量），退化为聚合所有叶子节点（无出边的非 End 节点）的输出，
/// 保证聚合精简能力不失效。
pub(crate) fn extract_end_output(
    nodes: &[WorkflowNode],
    edges: &[WorkflowEdge],
    results: &HashMap<String, serde_json::Value>,
) -> Option<serde_json::Value> {
    let end_nodes: Vec<_> = nodes
        .iter()
        .filter_map(|n| match n {
            WorkflowNode::End(en) => Some(en),
            _ => None,
        })
        .collect();

    if end_nodes.is_empty() {
        return None;
    }

    // 收集所有作为边 source 的节点 id（有出边 ⇒ 非叶子）
    let source_ids: HashSet<String> = edges.iter().map(|e| e.source.clone()).collect();

    // 收集所有 EndNode 的输出
    let mut outputs = serde_json::Map::new();
    for en in &end_nodes {
        let cfg = &en.config;
        if let Some(ref var) = cfg.output_var
            && let Some(val) = results.get(var)
        {
            outputs.insert(var.clone(), val.clone());
            continue;
        }
        // 输出变量缺失 → 聚合叶子节点输出（按节点 id 为键）
        for n in nodes {
            if matches!(n, WorkflowNode::End(_)) {
                continue;
            }
            let id = n.base_id().to_string();
            if source_ids.contains(&id) {
                continue;
            }
            if let Some(val) = results.get(&id) {
                outputs.insert(id, val.clone());
            }
        }
    }

    if outputs.is_empty() {
        None
    } else if outputs.len() == 1 {
        outputs.into_values().next()
    } else {
        Some(serde_json::Value::Object(outputs))
    }
}

/// 按 output_schema 过滤/重组输出。
/// schema 中通过 `"$source": "node_id"` 字段标记值来源节点。
pub(crate) fn build_workflow_output(
    results: &HashMap<String, serde_json::Value>,
    end_output: Option<serde_json::Value>,
    output_schema: Option<&JsonSchema>,
) -> Option<serde_json::Value> {
    match output_schema {
        None => {
            // 无 schema → 优先使用 EndNode 聚合输出，否则返回全部 results
            end_output.or_else(|| Some(serde_json::json!(results)))
        },
        Some(schema) => {
            let filtered = filter_by_schema(results, schema);
            Some(filtered)
        },
    }
}

/// 按 JsonSchema 从 results 中提取/重组字段。
fn filter_by_schema(
    results: &HashMap<String, serde_json::Value>,
    schema: &JsonSchema,
) -> serde_json::Value {
    let props = match &schema.properties {
        Some(p) => p,
        None => return serde_json::json!(results),
    };

    let mut out = serde_json::Map::new();
    for (key, prop) in props {
        // 检查是否有 $source 自定义字段（标记值来源节点）
        let source = prop.default.as_ref().and_then(|d| d.get("$source")).and_then(|s| s.as_str());

        if let Some(node_id) = source {
            // 从指定节点输出中提取
            if let Some(node_output) = results.get(node_id) {
                out.insert(key.clone(), extract_nested(node_output, key));
            }
        } else if let Some(val) = results.get(key) {
            // 按 key 名直接匹配 node_id
            out.insert(key.clone(), val.clone());
        }
    }

    if out.is_empty() {
        serde_json::json!(results)
    } else {
        serde_json::Value::Object(out)
    }
}

/// 从嵌套 JSON 中提取最内层有意义的值。
fn extract_nested(value: &serde_json::Value, _key: &str) -> serde_json::Value {
    match value {
        serde_json::Value::Object(obj) => {
            // 尝试提取常见的包装字段
            if let Some(inner) =
                obj.get("result").or_else(|| obj.get("output")).or_else(|| obj.get("content"))
            {
                inner.clone()
            } else {
                value.clone()
            }
        },
        _ => value.clone(),
    }
}

/// 用 jsonschema crate 校验 input 是否匹配 schema。
pub(crate) fn validate_input(
    input: &serde_json::Value,
    schema: &JsonSchema,
) -> Result<(), Vec<String>> {
    let schema_json = serde_json::to_value(schema).unwrap_or(serde_json::Value::Null);
    let validator = jsonschema::Validator::new(&schema_json)
        .map_err(|e| vec![format!("Schema compile error: {e}")])?;
    let mut errors: Vec<String> = Vec::new();
    for err in validator.iter_errors(input) {
        errors.push(format!("{}: {}", err.instance_path(), err));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// 扫描工作流节点，收集所有 AgentNode 中引用的工具名。
pub(crate) fn collect_workflow_tool_names(nodes: &[WorkflowNode]) -> Vec<String> {
    let mut names = std::collections::HashSet::new();
    for node in nodes {
        match node {
            WorkflowNode::Agent(an) => {
                for tool in &an.config.tools {
                    names.insert(tool.name.clone());
                }
            },
            WorkflowNode::Tool(tn) if !tn.config.tool_name.is_empty() => {
                names.insert(tn.config.tool_name.clone());
            },
            _ => {},
        }
    }
    names.into_iter().collect()
}
