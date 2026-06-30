// SPDX-License-Identifier: AGPL-3.0-only

use axagent_harness::workflow_types::*;
use chrono::Utc;
use std::collections::HashMap;

pub struct PresetTemplate {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
    pub tags: Vec<&'static str>,
    pub system_prompt: &'static str,
    pub steps: Vec<PresetStep>,
}

#[derive(Debug, Clone)]
pub struct PresetStep {
    pub id: &'static str,
    pub goal: &'static str,
    pub role: &'static str,
    pub needs: Vec<&'static str>,
}

pub fn get_input_schema_for_template(preset: &PresetTemplate) -> Option<JsonSchema> {
    let mut props = HashMap::new();
    props.insert(
        "task".to_string(),
        JsonSchemaProperty {
            schema_type: "string".to_string(),
            description: Some("The main task or goal for this workflow".to_string()),
            default: None,
            enum_values: None,
            format: None,
        },
    );
    props.insert(
        "context".to_string(),
        JsonSchemaProperty {
            schema_type: "object".to_string(),
            description: Some("Additional context for the workflow".to_string()),
            default: None,
            enum_values: None,
            format: None,
        },
    );
    Some(JsonSchema {
        schema_type: "object".to_string(),
        description: Some(format!("Input schema for {} workflow", preset.name)),
        properties: Some(props),
        required: Some(vec!["task".to_string()]),
        items: None,
    })
}

pub fn get_output_schema_for_template(preset: &PresetTemplate) -> Option<JsonSchema> {
    let mut props = HashMap::new();
    props.insert(
        "result".to_string(),
        JsonSchemaProperty {
            schema_type: "object".to_string(),
            description: Some("The workflow execution result".to_string()),
            default: None,
            enum_values: None,
            format: None,
        },
    );
    props.insert(
        "success".to_string(),
        JsonSchemaProperty {
            schema_type: "boolean".to_string(),
            description: Some("Whether the workflow completed successfully".to_string()),
            default: Some(serde_json::json!(true)),
            enum_values: None,
            format: None,
        },
    );
    props.insert(
        "summary".to_string(),
        JsonSchemaProperty {
            schema_type: "string".to_string(),
            description: Some("Summary of the workflow execution".to_string()),
            default: None,
            enum_values: None,
            format: None,
        },
    );
    props.insert(
        "outputs".to_string(),
        JsonSchemaProperty {
            schema_type: "object".to_string(),
            description: Some("Named outputs from each step".to_string()),
            default: None,
            enum_values: None,
            format: None,
        },
    );
    Some(JsonSchema {
        schema_type: "object".to_string(),
        description: Some(format!("Output schema for {} workflow", preset.name)),
        properties: Some(props),
        required: Some(vec!["success".to_string()]),
        items: None,
    })
}

pub fn get_preset_templates() -> Vec<PresetTemplate> {
    vec![]
}

fn step_to_agent_node(
    step: &PresetStep,
    index: usize,
    template_system_prompt: &str,
) -> WorkflowNode {
    let base = WorkflowNodeBase {
        id: step.id.to_string(),
        title: format!("Agent: {}", step.role),
        description: Some(step.goal.to_string()),
        position: Position {
            x: 250.0,
            y: 100.0 + (index as f64 * 200.0),
        },
        retry: RetryConfig::default(),
        timeout: Some(300),
        enabled: true,
        parent_id: None,
        compensation: None,
    };

    WorkflowNode::Agent(AgentNode {
        base,
        config: AgentNodeConfig {
            system_prompt: format!(
                "You are a {} agent. Your goal: {}\n\n---\n\n{}",
                step.role, step.goal, template_system_prompt
            ),
            context_sources: vec![],
            output_var: format!("{}_result", step.id),
            model: None,
            temperature: None,
            max_tokens: None,
            tools: vec![],
            exposed_tools: vec![],
            output_mode: OutputMode::Json,
            agent_profile_id: None,
            max_tool_rounds: None,
            execution_mode: None,
            rag_source_ids: vec![],
            model_role: None,
            consistency_check: None,
            hallucination_guard: None,
            input_mapping: std::collections::HashMap::new(),
        },
    })
}

fn create_edges_for_steps(steps: &[PresetStep]) -> Vec<WorkflowEdge> {
    let mut edges = Vec::new();
    let mut edge_id = 0;

    for step in steps {
        for need in &step.needs {
            edges.push(WorkflowEdge {
                id: format!("edge_{}", edge_id),
                source: need.to_string(),
                source_handle: None,
                target: step.id.to_string(),
                target_handle: None,
                edge_type: EdgeType::Direct,
                label: None,
            });
            edge_id += 1;
        }
    }

    edges
}

fn detect_parallel_groups(steps: &[PresetStep]) -> Vec<Vec<&PresetStep>> {
    if steps.is_empty() {
        return vec![];
    }

    let mut groups: Vec<Vec<&PresetStep>> = Vec::new();
    let mut processed: std::collections::HashSet<_> = std::collections::HashSet::new();

    for step in steps {
        if processed.contains(&step.id) {
            continue;
        }

        let mut group: Vec<&PresetStep> = vec![step];
        processed.insert(step.id);

        for other in steps {
            if processed.contains(&other.id) {
                continue;
            }

            if step.id == other.id {
                continue;
            }

            let step_needs: std::collections::HashSet<_> = step.needs.iter().collect();
            let other_needs: std::collections::HashSet<_> = other.needs.iter().collect();

            if step_needs == other_needs && !step_needs.is_empty() {
                let step_deps_on_other = step.needs.contains(&other.id);
                let other_deps_on_step = other.needs.contains(&step.id);

                if !step_deps_on_other && !other_deps_on_step {
                    group.push(other);
                    processed.insert(other.id);
                }
            }
        }

        if group.len() > 1 {
            groups.push(group);
        }
    }

    groups
}

fn build_workflow_nodes(
    steps: &[PresetStep],
    start_y: f64,
    template_system_prompt: &str,
) -> Vec<WorkflowNode> {
    let mut nodes: Vec<WorkflowNode> = Vec::new();
    let parallel_groups = detect_parallel_groups(steps);

    let parallel_group_ids: std::collections::HashSet<_> = parallel_groups
        .iter()
        .flat_map(|g| g.iter().map(|s| s.id))
        .collect();

    let mut node_index = 0;
    for (i, step) in steps.iter().enumerate() {
        if parallel_group_ids.contains(&step.id) {
            continue;
        }

        let y = start_y + (node_index as f64 * 200.0);
        nodes.push(step_to_agent_node(step, i, template_system_prompt));
        if let Some(WorkflowNode::Agent(agent)) = nodes.last_mut() {
            agent.base.position.y = y;
        }
        node_index += 1;
    }

    for (group_idx, group) in parallel_groups.iter().enumerate() {
        let y = start_y + ((steps.len() + group_idx) as f64 * 200.0);

        let branch_ids: Vec<String> = group.iter().map(|s| s.id.to_string()).collect();

        nodes.push(WorkflowNode::Parallel(ParallelNode {
            base: WorkflowNodeBase {
                id: format!("parallel_{}", group[0].id),
                title: "Parallel Execution".to_string(),
                description: Some(format!("Executes {} branches in parallel", branch_ids.len())),
                position: Position { x: 400.0, y },
                retry: RetryConfig::default(),
                timeout: Some(600),
                enabled: true,
                parent_id: None,
                compensation: None,
            },
            config: ParallelNodeConfig {
                branches: group
                    .iter()
                    .enumerate()
                    .map(|(i, s)| Branch {
                        id: format!("branch_{}", i),
                        title: s.role.to_string(),
                        steps: vec![s.id.to_string()],
                        branch_timeout_ms: None,
                        degrade_strategy: DegradeStrategy::default(),
                    })
                    .collect(),
                wait_for_all: true,
                timeout: Some(600),
                aggregation: None,
                auto_input_from_parent: true,
                sub_graph: None,
            },
        }));

        nodes.push(WorkflowNode::Merge(MergeNode {
            base: WorkflowNodeBase {
                id: format!("merge_{}", group[0].id),
                title: "Merge".to_string(),
                description: Some("Merges parallel branches".to_string()),
                position: Position {
                    x: 250.0,
                    y: y + 250.0,
                },
                retry: RetryConfig::default(),
                timeout: None,
                enabled: true,
                parent_id: None,
                compensation: None,
            },
            config: MergeNodeConfig {
                merge_type: MergeStrategy::All,
                inputs: branch_ids.clone(),
                auto_inputs_from_branches: true,
            },
        }));
    }

    nodes
}

fn build_stock_analysis_nodes(_steps: &[PresetStep], start_y: f64) -> Vec<WorkflowNode> {
    let mut nodes: Vec<WorkflowNode> = Vec::new();
    let mut y = start_y;

    nodes.push(WorkflowNode::Parallel(ParallelNode {
        base: WorkflowNodeBase {
            id: "p-analysts".to_string(),
            title: "9 Analyst Agents".to_string(),
            description: Some("9 parallel branches for comprehensive stock analysis".to_string()),
            position: Position { x: 400.0, y },
            retry: RetryConfig::default(),
            timeout: Some(600),
            enabled: true,
            parent_id: None,
            compensation: None,
        },
        config: ParallelNodeConfig {
            branches: vec![
                Branch {
                    id: "branch_fundamental".to_string(),
                    title: "Fundamental".to_string(),
                    steps: vec![
                        "fundamental-tool".to_string(),
                        "fundamental-analyst".to_string(),
                    ],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::default(),
                },
                Branch {
                    id: "branch_technical".to_string(),
                    title: "Technical".to_string(),
                    steps: vec![
                        "technical-tool".to_string(),
                        "technical-analyst".to_string(),
                    ],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::default(),
                },
                Branch {
                    id: "branch_sentiment".to_string(),
                    title: "Sentiment".to_string(),
                    steps: vec!["news-tool".to_string(), "sentiment-analyst".to_string()],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::default(),
                },
                Branch {
                    id: "branch_peer".to_string(),
                    title: "Peer".to_string(),
                    steps: vec!["peer-tool".to_string(), "peer-analyst".to_string()],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::default(),
                },
                Branch {
                    id: "branch_macro".to_string(),
                    title: "Macro".to_string(),
                    steps: vec!["macro-tool".to_string(), "macro-analyst".to_string()],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::default(),
                },
                Branch {
                    id: "branch_risk".to_string(),
                    title: "Risk".to_string(),
                    steps: vec!["risk-tool".to_string(), "risk-analyst".to_string()],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::default(),
                },
                Branch {
                    id: "branch_insider".to_string(),
                    title: "Insider".to_string(),
                    steps: vec!["insider-tool".to_string(), "insider-analyst".to_string()],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::default(),
                },
                Branch {
                    id: "branch_dividend".to_string(),
                    title: "Dividend".to_string(),
                    steps: vec!["dividend-tool".to_string(), "dividend-analyst".to_string()],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::default(),
                },
                Branch {
                    id: "branch_forecast".to_string(),
                    title: "Forecast".to_string(),
                    steps: vec!["forecast-tool".to_string(), "forecast-analyst".to_string()],
                    branch_timeout_ms: None,
                    degrade_strategy: DegradeStrategy::default(),
                },
            ],
            wait_for_all: true,
            timeout: Some(600),
            aggregation: Some(MergeStrategy::All),
            auto_input_from_parent: true,
            sub_graph: None,
        },
    }));

    y += 250.0;

    nodes.push(WorkflowNode::Merge(MergeNode {
        base: WorkflowNodeBase {
            id: "m-analysts".to_string(),
            title: "Merge Analysts".to_string(),
            description: Some("Merges outputs from all 9 analyst branches".to_string()),
            position: Position { x: 250.0, y },
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
            compensation: None,
        },
        config: MergeNodeConfig {
            merge_type: MergeStrategy::All,
            inputs: vec![
                "fundamental-analyst".to_string(),
                "technical-analyst".to_string(),
                "sentiment-analyst".to_string(),
                "peer-analyst".to_string(),
                "macro-analyst".to_string(),
                "risk-analyst".to_string(),
                "insider-analyst".to_string(),
                "dividend-analyst".to_string(),
                "forecast-analyst".to_string(),
            ],
            auto_inputs_from_branches: true,
        },
    }));

    y += 250.0;

    nodes.push(WorkflowNode::Condition(ConditionNode {
        base: WorkflowNodeBase {
            id: "c-need-debate".to_string(),
            title: "Need Debate?".to_string(),
            description: Some("Determines if further debate is needed".to_string()),
            position: Position { x: 250.0, y },
            retry: RetryConfig::default(),
            timeout: Some(60),
            enabled: true,
                parent_id: None,
                compensation: None,
        },
        config: ConditionNodeConfig {
            conditions: vec![],
            logical_op: LogicalOperator::And,
            judge_by_llm: Some(true),
            routing_prompt: Some(
                "Based on the analysis results, determine if there are conflicting opinions that require debate. Consider: 1) Significant disagreement between analysts, 2) High uncertainty in key metrics, 3) Material differences in risk assessment. Return true if debate is needed, false otherwise.".to_string(),
            ),
            routing_model: None,
            confidence_threshold: None,
        },
    }));

    y += 250.0;

    nodes.push(WorkflowNode::End(EndNode {
        base: WorkflowNodeBase {
            id: "end".to_string(),
            title: "End".to_string(),
            description: Some("Workflow completed".to_string()),
            position: Position { x: 250.0, y },
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
            compensation: None,
        },
        config: EndNodeConfig { output_var: None },
    }));

    nodes
}

pub fn convert_preset_to_workflow_template(preset: &PresetTemplate) -> WorkflowTemplateData {
    let now = Utc::now().timestamp_millis();

    let mut nodes: Vec<WorkflowNode> = Vec::new();
    let mut edges: Vec<WorkflowEdge> = Vec::new();

    nodes.push(WorkflowNode::Trigger(TriggerNode {
        base: WorkflowNodeBase {
            id: "trigger".to_string(),
            title: "Manual Trigger".to_string(),
            description: Some("Starts the workflow manually".to_string()),
            position: Position { x: 250.0, y: 0.0 },
            retry: RetryConfig::default(),
            timeout: None,
            enabled: true,
            parent_id: None,
            compensation: None,
        },
        config: TriggerConfig {
            trigger_type: TriggerType::Manual,
            config: serde_json::json!({}),
        },
    }));

    if preset.id == "stock-analysis" {
        let stock_nodes = build_stock_analysis_nodes(&preset.steps, 100.0);
        nodes.extend(stock_nodes);

        edges.push(WorkflowEdge {
            id: "edge_trigger_to_p_analysts".to_string(),
            source: "trigger".to_string(),
            source_handle: None,
            target: "p-analysts".to_string(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        });

        edges.push(WorkflowEdge {
            id: "edge_p_analysts_to_m_analysts".to_string(),
            source: "p-analysts".to_string(),
            source_handle: None,
            target: "m-analysts".to_string(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        });

        edges.push(WorkflowEdge {
            id: "edge_m_analysts_to_c_need_debate".to_string(),
            source: "m-analysts".to_string(),
            source_handle: None,
            target: "c-need-debate".to_string(),
            target_handle: None,
            edge_type: EdgeType::Direct,
            label: None,
        });

        edges.push(WorkflowEdge {
            id: "edge_c_need_debate_true_to_end".to_string(),
            source: "c-need-debate".to_string(),
            source_handle: Some("true".to_string()),
            target: "end".to_string(),
            target_handle: None,
            edge_type: EdgeType::ConditionTrue,
            label: None,
        });

        edges.push(WorkflowEdge {
            id: "edge_c_need_debate_false_to_end".to_string(),
            source: "c-need-debate".to_string(),
            source_handle: Some("false".to_string()),
            target: "end".to_string(),
            target_handle: None,
            edge_type: EdgeType::ConditionFalse,
            label: None,
        });
    } else {
        let step_nodes = build_workflow_nodes(&preset.steps, 100.0, preset.system_prompt);
        nodes.extend(step_nodes);

        let end_y = 100.0 + ((preset.steps.len() + 2) as f64 * 200.0);
        nodes.push(WorkflowNode::End(EndNode {
            base: WorkflowNodeBase {
                id: "end".to_string(),
                title: "End".to_string(),
                description: Some("Workflow completed".to_string()),
                position: Position { x: 250.0, y: end_y },
                retry: RetryConfig::default(),
                timeout: None,
                enabled: true,
                parent_id: None,
                compensation: None,
            },
            config: EndNodeConfig { output_var: None },
        }));

        edges.extend(create_edges_for_steps(&preset.steps));

        let parallel_groups = detect_parallel_groups(&preset.steps);
        for group in &parallel_groups {
            let parallel_id = format!("parallel_{}", group[0].id);
            let merge_id = format!("merge_{}", group[0].id);

            for (i, step) in group.iter().enumerate() {
                edges.push(WorkflowEdge {
                    id: format!("edge_parallel_to_{}", step.id),
                    source: parallel_id.clone(),
                    source_handle: Some(format!("branch_{}", i)),
                    target: step.id.to_string(),
                    target_handle: None,
                    edge_type: EdgeType::Direct,
                    label: None,
                });

                edges.push(WorkflowEdge {
                    id: format!("edge_{}_to_merge", step.id),
                    source: step.id.to_string(),
                    source_handle: None,
                    target: merge_id.clone(),
                    target_handle: Some(format!("input_{}", i)),
                    edge_type: EdgeType::Direct,
                    label: None,
                });
            }

            if let Some(first_need) = group[0].needs.first() {
                edges.push(WorkflowEdge {
                    id: format!("edge_{}_to_parallel", first_need),
                    source: first_need.to_string(),
                    source_handle: None,
                    target: parallel_id.clone(),
                    target_handle: None,
                    edge_type: EdgeType::Direct,
                    label: None,
                });
            }
        }

        if let Some(first_step) = preset.steps.first() {
            let is_in_parallel = parallel_groups
                .iter()
                .any(|g| g.iter().any(|s| s.id == first_step.id));
            if !is_in_parallel {
                edges.push(WorkflowEdge {
                    id: "edge_trigger_start".to_string(),
                    source: "trigger".to_string(),
                    source_handle: None,
                    target: first_step.id.to_string(),
                    target_handle: None,
                    edge_type: EdgeType::Direct,
                    label: None,
                });
            }
        }

        for group in &parallel_groups {
            if !group[0].needs.is_empty() {
                edges.push(WorkflowEdge {
                    id: format!("edge_trigger_to_parallel_{}", group[0].id),
                    source: "trigger".to_string(),
                    source_handle: None,
                    target: format!("parallel_{}", group[0].id),
                    target_handle: None,
                    edge_type: EdgeType::Direct,
                    label: None,
                });
            }
        }

        let non_parallel_last_steps: Vec<_> = preset
            .steps
            .iter()
            .filter(|s| {
                !parallel_groups
                    .iter()
                    .any(|g| g.iter().any(|gs| gs.id == s.id))
            })
            .collect();

        if let Some(last_step) = non_parallel_last_steps.last() {
            edges.push(WorkflowEdge {
                id: "edge_last_end".to_string(),
                source: last_step.id.to_string(),
                source_handle: None,
                target: "end".to_string(),
                target_handle: None,
                edge_type: EdgeType::Direct,
                label: None,
            });
        }

        for group in &parallel_groups {
            edges.push(WorkflowEdge {
                id: format!("edge_merge_{}_to_end", group[0].id),
                source: format!("merge_{}", group[0].id),
                source_handle: None,
                target: "end".to_string(),
                target_handle: None,
                edge_type: EdgeType::Direct,
                label: None,
            });
        }
    }

    WorkflowTemplateData {
        id: preset.id.to_string(),
        name: preset.name.to_string(),
        description: Some(preset.description.to_string()),
        icon: preset.icon.to_string(),
        tags: preset.tags.iter().map(|s| s.to_string()).collect(),
        version: 1,
        is_preset: true,
        is_editable: false,
        is_public: false,
        trigger_config: Some(TriggerConfig {
            trigger_type: TriggerType::Manual,
            config: serde_json::json!({}),
        }),
        nodes,
        edges,
        input_schema: get_input_schema_for_template(preset),
        output_schema: get_output_schema_for_template(preset),
        variables: vec![],
        error_config: Some(ErrorConfig {
            retry_policy: Some(RetryPolicy {
                max_retries: 3,
                base_delay_ms: 1000,
                max_delay_ms: 30000,
            }),
            on_failure: OnFailureAction::Abort,
            error_branch: None,
            compensation_steps: None,
        }),
        tool_defs: vec![],
        created_at: now,
        updated_at: now,
    }
}
