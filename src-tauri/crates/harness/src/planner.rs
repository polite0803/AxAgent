//! 分层规划器适配器契约。
//!
//! 提供计划生成、执行管理、重规划能力，用于工作流中的 Plan 模式。
//!
//! 实现方（`axagent-agent::hierarchical_planner`）负责计划验证、
//! 依赖管理、状态追踪和重规划。

use std::fmt;

use serde_json::Value as JsonValue;

/// 分层规划器适配器契约
///
/// 封装计划生命周期：创建 → 执行 → 监控 → 重规划。
/// 所有 DTO 通过 JSON 传递，避免 harness 对具体类型的依赖。
pub trait PlannerAdapter: fmt::Debug + Send + Sync {
    /// 创建计划
    ///
    /// `goal`：计划目标描述
    /// `phases_json`：阶段定义数组 JSON
    fn create_plan(&mut self, goal: &str, phases_json: &[JsonValue]) -> Result<(), String>;

    /// 开始执行计划（验证完整性后切换状态为 Executing）
    fn start_execution(&mut self) -> Result<(), String>;

    /// 获取当前计划快照（JSON 格式）
    fn current_plan(&self) -> Option<JsonValue>;

    /// 请求重规划
    ///
    /// `reason`：重规划原因
    /// `actions_json`：重规划动作数组 JSON
    fn request_replan(&mut self, reason: &str, actions_json: &[JsonValue]) -> Result<(), String>;

    /// 检查计划是否已全部完成
    fn is_completed(&self) -> bool;

    /// 标记指定任务为已完成
    fn mark_task_completed(&mut self, phase_index: usize, task_index: usize, result: JsonValue);

    /// 标记指定阶段为已完成（检查所有任务状态）
    fn mark_phase_completed(&mut self, phase_index: usize) -> Result<(), String>;

    /// 获取所有失败步骤的 task_id 列表
    fn get_failed_steps(&self) -> Vec<String>;

    /// 获取所有待处理步骤的 task_id 列表
    fn get_pending_steps(&self) -> Vec<String>;
}

/// 空实现 — 总是失败（规划器未配置）
#[derive(Debug)]
pub struct NoopPlannerAdapter;

impl PlannerAdapter for NoopPlannerAdapter {
    fn create_plan(&mut self, _goal: &str, _phases_json: &[JsonValue]) -> Result<(), String> {
        Err("Planner is not configured".to_string())
    }

    fn start_execution(&mut self) -> Result<(), String> {
        Err("Planner is not configured".to_string())
    }

    fn current_plan(&self) -> Option<JsonValue> {
        None
    }

    fn request_replan(&mut self, _reason: &str, _actions_json: &[JsonValue]) -> Result<(), String> {
        Err("Planner is not configured".to_string())
    }

    fn is_completed(&self) -> bool {
        false
    }

    fn mark_task_completed(&mut self, _phase_index: usize, _task_index: usize, _result: JsonValue) {
    }

    fn mark_phase_completed(&mut self, _phase_index: usize) -> Result<(), String> {
        Err("Planner is not configured".to_string())
    }

    fn get_failed_steps(&self) -> Vec<String> {
        Vec::new()
    }

    fn get_pending_steps(&self) -> Vec<String> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_never_creates_plan() {
        let mut adapter = NoopPlannerAdapter;
        assert!(adapter.create_plan("test", &[]).is_err());
        assert!(adapter.current_plan().is_none());
        assert!(!adapter.is_completed());
    }
}
