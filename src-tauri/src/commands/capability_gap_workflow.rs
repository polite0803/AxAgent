// SPDX-License-Identifier: AGPL-3.0-only

//! 能力补齐工作流模板生成器
//!
//! 当认知编排器判定需要补齐能力（CapabilityMissing）且用户同意后，
//! 生成实际的工作流模板（nodes/edges/variables），写入 `workflow_templates` 表，
//! 使 WorkEngine 可执行该模板。

use crate::AppState;
use crate::commands::error::{CommandError, ErrorCategory};
use axagent_dao::repo::workflow_template::{
    build_active_model_from_data, insert_workflow_template,
};
use axagent_harness::runtime_types::capability_gap::CapabilityGapProposal;
use axagent_harness::workflow_types::WorkflowTemplateData;

/// 从用户原始输入 + 补齐提议，生成工作流模板并写入数据库。
pub(crate) async fn generate_gap_workflow_template(
    state: &AppState,
    input: &str,
    proposal: &CapabilityGapProposal,
) -> Result<WorkflowTemplateData, CommandError> {
    // 构建最小可用的补齐工作流模板（Trigger + LLM + End）
    let template = build_minimal_gap_template(input, proposal);

    // 写入 workflow_templates 表
    let active_model = build_active_model_from_data(&template);
    insert_workflow_template(state.harness.db(), active_model).await.map_err(|e| {
        CommandError::new(axagent_harness::error_codes::cognitive::GAP_PROPOSAL_PENDING)
            .with_category(ErrorCategory::Unrecoverable)
            .with_detail(format!("工作流模板创建失败: {e}"))
    })?;

    Ok(template)
}

/// 构建最小可用的补齐工作流模板（Trigger + LLM + End）
fn build_minimal_gap_template(
    input: &str,
    proposal: &CapabilityGapProposal,
) -> WorkflowTemplateData {
    use axagent_harness::capability::Visibility;
    use axagent_harness::workflow_types::{
        EdgeType, EndNode, EndNodeConfig, LLMNode, LLMNodeConfig, Position, RetryConfig,
        TriggerConfig, TriggerNode, TriggerType, WorkflowEdge, WorkflowNode, WorkflowNodeBase,
    };
    use chrono::Utc;

    let now = Utc::now().timestamp_millis();

    // 构建节点基础信息（使用与 cognitive_router_init 一致的 simple_base 风格）
    let trigger_base = WorkflowNodeBase {
        id: "trigger".to_string(),
        title: "开始".to_string(),
        description: Some("用户触发".to_string()),
        position: Position { x: 100.0, y: 80.0 },
        retry: RetryConfig::default(),
        timeout: None,
        enabled: true,
        parent_id: None,
        compensation: None,
        continue_on_fail: false,
    };

    let llm_base = WorkflowNodeBase {
        id: "llm_process".to_string(),
        title: proposal.title.trim().to_string(),
        description: Some(proposal.proposal.clone()),
        position: Position { x: 400.0, y: 80.0 },
        retry: RetryConfig::default(),
        timeout: None,
        enabled: true,
        parent_id: None,
        compensation: None,
        continue_on_fail: false,
    };

    let end_base = WorkflowNodeBase {
        id: "end".to_string(),
        title: "结束".to_string(),
        description: Some("输出结果".to_string()),
        position: Position { x: 700.0, y: 80.0 },
        retry: RetryConfig::default(),
        timeout: None,
        enabled: true,
        parent_id: None,
        compensation: None,
        continue_on_fail: false,
    };

    // 构建 LLM 节点配置
    let llm_config = LLMNodeConfig {
        model: String::new(),
        prompt: format!("用户请求：{}\n\n请根据以下描述处理：{}", input, proposal.proposal),
        messages: None,
        temperature: Some(0.7),
        max_tokens: Some(2048),
        tools: None,
        functions: None,
        consistency_check: None,
        max_context_tokens: None,
        reserved_output_tokens: None,
    };

    WorkflowTemplateData {
        id: format!("evolution:workflow:{}", proposal.id),
        name: proposal.title.trim().to_string(),
        description: Some(proposal.proposal.clone()),
        icon: "evolution".to_string(),
        tags: vec![
            "auto_evolved".to_string(),
            "capability_gap".to_string(),
            "evolvable".to_string(),
        ],
        version: 1,
        is_preset: false,
        is_editable: true,
        is_public: false,
        visibility: Visibility::Public,
        trigger_config: Some(TriggerConfig {
            trigger_type: TriggerType::Manual,
            config: serde_json::json!({}),
        }),
        nodes: vec![
            WorkflowNode::Trigger(TriggerNode {
                base: trigger_base,
                config: TriggerConfig {
                    trigger_type: TriggerType::Manual,
                    config: serde_json::json!({}),
                },
            }),
            WorkflowNode::Llm(LLMNode { base: llm_base, config: llm_config }),
            WorkflowNode::End(EndNode {
                base: end_base,
                config: EndNodeConfig { output_var: None },
            }),
        ],
        edges: vec![
            WorkflowEdge {
                id: "e1".to_string(),
                source: "trigger".to_string(),
                source_handle: None,
                target: "llm_process".to_string(),
                target_handle: None,
                edge_type: EdgeType::Direct,
                label: None,
            },
            WorkflowEdge {
                id: "e2".to_string(),
                source: "llm_process".to_string(),
                source_handle: None,
                target: "end".to_string(),
                target_handle: None,
                edge_type: EdgeType::Direct,
                label: None,
            },
        ],
        input_schema: None,
        output_schema: None,
        variables: vec![],
        error_config: None,
        error_workflow_id: None,
        tool_defs: vec![],
        mission_hash: None,
        created_at: now,
        updated_at: now,
    }
}
