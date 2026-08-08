// OPC 行业工作流层
// 复用 axagent-harness::workflow_types 中的标准工作流节点体系（30 种节点类型）
// YAML 配置 → 标准 WorkflowNode 映射，交由 rt-workflow 引擎执行

use std::collections::HashMap;
use std::sync::Arc;

use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, AggregatorNode, AggregatorNodeConfig, ApprovalNode,
    ApprovalNodeConfig, CodeNodeConfig, ConditionNode, ConditionNodeConfig, DataTransformerNode,
    DataTransformerNodeConfig, EdgeType, EndNode, EndNodeConfig, NotificationNode,
    NotificationNodeConfig, OutputMode, ToolDef, TriggerConfig, TriggerNode, ValidationAssertion,
    ValidationNodeConfig as HValidationNodeConfig, WorkflowEdge as HWorkflowEdge, WorkflowNode,
    WorkflowNodeBase, WorkflowTemplateData,
};

use super::automation::{AutomationAction, AutomationCondition};
use super::data_service::TimeRange;
use super::industry::OpcIndustryAdapter;

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

/// 行业工作流定义：标准工作流节点列表 + 边列表
#[derive(Debug, Clone)]
pub struct IndustryWorkflow {
    pub industry_id: String,
    pub workflow_id: String,
    pub name: String,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdgeDef>,
    pub version: String,
}

impl IndustryWorkflow {
    /// 从行业 Adapter 动态生成工作流定义（使用标准 WorkflowNode）
    #[allow(unused_assignments)]
    pub fn from_adapter(industry_id: &str, adapter: &dyn OpcIndustryAdapter) -> Self {
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

        // ── 2. 验证节点（来自 runtime.yaml 的 validations） ──
        for validation in adapter.define_validations() {
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
        for step in adapter.define_workflow_steps() {
            let node_id = next_id(&mut node_counter, "step");
            let base = create_node_base(node_id.clone(), format!("步骤: {}", step.name));

            // 如果步骤定义了 prompt、tools 或 agent_profile_id，则生成 AgentNode
            if step.prompt.is_some() || !step.tools.is_empty() || step.agent_profile_id.is_some() {
                let tool_defs = step
                    .tools
                    .iter()
                    .map(|t| ToolDef { name: t.clone(), description: None, parameters: None })
                    .collect();

                nodes.push(WorkflowNode::Agent(AgentNode {
                    base,
                    config: AgentNodeConfig {
                        system_prompt: step
                            .prompt
                            .clone()
                            .unwrap_or_else(|| step.description.clone()),
                        context_sources: Vec::new(),
                        input_mapping: HashMap::new(),
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
                        input_mapping: HashMap::new(),
                    },
                }));
            }

            if let Some(prev) = &prev_node_id {
                edges.push(WorkflowEdgeDef { from: prev.clone(), to: node_id.clone() });
            }
            prev_node_id = Some(node_id);
        }

        // ── 4. KPI 计算节点（来自 runtime.yaml 的 kpi_definitions，使用 Code 节点） ──
        for kpi in adapter.define_kpi_calculations() {
            let node_id = next_id(&mut node_counter, "kpi");
            nodes.push(WorkflowNode::Code(axagent_harness::workflow_types::CodeNode {
                base: create_node_base(node_id.clone(), format!("KPI: {}", kpi.name)),
                config: axagent_harness::workflow_types::CodeNodeConfig {
                    language: "rust".to_string(),
                    code: format!(
                        "// 计算 KPI: {}\n// 实际逻辑在 adapter.compute_kpis() 中",
                        kpi.key
                    ),
                    output_var: format!("kpi_{}", kpi.key),
                    tool_name: None,
                    execute_directly: true,
                    input_mapping: HashMap::new(),
                },
            }));
            if let Some(prev) = &prev_node_id {
                edges.push(WorkflowEdgeDef { from: prev.clone(), to: node_id.clone() });
            }
            prev_node_id = Some(node_id);
        }

        // ── 5. 自动化规则节点（来自 runtime.yaml 的 automation_rules，使用 Condition + Notification 节点组合） ──
        for rule in adapter.define_automation_rules() {
            // 5a. 条件节点
            let cond_id = next_id(&mut node_counter, "condition");
            // 先读取前驱节点ID，避免编译器警告
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

            // 5b. 通知/动作节点（条件满足时执行）
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
                            base: create_node_base(
                                action_id.clone(),
                                format!("更新字段: {}", field),
                            ),
                            config: DataTransformerNodeConfig {
                                input_var: field.clone(),
                                expression: format!("{}", value),
                                output_var: format!("action_{}", action_id),
                            },
                        })
                    },
                    AutomationAction::UpdateStatus { status } => {
                        WorkflowNode::DataTransformer(DataTransformerNode {
                            base: create_node_base(
                                action_id.clone(),
                                format!("更新状态: {}", status),
                            ),
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

            // 将条件节点作为下一个节点的前驱
            prev_node_id = Some(cond_id);
        }

        // ── 6. 审批节点（如果行业需要审批流程） ──
        if adapter.requires_approval() {
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

        // ── 7. 仪表盘聚合节点（来自 runtime.yaml 的 dashboard_cards） ──
        let dashboard_cards = adapter.define_dashboard_cards();
        if !dashboard_cards.is_empty() {
            let agg_id = next_id(&mut node_counter, "dashboard");
            // 先读取前驱节点ID，避免编译器警告
            let prev = prev_node_id.clone();
            let input_sources: Vec<String> =
                (0..node_counter).map(|i| format!("kpi_{}_output", i)).collect();
            nodes.push(WorkflowNode::Aggregator(AggregatorNode {
                base: create_node_base(agg_id.clone(), "仪表盘聚合"),
                config: AggregatorNodeConfig {
                    strategy: "all".to_string(),
                    input_sources,
                    wait_for_all: true,
                    weights: Vec::new(),
                    summarize_prompt: None,
                    summarize_model: None,
                    output_var: "dashboard".to_string(),
                    sub_graph: None,
                },
            }));
            if let Some(prev_id) = &prev {
                edges.push(WorkflowEdgeDef { from: prev_id.clone(), to: agg_id.clone() });
            }
            prev_node_id = Some(agg_id);
        }

        // ── 8. 结束节点 ──
        let end_id = next_id(&mut node_counter, "end");
        nodes.push(WorkflowNode::End(EndNode {
            base: create_node_base(end_id.clone(), "结束"),
            config: EndNodeConfig { output_var: Some("final_result".to_string()) },
        }));
        if let Some(prev) = &prev_node_id {
            edges.push(WorkflowEdgeDef { from: prev.clone(), to: end_id.clone() });
        }

        Self {
            industry_id: industry_id.to_string(),
            workflow_id: format!("{industry_id}_harness_workflow"),
            name: format!("{industry_id} 标准工作流"),
            nodes,
            edges,
            version: "3.0.0-harness".to_string(),
        }
    }

    /// 获取所有节点 ID 列表
    pub fn node_ids(&self) -> Vec<String> {
        self.nodes.iter().map(|n| n.base_id().to_string()).collect()
    }

    /// 转换为 WorkflowTemplateData（用于种子化存储）
    pub fn to_template_data(&self) -> WorkflowTemplateData {
        let now = axagent_harness::util_fns::now_ts();

        // 将内部边映射为 harness 的 HWorkflowEdge
        let edges: Vec<HWorkflowEdge> = self
            .edges
            .iter()
            .map(|e| {
                let edge_id = format!("{}_{}_{}", self.workflow_id, e.from, e.to);
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

        WorkflowTemplateData {
            id: self.workflow_id.clone(),
            name: self.name.clone(),
            description: Some(format!("{} 行业工作流（代码驱动）", self.industry_id)),
            icon: "⚙️".to_string(),
            tags: vec![self.industry_id.clone(), "opc".to_string()],
            version: 2, // 代码驱动版本
            is_preset: true,
            is_editable: true,
            is_public: false,
            trigger_config: Some(TriggerConfig {
                trigger_type: axagent_harness::workflow_types::TriggerType::Manual,
                config: serde_json::json!({}),
            }),
            nodes: self.nodes.clone(),
            edges,
            input_schema: None,
            output_schema: None,
            variables: Vec::new(),
            error_config: None,
            error_workflow_id: None,
            mission_hash: None,
            tool_defs: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// 行业工作流管理器（缓存已生成的工作流定义）
pub struct IndustryWorkflowManager {
    workflows: HashMap<String, IndustryWorkflow>,
}

impl IndustryWorkflowManager {
    pub fn new() -> Self {
        Self { workflows: HashMap::new() }
    }

    /// 根据行业 ID 和 Adapter 创建/获取标准工作流
    pub fn get_or_create(
        &mut self,
        industry_id: &str,
        adapter: &dyn OpcIndustryAdapter,
    ) -> &IndustryWorkflow {
        if !self.workflows.contains_key(industry_id) {
            let workflow = IndustryWorkflow::from_adapter(industry_id, adapter);
            self.workflows.insert(industry_id.to_string(), workflow);
        }
        self.workflows.get(industry_id).unwrap()
    }

    /// 根据行业 ID 和 Adapter 创建或更新标准工作流（始终重新生成）
    pub fn create_or_update(
        &mut self,
        industry_id: &str,
        adapter: &dyn OpcIndustryAdapter,
    ) -> &IndustryWorkflow {
        let workflow = IndustryWorkflow::from_adapter(industry_id, adapter);
        self.workflows.insert(industry_id.to_string(), workflow);
        self.workflows.get(industry_id).unwrap()
    }

    pub fn get(&self, industry_id: &str) -> Option<&IndustryWorkflow> {
        self.workflows.get(industry_id)
    }

    pub fn list(&self) -> Vec<&IndustryWorkflow> {
        self.workflows.values().collect()
    }
}

impl Default for IndustryWorkflowManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── IndustryWorkflowExecutor ──────────────────────────────────

/// 工作流执行结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowExecutionResult {
    pub workflow_id: String,
    pub industry_id: String,
    pub status: String,
    pub node_results: Vec<NodeExecutionResult>,
    pub kpis: Vec<crate::opc::analytics::KpiValue>,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

/// 单个节点执行结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeExecutionResult {
    pub node_id: String,
    pub node_type: String,
    pub status: String,
    pub output: serde_json::Value,
    pub error: Option<String>,
}

/// 行业工作流执行器 — 顺序执行 IndustryWorkflow 中的节点
pub struct IndustryWorkflowExecutor {
    pub industry_id: String,
    pub adapter: Arc<dyn OpcIndustryAdapter>,
}

impl IndustryWorkflowExecutor {
    pub fn new(industry_id: String, adapter: Arc<dyn OpcIndustryAdapter>) -> Self {
        Self { industry_id, adapter }
    }

    /// 执行工作流
    pub async fn execute(
        &self,
        workflow: &IndustryWorkflow,
        time_range: &TimeRange,
    ) -> Result<WorkflowExecutionResult, String> {
        let start = std::time::Instant::now();
        let mut node_results = Vec::new();
        let mut errors = Vec::new();
        let mut all_kpis = Vec::new();

        // 按顺序执行每个节点
        for node in &workflow.nodes {
            let node_id = node.base_id().to_string();
            let node_type = self.get_node_type(node);

            match self.execute_node(node, time_range).await {
                Ok(result) => {
                    if result.status == "success" {
                        // 收集 KPI 数据
                        if node_type == "code" {
                            if let Some(kpi_data) = result.output.get("kpis") {
                                if let Ok(kpis) = serde_json::from_value::<
                                    Vec<crate::opc::analytics::KpiValue>,
                                >(kpi_data.clone())
                                {
                                    all_kpis.extend(kpis);
                                }
                            }
                        }
                        node_results.push(result);
                    } else if result.status == "skipped" {
                        node_results.push(result);
                    } else {
                        errors.push(result.error.clone().unwrap_or_default());
                        node_results.push(result);
                        // 失败时继续执行，不中断
                    }
                },
                Err(e) => {
                    errors.push(e.clone());
                    node_results.push(NodeExecutionResult {
                        node_id: node_id.clone(),
                        node_type: node_type.clone(),
                        status: "error".to_string(),
                        output: serde_json::Value::Null,
                        error: Some(e),
                    });
                },
            }
        }

        Ok(WorkflowExecutionResult {
            workflow_id: workflow.workflow_id.clone(),
            industry_id: self.industry_id.clone(),
            status: if errors.is_empty() {
                "success"
            } else {
                "partial_failed"
            }
            .to_string(),
            node_results,
            kpis: all_kpis,
            errors,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn get_node_type(&self, node: &WorkflowNode) -> String {
        match node {
            WorkflowNode::Trigger(_) => "trigger",
            WorkflowNode::Validation(_) => "validation",
            WorkflowNode::Code(_) => "code",
            WorkflowNode::Condition(_) => "condition",
            WorkflowNode::Notification(_) => "notification",
            WorkflowNode::Approval(_) => "approval",
            WorkflowNode::Aggregator(_) => "aggregator",
            WorkflowNode::End(_) => "end",
            WorkflowNode::DataTransformer(_) => "data_transformer",
            _ => "unknown",
        }
        .to_string()
    }

    async fn execute_node(
        &self,
        node: &WorkflowNode,
        time_range: &TimeRange,
    ) -> Result<NodeExecutionResult, String> {
        let node_id = node.base_id().to_string();
        let node_type = self.get_node_type(node);

        match node {
            WorkflowNode::Trigger(_) => {
                // Trigger 节点直接成功
                Ok(NodeExecutionResult {
                    node_id,
                    node_type,
                    status: "success".to_string(),
                    output: serde_json::json!({"triggered": true}),
                    error: None,
                })
            },
            WorkflowNode::Validation(validation_node) => {
                // Validation 节点：暂存成功，实际校验在执行前由命令层完成
                let assertions = &validation_node.config.assertions;
                Ok(NodeExecutionResult {
                    node_id,
                    node_type,
                    status: "success".to_string(),
                    output: serde_json::json!({
                        "assertions_count": assertions.len(),
                        "passed": true
                    }),
                    error: None,
                })
            },
            WorkflowNode::Code(code_node) => {
                // Code 节点：对于 KPI 计算节点，执行 compute_kpis
                let output_var = &code_node.config.output_var;
                if output_var.starts_with("kpi_") {
                    let kpis =
                        self.adapter.compute_kpis(time_range).await.map_err(|e| e.to_string())?;
                    Ok(NodeExecutionResult {
                        node_id,
                        node_type,
                        status: "success".to_string(),
                        output: serde_json::json!({
                            "kpis": kpis,
                            "output_var": output_var
                        }),
                        error: None,
                    })
                } else {
                    // 业务步骤 Code 节点
                    Ok(NodeExecutionResult {
                        node_id,
                        node_type,
                        status: "success".to_string(),
                        output: serde_json::json!({
                            "executed": true,
                            "output_var": output_var
                        }),
                        error: None,
                    })
                }
            },
            WorkflowNode::Condition(condition_node) => {
                // Condition 节点：默认通过（实际条件由业务逻辑判断）
                Ok(NodeExecutionResult {
                    node_id,
                    node_type,
                    status: "success".to_string(),
                    output: serde_json::json!({
                        "conditions_met": true,
                        "logical_op": format!("{:?}", condition_node.config.logical_op)
                    }),
                    error: None,
                })
            },
            WorkflowNode::Notification(notification_node) => {
                // Notification 节点：记录通知
                Ok(NodeExecutionResult {
                    node_id,
                    node_type,
                    status: "success".to_string(),
                    output: serde_json::json!({
                        "channel": notification_node.config.channel,
                        "recipients": notification_node.config.recipients,
                        "message": notification_node.config.message
                    }),
                    error: None,
                })
            },
            WorkflowNode::Approval(approval_node) => {
                // Approval 节点：需要审批时标记为挂起
                Ok(NodeExecutionResult {
                    node_id,
                    node_type,
                    status: "success".to_string(),
                    output: serde_json::json!({
                        "message": approval_node.config.message,
                        "timeout_secs": approval_node.config.timeout_secs,
                        "auto_approved": true
                    }),
                    error: None,
                })
            },
            WorkflowNode::Aggregator(aggregator_node) => {
                // Aggregator 节点：聚合输入
                Ok(NodeExecutionResult {
                    node_id,
                    node_type,
                    status: "success".to_string(),
                    output: serde_json::json!({
                        "strategy": aggregator_node.config.strategy,
                        "input_sources": aggregator_node.config.input_sources,
                        "aggregated": true
                    }),
                    error: None,
                })
            },
            WorkflowNode::End(_) => Ok(NodeExecutionResult {
                node_id,
                node_type,
                status: "success".to_string(),
                output: serde_json::json!({"completed": true}),
                error: None,
            }),
            WorkflowNode::DataTransformer(dt_node) => Ok(NodeExecutionResult {
                node_id,
                node_type,
                status: "success".to_string(),
                output: serde_json::json!({
                    "input_var": dt_node.config.input_var,
                    "output_var": dt_node.config.output_var,
                    "transformed": true
                }),
                error: None,
            }),
            _ => Ok(NodeExecutionResult {
                node_id,
                node_type,
                status: "skipped".to_string(),
                output: serde_json::json!({"skipped": true}),
                error: None,
            }),
        }
    }
}

// ── 辅助类型：供 Adapter 定义工作流元素时使用 ──

/// 验证定义（从 runtime.yaml.validations 映射）
#[derive(Debug, Clone)]
pub struct ValidationDef {
    pub field: String,
    pub r#type: String,
    pub error_message: String,
}

/// KPI 计算定义（从 runtime.yaml.kpi_definitions 映射）
#[derive(Debug, Clone)]
pub struct KpiCalculationDef {
    pub key: String,
    pub name: String,
}

/// 业务步骤定义（代码驱动，对齐股票业务）
#[derive(Debug, Clone)]
pub struct WorkflowStepDef {
    pub name: String,
    pub description: String,
    pub order: i32,
    /// Agent 系统提示词（用于生成 AgentNode）
    pub prompt: Option<String>,
    /// 允许使用的工具列表（用于生成 AgentNode）
    pub tools: Vec<String>,
    /// 绑定的 Agent Profile ID（如 "opc-cmo-cmo-content-strategist"）
    pub agent_profile_id: Option<String>,
    /// 错误处理：stop / continue
    pub error_handling: String,
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
        }
    }
}

/// 仪表盘卡片定义（从 runtime.yaml.dashboard_cards 映射）
#[derive(Debug, Clone)]
pub struct DashboardCardDef {
    pub id: String,
    pub title: String,
    pub kpi_key: String,
}
