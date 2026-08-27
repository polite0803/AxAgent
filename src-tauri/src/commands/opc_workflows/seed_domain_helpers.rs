// SPDX-License-Identifier: AGPL-3.0-only

//! 领域工作流种子化共享辅助函数
//!
//! v4：从 content_media 样板提升的完整构造器层，禁止退回纯直线 agent 链。
//! 支持拓扑元素：
//! - AgentNode（含 input_mapping / context_sources 软依赖）
//! - ToolNode（工具调用：数据采集 / 导出 / 提交）
//! - ConditionNode（条件分支，true/false 条件边；支持 LLM 动态路由）
//! - LoopNode（循环体，body_steps 驱动，Loop 内节点不连边）
//! - ApprovalNode（人工审批）
//! - MergeNode（分支汇合）
//!
//! 设计约定（与 rt-workflow 执行语义对齐）：
//! - 条件分支边：`edge_cond(id, source, "true"|"false", target, EdgeType::ConditionTrue/False)`
//! - Loop 内节点不通过 edges 连接，由 LoopExecutor 按 body_steps 驱动
//! - 下游引用上游输出一律用「节点 ID.字段」（deps_results 按 node_id 注入）
//! - Agent 的 output_var 不进入 ctx.variables，跨级引用用 context_sources 软依赖

use axagent_harness::capability::Visibility;
use axagent_harness::workflow_types::*;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

/// 全局领域工作流版本号（未重写的领域模板保持旧版本，不被覆盖）
pub const DOMAIN_TEMPLATE_VERSION: i32 = 3;

/// 创建触发节点
pub fn make_trigger(x: f64, y: f64) -> WorkflowNode {
    WorkflowNode::Trigger(TriggerNode {
        base: super::make_base("trigger", "手动启动", "用户选择后启动工作流", x, y),
        config: TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) },
    })
}

/// 创建结束节点
pub fn make_end(x: f64, y: f64) -> WorkflowNode {
    WorkflowNode::End(EndNode {
        base: super::make_base("end", "完成", "", x, y),
        config: EndNodeConfig { output_var: None },
    })
}

/// 创建 Agent 节点。
///
/// 默认注入 `user_input → trigger` 输入映射 + `trigger` 软依赖，
/// 保证首节点能拿到用户输入；下游节点请用 make_agent_node_full
/// 显式声明 input_mapping / context_sources。
pub fn make_agent_node(
    id: &str,
    title: &str,
    prompt: &str,
    tools: Vec<ToolDef>,
    profile_id: Option<&str>,
    output_var: &str,
    x: f64,
    y: f64,
) -> WorkflowNode {
    let mut input_mapping = HashMap::new();
    input_mapping.insert("user_input".to_string(), "trigger".to_string());

    WorkflowNode::Agent(AgentNode {
        base: super::make_base(id, title, "", x, y),
        config: AgentNodeConfig {
            system_prompt: prompt.to_string(),
            context_sources: vec!["trigger".to_string()],
            input_mapping,
            output_var: output_var.to_string(),
            model: None,
            temperature: None,
            max_tokens: None,
            tools: tools.clone(),
            exposed_tools: tools.iter().map(|t| t.name.clone()).collect(),
            output_mode: OutputMode::Json,
            agent_profile_id: profile_id.map(|s| s.to_string()),
            max_tool_rounds: Some(10),
            execution_mode: None,
            rag_source_ids: vec![],
            model_role: Some("opc-worker".to_string()),
            consistency_check: None,
            hallucination_guard: None,
            fallback_model: None,
            task_scene: None,
            stream_chunk_timeout_secs: None,
        },
    })
}

/// 创建带输入映射的 Agent 节点（节点间数据传递）
pub fn make_agent_node_with_inputs(
    id: &str,
    title: &str,
    prompt: &str,
    tools: Vec<ToolDef>,
    profile_id: Option<&str>,
    output_var: &str,
    input_mapping: HashMap<String, String>,
    x: f64,
    y: f64,
) -> WorkflowNode {
    let mut node = make_agent_node(id, title, prompt, tools, profile_id, output_var, x, y);
    if let WorkflowNode::Agent(ref mut agent) = node {
        agent.config.input_mapping = input_mapping;
    }
    node
}

/// 创建支持 input_mapping + context_sources 的 Agent 节点。
///
/// context_sources 是软依赖：按节点 ID 从 workflow.results 取输出注入
/// ctx.variables。Loop body 节点 / 无 edges 直接上游的节点引用跨级输出
/// 时必须用这个（例如引用 `l-x.items` 累积数组）。
pub fn make_agent_node_full(
    id: &str,
    title: &str,
    prompt: &str,
    tools: Vec<ToolDef>,
    profile_id: Option<&str>,
    output_var: &str,
    inputs: Vec<(&str, &str)>,
    context_sources: Vec<&str>,
    x: f64,
    y: f64,
) -> WorkflowNode {
    let mut node = make_agent_node(id, title, prompt, tools, profile_id, output_var, x, y);
    if let WorkflowNode::Agent(ref mut a) = node {
        a.config.input_mapping =
            inputs.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        let mut sources = context_sources.into_iter().map(|s| s.to_string()).collect::<Vec<_>>();
        if !sources.iter().any(|s| s == "trigger") {
            sources.insert(0, "trigger".to_string());
        }
        a.config.context_sources = sources;
    }
    node
}

/// 创建直线边
pub fn edge(id: &str, source: &str, target: &str) -> WorkflowEdge {
    WorkflowEdge {
        id: id.into(),
        source: source.into(),
        source_handle: None,
        target: target.into(),
        target_handle: None,
        edge_type: EdgeType::Direct,
        label: None,
    }
}

/// 条件边（ConditionNode 的 true/false 分支）
pub fn edge_cond(
    id: &str,
    source: &str,
    handle: &str,
    target: &str,
    etype: EdgeType,
) -> WorkflowEdge {
    WorkflowEdge {
        id: id.into(),
        source: source.into(),
        source_handle: Some(handle.into()),
        target: target.into(),
        target_handle: None,
        edge_type: etype,
        label: None,
    }
}

/// 创建工具定义（仅名称，无描述）
pub fn td(name: &str) -> ToolDef {
    ToolDef { name: name.into(), description: None, parameters: None }
}

/// 创建工具定义（带描述）
pub fn td_desc(name: &str, desc: &str) -> ToolDef {
    ToolDef { name: name.into(), description: Some(desc.into()), parameters: None }
}

/// 创建条件判断节点。
///
/// 出边必须用 edge_cond 连接：
/// - `edge_cond(id, cond_node_id, "true", target, EdgeType::ConditionTrue)`
/// - `edge_cond(id, cond_node_id, "false", target, EdgeType::ConditionFalse)`
///
/// 条件引用上游输出用「节点 ID.字段」，例如 `a-analyze.deviation_pct`。
pub fn make_condition_node(
    id: &str,
    title: &str,
    conditions: Vec<Condition>,
    logical_op: LogicalOperator,
    x: f64,
    y: f64,
) -> WorkflowNode {
    WorkflowNode::Condition(ConditionNode {
        base: super::make_base(id, title, "条件判断", x, y),
        config: ConditionNodeConfig {
            conditions,
            logical_op,
            judge_by_llm: None,
            routing_prompt: None,
            routing_model: None,
            confidence_threshold: None,
        },
    })
}

/// 创建 LLM 判断条件节点（judge_by_llm）。
///
/// 用于无法用静态字段表达的分支语义（如"机会是否充分"、"原型是否可玩"）：
/// 引擎调用 LLM 按 routing_prompt 判断上游节点输出，走 true/false 分支。
/// var_path 引用上游节点 ID（deps 注入的节点输出）。
pub fn make_condition_node_llm(
    id: &str,
    title: &str,
    routing_prompt: &str,
    var_path: &str,
    x: f64,
    y: f64,
) -> WorkflowNode {
    // LLM 路由模式下由 routing_prompt 决定分支；var_path 保留用于静态回退与前端展示
    let _ = var_path;
    WorkflowNode::Condition(ConditionNode {
        base: super::make_base(id, title, "LLM 条件判断", x, y),
        config: ConditionNodeConfig {
            conditions: vec![],
            logical_op: LogicalOperator::And,
            judge_by_llm: Some(true),
            routing_prompt: Some(routing_prompt.to_string()),
            routing_model: None,
            confidence_threshold: None,
        },
    })
}

/// 创建循环节点。
///
/// 语义约定：
/// - `iter_input_var` 必须是 deps 注入的节点 ID（Loop 的输入数组）
/// - `iter_output_var`：每轮输出变量名，跨轮累积数组经 `{iter_output_var}__partial` 读取
/// - body_steps：循环体节点 ID 列表，由 LoopExecutor 驱动，**不连边**
/// - 循环体内节点引用上一轮输出用 `{iter_output_var}__partial` 取最后一项
pub fn make_loop_node(
    id: &str,
    title: &str,
    loop_type: LoopType,
    iter_input_var: Option<&str>,
    iteratee_var: Option<&str>,
    iter_output_var: Option<&str>,
    partial_result_var: Option<&str>,
    max_iterations: Option<u32>,
    body_steps: Vec<String>,
    x: f64,
    y: f64,
) -> WorkflowNode {
    WorkflowNode::Loop(LoopNode {
        base: super::make_base(id, title, "循环执行", x, y),
        config: LoopNodeConfig {
            loop_type,
            items_var: None,
            iter_input_var: iter_input_var.map(|s| s.to_string()),
            iteratee_var: iteratee_var.map(|s| s.to_string()),
            iter_output_var: iter_output_var.map(|s| s.to_string()),
            partial_result_var: partial_result_var.map(|s| s.to_string()),
            max_iterations,
            continue_condition: None,
            continue_on_error: false,
            body_steps,
            interrupt_after_each: false,
            interrupt_nodes: vec![],
            sub_graph: None,
        },
    })
}

/// 创建工具调用节点（ToolNode）。
///
/// input_mapping 引用上游节点输出（「节点 ID.字段」或 trigger 用户输入）。
/// 工具名须与运行时注册的工具一致（stock_tool_defs / opc_tool_defs / local_tool_defs）。
pub fn make_tool_node(
    id: &str,
    title: &str,
    tool_name: &str,
    input_mapping: Vec<(&str, &str)>,
    output_var: &str,
    x: f64,
    y: f64,
) -> WorkflowNode {
    let mut im = HashMap::new();
    for (k, v) in input_mapping {
        im.insert(k.to_string(), v.to_string());
    }
    WorkflowNode::Tool(ToolNode {
        base: super::make_base(id, title, "工具调用", x, y),
        config: ToolNodeConfig {
            tool_name: tool_name.into(),
            input_mapping: im,
            output_var: output_var.into(),
        },
    })
}

/// 创建人工审批节点。
///
/// 建议紧随条件分支后：条件满足（如金额超阈值）时进入人工审批。
/// 审批通过/拒绝由前端处理回环，工作流内以通过路径继续。
pub fn make_approval_node(
    id: &str,
    title: &str,
    message: &str,
    approver: Option<&str>,
    timeout_secs: u64,
    output_var: &str,
    x: f64,
    y: f64,
) -> WorkflowNode {
    WorkflowNode::Approval(ApprovalNode {
        base: super::make_base(id, title, "人工审批", x, y),
        config: ApprovalNodeConfig {
            message: message.into(),
            approver: approver.map(|s| s.to_string()),
            timeout_secs,
            timeout_action: "reject".to_string(),
            output_var: output_var.into(),
        },
    })
}

/// 创建分支汇合节点（MergeNode）。
///
/// 多条条件边汇入同一 Merge 后再继续主链路，避免下游节点重复执行。
/// merge_type 默认 All：等待所有上游分支完成后继续。
pub fn make_merge_node(id: &str, title: &str, x: f64, y: f64) -> WorkflowNode {
    WorkflowNode::Merge(MergeNode {
        base: super::make_base(id, title, "分支汇合", x, y),
        config: MergeNodeConfig {
            merge_type: MergeStrategy::All,
            inputs: vec![],
            auto_inputs_from_branches: true,
        },
    })
}

/// 种子化单个领域工作流模板。
/// 版本保护：只有版本升级时覆盖，用户编辑不被启动覆盖。
pub(crate) async fn seed_domain_template(
    db: &DatabaseConnection,
    template: WorkflowTemplateData,
) -> Result<bool, String> {
    let should_seed = super::check_template_version(db, &template.id, template.version).await?;
    if !should_seed {
        return Ok(false);
    }
    super::upsert_template(db, template).await?;
    Ok(true)
}

/// 构建 WorkflowTemplateData（兼容旧版，version 使用公共 DOMAIN_TEMPLATE_VERSION）
pub fn build_domain_template(
    id: &str,
    name: &str,
    description: &str,
    icon: &str,
    tags: Vec<String>,
    _profile_id: &str,
    nodes: Vec<WorkflowNode>,
    edges: Vec<WorkflowEdge>,
) -> WorkflowTemplateData {
    build_domain_template_rich(
        id,
        name,
        description,
        icon,
        tags,
        DOMAIN_TEMPLATE_VERSION,
        nodes,
        edges,
        Vec::new(),
    )
}

/// 领域输入字段（映射为工作流 variables + input_schema）
#[derive(Debug, Clone)]
pub struct DomainInputField {
    pub key: &'static str,
    pub label: &'static str,
    pub field_type: &'static str,
    pub required: bool,
}

/// 构建 WorkflowTemplateData（完整版：自定义版本号 + 输入字段）。
///
/// 重设计的领域文件必须用本函数并传入**新版本号**（旧版本 + 1），
/// 否则 check_template_version 会因版本未提升而跳过覆盖，模板仍是旧直线链。
pub fn build_domain_template_rich(
    id: &str,
    name: &str,
    description: &str,
    icon: &str,
    tags: Vec<String>,
    version: i32,
    nodes: Vec<WorkflowNode>,
    edges: Vec<WorkflowEdge>,
    input_fields: Vec<DomainInputField>,
) -> WorkflowTemplateData {
    let now = chrono::Utc::now().timestamp_millis();

    let mut properties: HashMap<String, JsonSchemaProperty> = HashMap::new();
    let mut required_keys = Vec::new();
    for f in &input_fields {
        properties.insert(
            f.key.to_string(),
            JsonSchemaProperty {
                schema_type: if f.field_type == "number" {
                    "number".to_string()
                } else {
                    "string".to_string()
                },
                description: Some(f.label.to_string()),
                default: None,
                enum_values: None,
                format: None,
            },
        );
        if f.required {
            required_keys.push(f.key.to_string());
        }
    }
    let input_schema = if input_fields.is_empty() {
        None
    } else {
        Some(JsonSchema {
            schema_type: "object".to_string(),
            description: Some(name.to_string()),
            properties: Some(properties),
            required: if required_keys.is_empty() {
                None
            } else {
                Some(required_keys)
            },
            items: None,
        })
    };

    let variables = input_fields
        .iter()
        .map(|f| Variable {
            name: f.key.to_string(),
            var_type: if f.field_type == "number" {
                "number".to_string()
            } else {
                "string".to_string()
            },
            value: serde_json::Value::Null,
            description: Some(f.label.to_string()),
            is_secret: false,
        })
        .collect();

    WorkflowTemplateData {
        id: id.to_string(),
        name: name.to_string(),
        description: Some(description.to_string()),
        icon: icon.to_string(),
        cluster_id: None,
        route_path: None,
        tags,
        version,
        is_preset: true,
        is_editable: true,
        is_public: false,
        visibility: Visibility::Public,
        trigger_config: Some(TriggerConfig {
            trigger_type: TriggerType::Manual,
            config: serde_json::json!({}),
        }),
        nodes,
        edges,
        input_schema,
        output_schema: None,
        variables,
        error_config: Some(ErrorConfig {
            retry_policy: None,
            on_failure: OnFailureAction::RetryThenAbort,
            error_branch: None,
            compensation_steps: None,
        }),
        error_workflow_id: None,
        mission_hash: None,
        tool_defs: vec![],
        created_at: now,
        updated_at: now,
    }
}
