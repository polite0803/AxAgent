//! 计划编译 —— 将 Plan 转换为工作流 DAG。
//!
//! 由 `axagent-harness::plan_types` 提供数据类型，
//! `axagent-agent::hierarchical_planner` 和 `axagent-rt-workflow`
//! 均可使用，无需依赖彼此。

use axagent_harness::plan_types::Plan;
use axagent_harness::workflow_types::*;

// ── Plan → DAG 编译 ─────────────────────────────────────

/// 将 Plan 的 Phase 编译为工作流 DAG。
/// - Phase 间串行（按 phase.dependencies 排序）
/// - Phase 内 Task 按 task.dependencies 生成精确边
/// - action_type="tool" → ToolNode, "llm" → LLMNode, 其他 → AgentNode
pub fn compile_plan_to_dag(
    plan: &Plan,
    tool_names: &[String],
) -> (
    Vec<WorkflowNode>,
    Vec<WorkflowEdge>,
) {

    let mut nodes: Vec<WorkflowNode> = Vec::new();
    let mut edges: Vec<WorkflowEdge> = Vec::new();
    let mut edge_id = 0u32;
    let mut all_task_ids: Vec<String> = Vec::new();
    let mut phase_node_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut has_outgoing: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Trigger
    let trigger_id = "trigger".to_string();
    nodes.push(WorkflowNode::Trigger(TriggerNode {
        base: WorkflowNodeBase {
            id: trigger_id.clone(),
            title: "Start".to_string(),
            description: None,
            position: Position { x: 0.0, y: 0.0 },
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
        },
        config: TriggerConfig {
            trigger_type: TriggerType::Manual,
            config: serde_json::Value::Null,
        },
    }));

    for (pi, phase) in plan.phases.iter().enumerate() {
        // Phase 内 task_id → node_id 映射
        let mut task_to_node: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        let mut phase_nodes: Vec<String> = Vec::new();

        for (ti, task) in phase.tasks.iter().enumerate() {
            let nid = format!("p{pi}_t{ti}_{}", task.id);
            task_to_node.insert(task.id.clone(), nid.clone());
            all_task_ids.push(task.id.clone());

            let base = WorkflowNodeBase {
                id: nid.clone(),
                title: task.description.clone(),
                description: None,
                position: Position {
                    x: 200.0 + (ti as f64 * 220.0),
                    y: 150.0 + (pi as f64 * 180.0),
                },
                retry: RetryConfig {
                    enabled: true,
                    max_retries: task.max_retries,
                    ..RetryConfig::default()
                },
                timeout: Some(300),
                enabled: true,
                parent_id: None,
            };

            let node = match task.action_type.as_str() {
                "tool" => WorkflowNode::Tool(ToolNode {
                    base,
                    config: ToolNodeConfig {
                        tool_name: task
                            .parameters
                            .get("tool")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        input_mapping: task
                            .parameters
                            .as_object()
                            .map(|obj| {
                                obj.iter()
                                    .filter(|(k, _)| *k != "tool")
                                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        output_var: format!("r_{nid}"),
                    },
                }),
                "llm" => WorkflowNode::Llm(LLMNode {
                    base,
                    config: LLMNodeConfig {
                        model: "".to_string(),
                        prompt: task.description.clone(),
                        messages: None,
                        temperature: Some(0.3),
                        max_tokens: Some(4096),
                        tools: Some(tool_names.to_vec()),
                        functions: None,
                    },
                }),
                _ => WorkflowNode::Agent(AgentNode {
                    base,
                    config: AgentNodeConfig {
                        system_prompt: task.description.clone(),
                        context_sources: vec![],
                        output_var: format!("r_{nid}"),
                        model: None,
                        temperature: None,
                        max_tokens: None,
                        tools: tool_names
                            .iter()
                            .map(|n| ToolDef {
                                name: n.clone(),
                                description: None,
                                parameters: None,
                            })
                            .collect(),
                        exposed_tools: vec![],
                        output_mode: OutputMode::Text,
                        agent_profile_id: None,
                        max_tool_rounds: None,
                        execution_mode: None,
                        rag_source_ids: vec![],
                        model_role: None,
                    },
                }),
            };

            nodes.push(node);
            phase_nodes.push(nid.clone());
        }
        phase_node_map.insert(phase.id.clone(), phase_nodes.clone());

        // Phase 内 + Phase 间：按 task.dependencies + phase.dependencies 生成 edges
        for task in &phase.tasks {
            let Some(target_id) = task_to_node.get(&task.id) else { continue };
            if task.dependencies.is_empty() {
                // Task 无内部依赖 → 从 phase 依赖或 Trigger/前驱 phase 连入
                let source_phase_ids: Vec<&String> = if phase.dependencies.is_empty() {
                    if pi == 0 {
                        vec![]
                    } else {
                        vec![&plan.phases[pi - 1].id]
                    }
                } else {
                    phase.dependencies.iter().collect()
                };

                if source_phase_ids.is_empty() && pi == 0 {
                    // 第一层且无 phase 依赖 → Trigger
                    edge_id += 1;
                    edges.push(WorkflowEdge {
                        id: format!("e_{edge_id}"),
                        source: trigger_id.clone(),
                        target: target_id.clone(),
                        edge_type: EdgeType::Direct,
                        label: None,
                        source_handle: None,
                        target_handle: None,
                    });
                    has_outgoing.insert(trigger_id.clone());
                } else {
                    for src_phase_id in &source_phase_ids {
                        if let Some(src_nodes) = phase_node_map.get(*src_phase_id) {
                            let leaves: Vec<String> = src_nodes
                                .iter()
                                .filter(|pn| !has_outgoing.contains(*pn))
                                .cloned()
                                .collect();
                            let sources = if leaves.is_empty() {
                                src_nodes.clone()
                            } else {
                                leaves
                            };
                            for src in &sources {
                                has_outgoing.insert(src.clone());
                                edge_id += 1;
                                edges.push(WorkflowEdge {
                                    id: format!("e_{edge_id}"),
                                    source: src.clone(),
                                    target: target_id.clone(),
                                    edge_type: EdgeType::Direct,
                                    label: None,
                                    source_handle: None,
                                    target_handle: None,
                                });
                            }
                        }
                    }
                }
            } else {
                for dep_id in &task.dependencies {
                    if let Some(source_id) = task_to_node.get(dep_id) {
                        has_outgoing.insert(source_id.clone());
                        edge_id += 1;
                        edges.push(WorkflowEdge {
                            id: format!("e_{edge_id}"),
                            source: source_id.clone(),
                            target: target_id.clone(),
                            edge_type: EdgeType::Direct,
                            label: None,
                            source_handle: None,
                            target_handle: None,
                        });
                    }
                }
            }
        }
    }

    // End node
    let end_id = "end".to_string();
    nodes.push(WorkflowNode::End(EndNode {
        base: WorkflowNodeBase {
            id: end_id.clone(),
            title: "End".to_string(),
            description: None,
            position: Position { x: 600.0, y: 500.0 },
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
        },
        config: EndNodeConfig {
            output_var: Some("plan_result".to_string()),
        },
    }));

    // Connect leaf nodes to end
    {
        let leaf_ids: Vec<String> = nodes
            .iter()
            .filter(|n| {
                matches!(n, WorkflowNode::Tool(_) | WorkflowNode::Llm(_) | WorkflowNode::Agent(_))
            })
            .map(|n| n.base_id().to_string())
            .filter(|n| !has_outgoing.contains(n))
            .collect();
        for n in &leaf_ids {
            has_outgoing.insert(n.clone());
            edge_id += 1;
            edges.push(WorkflowEdge {
                id: format!("e_{edge_id}"),
                source: n.clone(),
                target: end_id.clone(),
                edge_type: EdgeType::Direct,
                label: None,
                source_handle: None,
                target_handle: None,
            });
        }
    }

    (nodes, edges)
}
