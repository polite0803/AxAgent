// SPDX-License-Identifier: AGPL-3.0-only

//! Steer 指令上下文注入器。
//!
//! 从 `SteerManager` 中取出挂起的 steer 指令，格式化为 XML 块注入系统提示。
//! 参考 nomifun-tauri 的 ContextContributor 设计。

use std::sync::Arc;

use axagent_runtime_core::context_contributor::{ContextContributor, ContextRequest};

/// 从 `SteerManager` 获取 pending steer 指令的 contributor。
pub struct SteerContributor {
    steer_manager: Arc<crate::steer_manager::SteerManager>,
}

impl SteerContributor {
    pub fn new(steer_manager: Arc<crate::steer_manager::SteerManager>) -> Self {
        Self { steer_manager }
    }
}

impl ContextContributor for SteerContributor {
    fn contribute(&self, _ctx: &ContextRequest) -> Option<String> {
        // 在当前线程 block_on 取 steer 指令是安全的（drain_pending 是轻量操作）
        let instructions = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.steer_manager.drain_pending())
        });
        if instructions.is_empty() {
            return None;
        }
        let formatted: Vec<String> = instructions
            .iter()
            .map(|m| format!("- [{}] {}", m.id, m.instruction))
            .collect();
        Some(format!(
            "<steer-instructions type=\"temporary\">\n# Steer Instructions\n\nThe following steering instructions were provided by the user in real time. These are temporary guidance for the current task only and should be followed with high priority.\n\n{}\n</steer-instructions>",
            formatted.join("\n")
        ))
    }
}
