// SPDX-License-Identifier: AGPL-3.0-only

use async_trait::async_trait;
use axagent_harness::node_output_status::NodeOutputStatus;
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
        let approval_request = ApprovalRequest {
            execution_id: ctx.execution_id.clone(),
            node_id: node.base_id().to_string(),
            title: n.base.title.clone(),
            message: c.message.clone(),
            approver: c.approver.clone(),
            channels: vec!["ui".to_string()],
            payload: serde_json::json!({
                "node_id": node.base_id(),
                "workflow_id": ctx.workflow_id,
                "message": c.message,
                "timeout_secs": c.timeout_secs,
                "output_var": c.output_var,
            }),
            timeout_secs: c.timeout_secs,
            timeout_action: c.timeout_action.clone(),
        };

        let resume_token = format!("approval-{}-{}", ctx.execution_id, node.base_id());

        // 使用结构化枚举生成输出状态
        let status = NodeOutputStatus::waiting_for_approval(
            Some(serde_json::to_value(&approval_request).unwrap_or_default()),
            Some(c.message.clone()),
            Some(c.timeout_secs),
        );

        let mut output = serde_json::to_value(&status).unwrap_or_else(|e| {
            tracing::warn!("ApprovalExecutor: 序列化 NodeOutputStatus 失败: {e}");
            serde_json::json!({"status": "waiting_for_approval"})
        });

        // 添加额外的输出字段
        if let Some(obj) = output.as_object_mut() {
            obj.insert(
                "node_id".to_string(),
                serde_json::Value::String(node.base_id().to_string()),
            );
            obj.insert(
                "pause_reason".to_string(),
                serde_json::Value::String("approval".to_string()),
            );
        }

        Ok(NodeOutput {
            output,
            control: Some(NodeControl::Suspend { resume_token, approval: approval_request }),
            output_var: if c.output_var.is_empty() {
                None
            } else {
                Some(c.output_var.clone())
            },
        })
    }
}
