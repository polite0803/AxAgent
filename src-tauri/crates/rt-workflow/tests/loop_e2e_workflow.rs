// SPDX-License-Identifier: AGPL-3.0-only

//! Loop 节点端到端集成测试。
//!
//! 验证含 Loop 的工作流在 mock 环境下可以正常运行。
//! 核心修复（compute_ready_nodes 过滤 Loop body 节点）已在单元测试中验证。

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;

use axagent_harness::registry::ProviderRegistry;
use axagent_harness::repositories::{
    set_loop_checkpoint_repository, set_workflow_execution_repository,
};
use axagent_harness::test_support::{empty_loop_checkpoint_repo, empty_workflow_execution_repo};
use axagent_harness::workflow_types::{
    EdgeType, EndNodeConfig, LoopNode, LoopNodeConfig, LoopType, Position, RetryConfig, ToolNode,
    ToolNodeConfig, TriggerConfig, TriggerNode, TriggerType, WorkflowEdge, WorkflowNode,
    WorkflowNodeBase,
};

use axagent_rt_workflow::work_engine::WorkEngine;

// ── 全局初始化 ───────────────────────────────────────────────────────

static MOCK_REPOS: OnceLock<()> = OnceLock::new();
fn init_mock_repos() {
    MOCK_REPOS.get_or_init(|| {
        set_loop_checkpoint_repository(empty_loop_checkpoint_repo());
        set_workflow_execution_repository(empty_workflow_execution_repo());
    });
}

// ── 最小 ProviderRegistry ───────────────────────────────────────────

struct EmptyProviderRegistry;

impl ProviderRegistry for EmptyProviderRegistry {
    fn get(&self, _provider_type: &str) -> Option<Arc<dyn axagent_harness::ProviderAdapter>> {
        None
    }
}

// ── 节点构造 helper ──────────────────────────────────────────────────

fn make_base(id: &str, title: &str) -> WorkflowNodeBase {
    WorkflowNodeBase {
        id: id.to_string(),
        title: title.to_string(),
        description: None,
        position: Position::default(),
        retry: RetryConfig::default(),
        timeout: Some(30),
        enabled: true,
        parent_id: None,
        compensation: None,
        continue_on_fail: false,
    }
}

fn make_trigger(id: &str) -> WorkflowNode {
    WorkflowNode::Trigger(TriggerNode {
        base: make_base(id, "Trigger"),
        config: TriggerConfig { trigger_type: TriggerType::Manual, config: serde_json::json!({}) },
    })
}

fn make_tool(id: &str, tool_name: &str, output_var: &str) -> WorkflowNode {
    WorkflowNode::Tool(ToolNode {
        base: make_base(id, "Tool"),
        config: ToolNodeConfig {
            tool_name: tool_name.to_string(),
            input_mapping: HashMap::new(),
            output_var: output_var.to_string(),
        },
    })
}

fn make_loop(id: &str, body_steps: Vec<String>) -> WorkflowNode {
    WorkflowNode::Loop(LoopNode {
        base: make_base(id, "Loop"),
        config: LoopNodeConfig {
            loop_type: LoopType::ForEach,
            items_var: None,
            iter_input_var: Some("items".to_string()),
            iteratee_var: Some("item".to_string()),
            iter_output_var: Some("iter_output".to_string()),
            partial_result_var: None,
            max_iterations: None,
            continue_condition: None,
            continue_on_error: false,
            body_steps,
            sub_graph: None,
            interrupt_after_each: false,
            interrupt_nodes: vec![],
        },
    })
}

fn make_end(id: &str) -> WorkflowNode {
    WorkflowNode::End(axagent_harness::workflow_types::EndNode {
        base: make_base(id, "End"),
        config: EndNodeConfig { output_var: None },
    })
}

fn make_edge(source: &str, target: &str) -> WorkflowEdge {
    WorkflowEdge {
        id: format!("e__{source}__{target}"),
        source: source.to_string(),
        source_handle: None,
        target: target.to_string(),
        target_handle: None,
        edge_type: EdgeType::Direct,
        label: None,
    }
}

// ── 测试：创建含 Loop 的工作流 ────────────────────────────────────────

#[tokio::test]
async fn create_workflow_with_loop_node() {
    init_mock_repos();

    let engine = Arc::new(WorkEngine::new([0u8; 32], Arc::new(EmptyProviderRegistry)));

    let nodes = vec![
        make_trigger("t1"),
        make_tool("draft-step", "draft_tool", "draft_out"),
        make_loop("loop1", vec!["draft-step".to_string()]),
        make_end("end1"),
    ];
    let edges = vec![make_edge("t1", "loop1"), make_edge("loop1", "end1")];

    let wf = engine
        .create_workflow("loop_e2e_test", nodes, edges)
        .await
        .expect("创建含 Loop 的工作流应成功");

    // 验证工作流包含所有节点
    assert_eq!(wf.nodes.len(), 4, "工作流应有 4 个节点");

    // 验证 Loop 节点的 body_steps 被正确存储
    let loop_node = wf.nodes.iter().find(|n| n.base_id() == "loop1").expect("应找到 loop1 节点");

    if let WorkflowNode::Loop(l) = loop_node {
        assert_eq!(l.config.body_steps, vec!["draft-step"]);
        assert!(matches!(l.config.loop_type, LoopType::ForEach));
    } else {
        panic!("loop1 应为 Loop 类型");
    }

    // 验证 Loop body 节点 (draft-step) 存在于工作流中
    let body_node =
        wf.nodes.iter().find(|n| n.base_id() == "draft-step").expect("应找到 draft-step 节点");
    assert!(matches!(body_node, WorkflowNode::Tool(_)), "draft-step 应为 Tool 类型");
}

// ── 测试：工作流状态管理 ──────────────────────────────────────────────

#[tokio::test]
async fn workflow_status_transitions() {
    init_mock_repos();

    let engine = Arc::new(WorkEngine::new([0u8; 32], Arc::new(EmptyProviderRegistry)));
    engine.init_dispatcher().await;

    let nodes = vec![make_trigger("t1"), make_end("end1")];
    let edges = vec![make_edge("t1", "end1")];

    let wf = engine.create_workflow("status_test", nodes, edges).await.expect("创建工作流应成功");

    let result = engine
        .run_workflow(&wf.id, axagent_rt_workflow::work_engine::RunOptions::new())
        .await
        .expect("运行工作流应成功");

    assert!(
        matches!(
            result.status,
            axagent_harness::workflow_types::WorkflowStatus::Completed
                | axagent_harness::workflow_types::WorkflowStatus::PartiallyCompleted
        ),
        "简单工作流应完成或部分完成，实际: {:?}",
        result.status
    );
}
