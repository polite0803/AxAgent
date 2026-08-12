// SPDX-License-Identifier: AGPL-3.0-only

//! 领域工作流种子化共享辅助函数
//!
//! 提供构建 WorkflowNode/Edge 的辅助函数，与行业 seed 文件模式一致。
//! 各领域 seed 文件通过本模块函数完成种子化。

use axagent_harness::workflow_types::{
    AgentNode, AgentNodeConfig, EdgeType, EndNode, EndNodeConfig, ErrorConfig, OnFailureAction,
    OutputMode, ToolDef, TriggerConfig, TriggerNode, TriggerType, WorkflowEdge, WorkflowNode,
    WorkflowTemplateData,
};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

/// 全局领域工作流版本号（与行业工作流一致）
pub const DOMAIN_TEMPLATE_VERSION: i32 = 2;

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

/// 创建 Agent 节点
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
    WorkflowNode::Agent(AgentNode {
        base: super::make_base(id, title, "", x, y),
        config: AgentNodeConfig {
            system_prompt: prompt.to_string(),
            context_sources: vec![],
            input_mapping: HashMap::new(),
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

/// 创建带输入映射的 Agent 节点
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

/// 创建工具定义
pub fn td(name: &str) -> ToolDef {
    ToolDef { name: name.into(), description: None, parameters: None }
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

/// 构建 WorkflowTemplateData（简化版，适合简单线性链）
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
    let now = chrono::Utc::now().timestamp_millis();
    WorkflowTemplateData {
        id: id.to_string(),
        name: name.to_string(),
        description: Some(description.to_string()),
        icon: icon.to_string(),
        tags,
        version: DOMAIN_TEMPLATE_VERSION,
        is_preset: true,
        is_editable: true,
        is_public: false,
        trigger_config: Some(TriggerConfig {
            trigger_type: TriggerType::Manual,
            config: serde_json::json!({}),
        }),
        nodes,
        edges,
        input_schema: None,
        output_schema: None,
        variables: vec![],
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
