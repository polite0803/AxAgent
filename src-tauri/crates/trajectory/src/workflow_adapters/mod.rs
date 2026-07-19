// SPDX-License-Identifier: AGPL-3.0-only

//! 工作流反思/进化/优化三层 trait 的 trajectory 实现(阶段 4)
//!
//! 实现方位于 trajectory crate(implementor 层),遵循 harness 三层 trait 契约:
//! - [`WorkflowReflectorImpl`] 实现 `axagent_harness::WorkflowReflector`
//! - [`WorkflowEvolverImpl`] 实现 `axagent_harness::WorkflowEvolver`
//! - [`WorkflowOptimizerImpl`] 实现 `axagent_harness::WorkflowOptimizer`
//!
//! 设计原则(MVP):
//! - 不直接依赖 LLM,纯启发式规则,启动即可用
//! - 内存存储反思历史(`tokio::sync::RwLock<Vec<Reflection>>`),持久化交给 wiring 层
//! - 真正的 LLM 变异/沙箱验证由 wiring 层通过 `set_llm_provider` / `set_sandbox` 注入
//! - 复用 `axagent_harness::Reflection` DTO,工作流专有结构化数据写入 `metadata`

mod evolver;
mod optimizer;
mod reflector;

pub use evolver::WorkflowEvolverImpl;
pub use optimizer::WorkflowOptimizerImpl;
pub use reflector::{ReflectorConfig, WorkflowReflectorImpl};
