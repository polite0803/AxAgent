// SPDX-License-Identifier: AGPL-3.0-only

//! 工作流模板仓库契约（让 trajectory / runtime 等 consumer 不依赖 dao）。
//!
//! 仅提供最小读写接口:
//! - `list_templates` / `get_template`: Optimizer / Evolver 需要遍历全部模板
//! - `save_template`: tick 自动应用优化后回写
//!
//! 权威定义位于 harness foundation 层,dao crate 实现。

use crate::workflow_types::WorkflowTemplateData;
use async_trait::async_trait;

/// 工作流模板仓库 trait（极简读写接口）。
///
/// 实现方:dao crate(SeaORM SQLite 持久化)。
/// 调用方:trajectory(`start_workflow_evolution_tick`)、wiring 层。
#[async_trait]
pub trait WorkflowTemplateRepo: Send + Sync {
    /// 列出所有可进化的模板（排除系统模板 / 预设模板由实现方决定）。
    async fn list_templates(&self) -> Result<Vec<WorkflowTemplateData>, String>;

    /// 按 ID 取模板。
    async fn get_template(&self, id: &str) -> Result<Option<WorkflowTemplateData>, String>;

    /// 保存（更新）模板。由实现方处理版本号 / 更新时间戳。
    async fn save_template(&self, template: &WorkflowTemplateData) -> Result<(), String>;
}
