// SPDX-License-Identifier: AGPL-3.0-only

use async_trait::async_trait;
use axagent_harness::workflow_types::WorkflowNode;

use crate::work_engine::execution_state::ExecutionState;
use crate::work_engine::node_executor_trait::{
    ApprovalRequest, NodeControl, NodeError, NodeExecutorTrait, NodeOutput,
};

/// 审批节点执行器。
///
/// 修复 P0-1（审批断链）：
/// - 输出固定含 `result: false`（待决默认），供条件边（true→下一节点 / false→end）判定；
/// - 返回 `NodeControl::Suspend` 挂起整个工作流并写入 `workflow_approvals` 表，
///   等待人工通过 `resume_approval` 决策；
/// - 审批通过后由命令层把节点结果覆写为 `{"status":"approved","result":true}`，
///   引擎恢复后 true 分支（下一节点）激活；拒绝则覆写 `result:false` 走 end。
pub struct ApprovalExecutor;

impl ApprovalExecutor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ApprovalExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeExecutorTrait for ApprovalExecutor {
    fn node_type(&self) -> &'static str {
        "approval"
    }
    async fn execute(
        &self,
        node: &WorkflowNode,
        ctx: &ExecutionState,
    ) -> Result<NodeOutput, NodeError> {
        let WorkflowNode::Approval(n) = node else {
            return Err(NodeError::type_mismatch("approval", self.node_type()));
        };
        let c = &n.config;
        let node_id = node.base_id().to_string();
        let output = serde_json::json!({
            "status": "pending",
            "result": false,
            "message": c.message,
            "timeout_secs": c.timeout_secs,
            "node_id": node_id,
        });
        Ok(NodeOutput {
            output: output.clone(),
            control: Some(NodeControl::Suspend {
                resume_token: node_id.clone(),
                approval: ApprovalRequest {
                    execution_id: ctx.execution_id.clone(),
                    node_id,
                    title: n.base.title.clone(),
                    message: c.message.clone(),
                    approver: c.approver.clone(),
                    channels: vec![],
                    payload: output,
                    timeout_secs: c.timeout_secs,
                    timeout_action: c.timeout_action.clone(),
                },
            }),
            output_var: if c.output_var.is_empty() {
                None
            } else {
                Some(c.output_var.clone())
            },
        })
    }
}
