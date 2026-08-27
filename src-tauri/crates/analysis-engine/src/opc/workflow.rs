// OPC 行业工作流层
// 复用 axagent-harness::workflow_types 中的标准工作流节点体系
// IndustryConfig 定义步骤 → 生成 WorkflowTemplateData → 种子化到 DB → WorkEngine 执行

#![allow(clippy::type_complexity)]

use std::collections::HashMap;

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, ApprovalNode, ApprovalNodeConfig, CodeNodeConfig, ConditionNode,
    ConditionNodeConfig, DataTransformerNode, DataTransformerNodeConfig, EdgeType, EndNode,
    EndNodeConfig, JsonSchema, JsonSchemaProperty, NotificationNode, NotificationNodeConfig,
    OutputMode, ToolDef, TriggerConfig, TriggerNode, ValidationAssertion,
    ValidationNodeConfig as HValidationNodeConfig, Variable, WorkflowEdge as HWorkflowEdge,
    WorkflowNode, WorkflowNodeBase, WorkflowTemplateData,
};

use super::automation::{AutomationAction, AutomationCondition};
use super::industry_config::IndustryConfig;

/// 创建基础工作流节点
fn create_node_base(id: impl Into<String>, title: impl Into<String>) -> WorkflowNodeBase {
    WorkflowNodeBase {
        id: id.into(),
        title: title.into(),
        description: None,
        position: Default::default(),
        retry: Default::default(),
        timeout: None,
        enabled: true,
        parent_id: None,
        compensation: None,
        continue_on_fail: false,
    }
}

/// 工作流边：from_node_id → to_node_id（内部表示）
#[derive(Debug, Clone)]
pub struct WorkflowEdgeDef {
    pub from: String,
    pub to: String,
}

/// 从行业配置直接生成 WorkflowTemplateData（种子化到 DB 的入口）
///
/// 整合了原 IndustryWorkflow::from_adapter() + to_template_data() 的逻辑，
/// 让 OPC 行业工作流与股票分析工作流架构一致：
/// Config 定义步骤 → 生成模板数据 → 种子化 → WorkEngine 执行
///
/// # 参数
/// - `industry_id`: 行业 ID
/// - `config`: 行业配置
/// - `tool_resolver`: 可选的工具解析器，用于将工具名映射为完整的 ToolDef（含 description 和 parameters）
#[allow(unused_assignments)]
pub fn generate_industry_template_data(
    industry_id: &str,
    config: &IndustryConfig,
    tool_resolver: Option<&dyn Fn(&[String]) -> Vec<ToolDef>>,
) -> WorkflowTemplateData {
    let mut nodes: Vec<WorkflowNode> = Vec::new();
    let mut edges: Vec<WorkflowEdgeDef> = Vec::new();
    let mut prev_node_id: Option<String> = None;
    let mut node_counter = 0u32;

    let next_id = |counter: &mut u32, prefix: &str| -> String {
        *counter += 1;
        format!("{prefix}_{industry_id}_{counter}")
    };

    // ── 1. 触发节点（手动触发） ──
    let trigger_id = next_id(&mut node_counter, "trigger");
    nodes.push(WorkflowNode::Trigger(TriggerNode {
        base: create_node_base(trigger_id.clone(), format!("{industry_id} 行业分析触发")),
        config: TriggerConfig {
            trigger_type: axagent_harness::workflow_types::TriggerType::Manual,
            config: serde_json::json!({}),
        },
    }));
    prev_node_id = Some(trigger_id.clone());

    // ── 2. 验证节点（来自 config.validations） ──
    for validation in &config.validations {
        let node_id = next_id(&mut node_counter, "validation");
        let assertions = vec![ValidationAssertion {
            assertion_type: "field_check".to_string(),
            expected: Some(validation.r#type.clone()),
            actual: None,
            expression: Some(format!("field == {}", validation.field)),
        }];
        nodes.push(WorkflowNode::Validation(axagent_harness::workflow_types::ValidationNode {
            base: create_node_base(node_id.clone(), format!("验证: {}", validation.field)),
            config: HValidationNodeConfig {
                assertions,
                on_fail: "stop".to_string(),
                max_retries: 0,
            },
        }));
        if let Some(prev) = &prev_node_id {
            edges.push(WorkflowEdgeDef { from: prev.clone(), to: node_id.clone() });
        }
        prev_node_id = Some(node_id);
    }

    // ── 3. 业务步骤节点（代码驱动，支持 AgentNode/CodeNode） ──
    for step in &config.workflow_steps {
        let node_id = next_id(&mut node_counter, "step");
        let base = create_node_base(node_id.clone(), format!("步骤: {}", step.name));

        // 如果步骤定义了 prompt、tools 或 agent_profile_id，则生成 AgentNode
        if step.prompt.is_some() || !step.tools.is_empty() || step.agent_profile_id.is_some() {
            let tool_defs = if let Some(resolver) = tool_resolver {
                // 使用提供的工具解析器获取完整的工具定义
                resolver(&step.tools)
            } else {
                // 回退：只有工具名
                step.tools
                    .iter()
                    .map(|t| ToolDef { name: t.clone(), description: None, parameters: None })
                    .collect()
            };

            nodes.push(WorkflowNode::Agent(AgentNode {
                base,
                config: AgentNodeConfig {
                    system_prompt: step.prompt.clone().unwrap_or_else(|| step.description.clone()),
                    context_sources: Vec::new(),
                    input_mapping: step.inputs.clone(),
                    output_var: format!("step_{}", node_id),
                    model: None,
                    temperature: None,
                    max_tokens: None,
                    tools: tool_defs,
                    exposed_tools: step.tools.clone(),
                    output_mode: OutputMode::Text,
                    agent_profile_id: step.agent_profile_id.clone(),
                    max_tool_rounds: Some(3),
                    execution_mode: None,
                    rag_source_ids: Vec::new(),
                    model_role: None,
                    consistency_check: None,
                    hallucination_guard: None,
                    fallback_model: None,
                    task_scene: None,
                    stream_chunk_timeout_secs: None,
                },
            }));
        } else {
            // 回退到 CodeNode（纯逻辑步骤）
            nodes.push(WorkflowNode::Code(axagent_harness::workflow_types::CodeNode {
                base,
                config: CodeNodeConfig {
                    language: "rust".to_string(),
                    code: step.description.clone(),
                    output_var: format!("step_{}", node_id),
                    tool_name: None,
                    execute_directly: true,
                    input_mapping: step.inputs.clone(),
                },
            }));
        }

        if let Some(prev) = &prev_node_id {
            edges.push(WorkflowEdgeDef { from: prev.clone(), to: node_id.clone() });
        }
        prev_node_id = Some(node_id);
    }

    // ── 4. 自动化规则节点（Condition + Notification 组合） ──
    for rule in &config.automation_rules {
        let cond_id = next_id(&mut node_counter, "condition");
        let prev = prev_node_id.clone();
        let conditions = rule
            .conditions
            .iter()
            .map(|c| {
                let (var_path, operator, value) = match c {
                    AutomationCondition::FieldExceeds { field, threshold } => (
                        field.clone(),
                        axagent_harness::workflow_types::CompareOperator::Gte,
                        serde_json::json!(threshold),
                    ),
                    AutomationCondition::FieldBelow { field, threshold } => (
                        field.clone(),
                        axagent_harness::workflow_types::CompareOperator::Lte,
                        serde_json::json!(threshold),
                    ),
                    AutomationCondition::OverdueDaysGte { days } => (
                        "overdue_days".to_string(),
                        axagent_harness::workflow_types::CompareOperator::Gte,
                        serde_json::json!(days),
                    ),
                    AutomationCondition::EntityTypeIs { entity_type } => (
                        "entity_type".to_string(),
                        axagent_harness::workflow_types::CompareOperator::Eq,
                        serde_json::json!(entity_type),
                    ),
                    AutomationCondition::StatusIs { status } => (
                        "status".to_string(),
                        axagent_harness::workflow_types::CompareOperator::Eq,
                        serde_json::json!(status),
                    ),
                    AutomationCondition::CreatedDaysGte { days } => (
                        "created_days".to_string(),
                        axagent_harness::workflow_types::CompareOperator::Gte,
                        serde_json::json!(days),
                    ),
                    AutomationCondition::Custom { expression } => (
                        expression.clone(),
                        axagent_harness::workflow_types::CompareOperator::Eq,
                        serde_json::json!(true),
                    ),
                };
                axagent_harness::workflow_types::Condition { var_path, operator, value }
            })
            .collect();
        nodes.push(WorkflowNode::Condition(ConditionNode {
            base: create_node_base(cond_id.clone(), format!("条件: {}", rule.name)),
            config: ConditionNodeConfig {
                conditions,
                logical_op: axagent_harness::workflow_types::LogicalOperator::And,
                judge_by_llm: None,
                routing_prompt: None,
                routing_model: None,
                confidence_threshold: None,
            },
        }));
        if let Some(prev_id) = &prev {
            edges.push(WorkflowEdgeDef { from: prev_id.clone(), to: cond_id.clone() });
        }

        // 通知/动作节点（条件满足时执行）
        for action in &rule.actions {
            let action_id = next_id(&mut node_counter, "action");
            nodes.push(match action {
                AutomationAction::SendNotification { target, message } => {
                    WorkflowNode::Notification(NotificationNode {
                        base: create_node_base(action_id.clone(), format!("通知: {}", target)),
                        config: NotificationNodeConfig {
                            channel: "system".to_string(),
                            message: message.clone(),
                            webhook_url: None,
                            recipients: vec![target.clone()],
                            subject: None,
                            enabled: true,
                            output_var: format!("action_{}", action_id),
                        },
                    })
                },
                AutomationAction::UpdateField { field, value } => {
                    WorkflowNode::DataTransformer(DataTransformerNode {
                        base: create_node_base(action_id.clone(), format!("更新字段: {}", field)),
                        config: DataTransformerNodeConfig {
                            input_var: field.clone(),
                            expression: format!("{}", value),
                            output_var: format!("action_{}", action_id),
                        },
                    })
                },
                AutomationAction::UpdateStatus { status } => {
                    WorkflowNode::DataTransformer(DataTransformerNode {
                        base: create_node_base(action_id.clone(), format!("更新状态: {}", status)),
                        config: DataTransformerNodeConfig {
                            input_var: "status".to_string(),
                            expression: status.clone(),
                            output_var: format!("action_{}", action_id),
                        },
                    })
                },
                AutomationAction::MarkProcessed => {
                    WorkflowNode::DataTransformer(DataTransformerNode {
                        base: create_node_base(action_id.clone(), "标记为已处理"),
                        config: DataTransformerNodeConfig {
                            input_var: "status".to_string(),
                            expression: "processed".to_string(),
                            output_var: format!("action_{}", action_id),
                        },
                    })
                },
                AutomationAction::CreateRecord { entity_type, data } => {
                    WorkflowNode::DataTransformer(DataTransformerNode {
                        base: create_node_base(
                            action_id.clone(),
                            format!("创建记录: {}", entity_type),
                        ),
                        config: DataTransformerNodeConfig {
                            input_var: format!("{}_data", entity_type),
                            expression: format!("{}", data),
                            output_var: format!("action_{}", action_id),
                        },
                    })
                },
            });
            edges.push(WorkflowEdgeDef { from: cond_id.clone(), to: action_id.clone() });
        }

        prev_node_id = Some(cond_id);
    }

    // ── 5. 审批节点（如果行业需要审批流程） ──
    if config.requires_approval {
        let approval_id = next_id(&mut node_counter, "approval");
        nodes.push(WorkflowNode::Approval(ApprovalNode {
            base: create_node_base(approval_id.clone(), "审批"),
            config: ApprovalNodeConfig {
                message: format!("{industry_id} 行业流程需要审批"),
                approver: None,
                timeout_secs: 86400,
                timeout_action: "auto_reject".to_string(),
                output_var: "approval_result".to_string(),
            },
        }));
        if let Some(prev) = &prev_node_id {
            edges.push(WorkflowEdgeDef { from: prev.clone(), to: approval_id.clone() });
        }
        prev_node_id = Some(approval_id);
    }

    // ── 6. 结束节点 ──
    let end_id = next_id(&mut node_counter, "end");
    nodes.push(WorkflowNode::End(EndNode {
        base: create_node_base(end_id.clone(), "结束"),
        config: EndNodeConfig { output_var: Some("final_result".to_string()) },
    }));
    if let Some(prev) = &prev_node_id {
        edges.push(WorkflowEdgeDef { from: prev.clone(), to: end_id.clone() });
    }

    // ── 构建 WorkflowTemplateData ──
    let workflow_id = format!("{industry_id}_harness_workflow");
    let now = axagent_harness::util_fns::now_ts();

    let edges: Vec<HWorkflowEdge> = edges
        .iter()
        .map(|e| {
            let edge_id = format!("{}_{}_{}", workflow_id, e.from, e.to);
            HWorkflowEdge {
                id: edge_id,
                source: e.from.clone(),
                source_handle: None,
                target: e.to.clone(),
                target_handle: None,
                edge_type: EdgeType::Direct,
                label: None,
            }
        })
        .collect();

    let input_fields = &config.input_fields;
    let input_schema: Option<JsonSchema> = if input_fields.is_empty() {
        None
    } else {
        let mut properties = HashMap::new();
        let mut required_keys = Vec::new();
        for field in input_fields.iter() {
            let prop_type = if field.field_type == "number" {
                "number"
            } else {
                "string"
            };
            properties.insert(
                field.key.clone(),
                JsonSchemaProperty {
                    schema_type: prop_type.to_string(),
                    description: Some(field.label.clone()),
                    default: field.default.as_ref().map(|d| serde_json::json!(d)),
                    enum_values: None,
                    format: None,
                },
            );
            if field.required {
                required_keys.push(field.key.clone());
            }
        }
        Some(JsonSchema {
            schema_type: "object".to_string(),
            description: Some(format!("{} 工作流用户输入", industry_id)),
            properties: Some(properties),
            required: if required_keys.is_empty() {
                None
            } else {
                Some(required_keys)
            },
            items: None,
        })
    };

    let variables: Vec<Variable> = input_fields
        .iter()
        .map(|field| Variable {
            name: field.key.clone(),
            var_type: if field.field_type == "number" {
                "number".to_string()
            } else {
                "string".to_string()
            },
            value: field
                .default
                .as_ref()
                .map(|d| serde_json::json!(d))
                .unwrap_or(serde_json::Value::Null),
            description: Some(field.label.clone()),
            is_secret: false,
        })
        .collect();

    WorkflowTemplateData {
        id: workflow_id,
        name: format!("{} 标准工作流", industry_id),
        description: Some(format!("{} 行业工作流（代码驱动）", industry_id)),
        icon: "⚙️".to_string(),
        cluster_id: None,
        route_path: None,
        tags: vec![industry_id.to_string(), "opc".to_string()],
        version: 6, // v6: 直接生成 WorkflowTemplateData，移除中间层
        is_preset: true,
        is_editable: true,
        is_public: false,
        visibility: Visibility::Public,
        trigger_config: Some(TriggerConfig {
            trigger_type: axagent_harness::workflow_types::TriggerType::Manual,
            config: serde_json::json!({}),
        }),
        nodes,
        edges,
        input_schema,
        output_schema: None,
        variables,
        error_config: None,
        error_workflow_id: None,
        mission_hash: None,
        tool_defs: Vec::new(),
        created_at: now,
        updated_at: now,
    }
}

// ── 辅助类型：供 Adapter 定义工作流元素时使用 ──

/// 验证定义（从 runtime.yaml.validations 映射）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationDef {
    pub field: String,
    pub r#type: String,
    pub error_message: String,
}

/// KPI 计算定义（从 runtime.yaml.kpi_definitions 映射）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KpiCalculationDef {
    pub key: String,
    pub name: String,
}

/// 工作流用户输入字段定义（前端渲染表单 + 后端注入变量）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowInputField {
    /// 字段 key（对应工作流变量名，AgentNode input_mapping 引用此名）
    pub key: String,
    /// 显示标签
    pub label: String,
    /// 字段类型：string / number / textarea
    pub field_type: String,
    /// 是否必填
    pub required: bool,
    /// 占位提示
    pub placeholder: Option<String>,
    /// 默认值
    pub default: Option<String>,
}

/// 业务步骤定义（代码驱动，对齐股票业务）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowStepDef {
    pub name: String,
    pub description: String,
    pub order: i32,
    /// Agent 系统提示词（用于生成 AgentNode）
    pub prompt: Option<String>,
    /// 允许使用的工具列表（用于生成 AgentNode）
    pub tools: Vec<String>,
    /// 绑定的 Agent Profile ID
    pub agent_profile_id: Option<String>,
    /// 错误处理：stop / continue
    pub error_handling: String,
    /// 输入映射：key = 节点变量名, value = 工作流变量名
    pub inputs: std::collections::HashMap<String, String>,
}

impl Default for WorkflowStepDef {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            order: 0,
            prompt: None,
            tools: Vec::new(),
            agent_profile_id: None,
            error_handling: "stop".to_string(),
            inputs: std::collections::HashMap::new(),
        }
    }
}

/// 仪表盘卡片定义（从 runtime.yaml.dashboard_cards 映射）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DashboardCardDef {
    pub id: String,
    pub title: String,
    pub kpi_key: String,
}
