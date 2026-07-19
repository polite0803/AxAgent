// SPDX-License-Identifier: AGPL-3.0-only

// `ErrorContext` 已上移到 axagent-harness 并重命名为 `WorkflowErrorContext`,
// 以避免与 `axagent_harness::core_error::ErrorContext`(telemetry 语义)冲突。
// 本 crate 通过 pub use 复用,并提供 `ErrorContext` 别名以兼容现有引用。
pub use axagent_harness::workflow_types::WorkflowErrorContext;

/// 兼容别名:rt-workflow 内部代码继续用 `ErrorContext` 名称。
///
/// 新代码应直接使用 `WorkflowErrorContext` 以避免歧义。
pub type ErrorContext = WorkflowErrorContext;
