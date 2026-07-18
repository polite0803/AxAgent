// SPDX-License-Identifier: AGPL-3.0-only

//! 工作流共享类型定义。
//!
//! 节点类型统一为 axagent_harness::workflow_types::WorkflowNode（28 种），
//! 执行统一由 WorkEngine + NodeDispatcher 负责。
//!
//! 运行时执行态 DTO(NodeStatus/Workflow/NodeRuntimeState/WorkflowStatus/WorkflowError)
//! 已上移到 axagent-harness,本 crate 通过 pub use 复用。

// 运行时执行态 DTO 复用 harness(阶段 2 上移)
pub use axagent_harness::workflow_types::{
    NodeRuntimeState, NodeStatus, Workflow, WorkflowError, WorkflowStatus,
};

// 保留 From<WorkEngineError> for WorkflowError 转换,
// 因 WorkEngineError 是 rt-workflow 内部类型,harness 不依赖它。
impl From<crate::work_engine::engine::WorkEngineError> for WorkflowError {
    /// 跨错误体系转换:`WorkEngineError` 多用于运行态错误(execution 不存在 / 取消 / DB 等),
    /// 在 `run_workflow` 返回 `Result<Workflow, WorkflowError>` 路径上需要 `?` 隐式转换时使用。
    /// 保留原始 Display 文本以便排查,统一映射到 `SerializationError`(最接近"运行态序列化失败"语义)。
    fn from(e: crate::work_engine::engine::WorkEngineError) -> Self {
        Self::SerializationError(e.to_string())
    }
}

// ── 辅助函数 ──

pub(crate) fn current_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub(crate) fn current_timestamp() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
}
