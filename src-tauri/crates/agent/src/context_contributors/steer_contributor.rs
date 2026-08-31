// SPDX-License-Identifier: AGPL-3.0-only

//! Steer 指令上下文注入器。
//!
//! 从 `SteerManager` 中取出挂起的 steer 指令，格式化为 XML 块注入系统提示。
//! 参考 nomifun-tauri 的 ContextContributor 设计。

use std::sync::Arc;

use axagent_harness::context_contributor::{ContextContributor, ContextRequest};

/// 从 `SteerManager` 获取 pending steer 指令的 contributor。
pub struct SteerContributor {
    steer_manager: Arc<crate::steer_manager::SteerManager>,
}

impl SteerContributor {
    pub fn new(steer_manager: Arc<crate::steer_manager::SteerManager>) -> Self {
        Self { steer_manager }
    }
}

#[async_trait::async_trait]
impl ContextContributor for SteerContributor {
    async fn contribute(&self, _ctx: &ContextRequest<'_>) -> Option<String> {
        // 直接 await：`drain_pending` 是异步的，trait 改异步后不再需要
        // `block_in_place` 绕路（那会在多线程运行时下阻塞整个 worker）。
        let instructions = self.steer_manager.drain_pending().await;
        if instructions.is_empty() {
            return None;
        }
        let formatted: Vec<String> =
            instructions.iter().map(|m| format!("- [{}] {}", m.id, m.instruction)).collect();
        Some(format!(
            "<steer-instructions type=\"temporary\">\n# Steer Instructions\n\nThe following steering instructions were provided by the user in real time. These are temporary guidance for the current task only and should be followed with high priority.\n\n{}\n</steer-instructions>",
            formatted.join("\n")
        ))
    }
}
