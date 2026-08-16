// SPDX-License-Identifier: AGPL-3.0-only

//! 技能侧反思钩子 trait 契约（自我进化通道二：能力偏弱进化改进）。
//!
//! `ConversationRuntime`（runtime-core，consumer）通过本 trait 注入技能进化能力，
//! 不直接依赖 trajectory（implementor）实现层 —— 满足 Harness 架构依赖方向铁律。
//!
//! 实现方（wiring 层）在 `on_tool_executed` 内负责：
//!   1. `tool_name` → Skill 映射；
//!   2. 记录执行结果（更新 `consecutive_failures` / `total_usages` / `success_rate`）；
//!   3. `should_auto_evolve` 判定；
//!   4. 命中后生成进化提议并进入用户同意通道（emit `evolution-consent-request`）。

use async_trait::async_trait;

/// 技能侧反思钩子：工具执行完成后由 `ConversationRuntime` 调用。
#[async_trait]
pub trait SkillEvolutionHook: Send + Sync {
    /// 工具执行完成后的反思入口。
    ///
    /// - `tool_name`：本次被调用的工具名；
    /// - `success`：本次执行是否成功；
    /// - `output`：执行输出（供质量评估）。
    ///
    /// 返回 `true` 表示已生成进化提议并进入用户同意通道；调用方仅记录日志，
    /// 不因反思结果阻塞工具执行主流程。
    async fn on_tool_executed(&self, tool_name: &str, success: bool, output: &str) -> bool;
}
