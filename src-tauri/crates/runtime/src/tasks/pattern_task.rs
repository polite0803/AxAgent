// SPDX-License-Identifier: AGPL-3.0-only

//! PatternAnalyzerTask — 跨会话模式分析任务（后台周期执行）
//!
//! ## 当前状态（2026-07-27）
//!
//! `pattern_analyzer` 模块已删除（原 `axagent_trajectory::analyze_trajectories`
//! 函数与 `PatternAnalysisSummary` 等类型已不存在）。本任务暂时降级为
//! 「只统计轨迹数量,不进行模式分析」,等待后续恢复或重写。
//!
//! ## 历史职责（待恢复）
//!
//! 在定时触发时,执行以下操作：
//! - 从 trajectory 存储读取最近的会话轨迹
//! - 调用 PatternAnalyzer 提取跨会话行为模式
//!   （代码风格 / 时间分布 / 工具偏好 / 主题）
//! - 把关键发现作为 LearningInsight 写回 insight_system
//!
//! 与 `start_pattern_learning` 的关系：
//! - `start_pattern_learning` 用 `PatternLearner`（pattern.rs）从 Trajectory
//!   学习 `TrajectoryPattern`,结果是可持久化的模式记录（含 success_rate）
//! - 本任务原计划用 PatternAnalyzer 从 Trajectory 提取更细粒度的用户行为模式

use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// 模式分析任务执行上下文
#[derive(Default, Clone)]
pub struct PatternAnalyzerTaskContext {
    /// 轨迹存储（读取近期轨迹）
    pub trajectory_storage: Option<Arc<axagent_trajectory::TrajectoryStorage>>,
    /// 洞察系统（写入行为模式洞察）— 当前未使用,保留以备恢复
    pub insight_system: Option<Arc<tokio::sync::RwLock<axagent_trajectory::LearningInsightSystem>>>,
}

/// 模式分析任务执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternAnalyzerTaskResult {
    /// 分析的轨迹数量
    pub trajectories_analyzed: usize,
    /// 转换并喂给 PatternAnalyzer 的 BehaviorEvent 总数
    pub total_events_analyzed: usize,
    /// 提取的代码风格模式数量
    pub coding_patterns_count: usize,
    /// 提取的时间分布模式数量
    pub temporal_patterns_count: usize,
    /// 提取的工具偏好模式数量
    pub tool_preference_patterns_count: usize,
    /// 提取的主题模式数量
    pub topic_patterns_count: usize,
    /// 写入 insight_system 的洞察数量
    pub insights_written: usize,
    /// 执行耗时（毫秒）
    pub duration_ms: u64,
    /// 错误信息（如有）
    pub errors: Vec<String>,
}

/// 模式分析任务执行器
pub struct PatternAnalyzerTaskExecutor;

impl PatternAnalyzerTaskExecutor {
    /// 执行模式分析任务并返回结果
    ///
    /// **降级模式**：`pattern_analyzer` 模块已删除,本任务当前只统计
    /// 轨迹数量,不进行实际模式分析。原 `analyze_trajectories` 调用
    /// 已移除,等待后续重写后恢复。
    pub async fn execute(ctx: &PatternAnalyzerTaskContext) -> PatternAnalyzerTaskResult {
        let start = std::time::Instant::now();
        let mut result = PatternAnalyzerTaskResult {
            trajectories_analyzed: 0,
            total_events_analyzed: 0,
            coding_patterns_count: 0,
            temporal_patterns_count: 0,
            tool_preference_patterns_count: 0,
            topic_patterns_count: 0,
            insights_written: 0,
            duration_ms: 0,
            errors: Vec::new(),
        };

        let storage = match &ctx.trajectory_storage {
            Some(s) => s,
            None => {
                result.errors.push("跳过：未提供 trajectory_storage".to_string());
                result.duration_ms = start.elapsed().as_millis() as u64;
                return result;
            },
        };

        let trajectories = match storage.get_trajectories(Some(30)).await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("[PatternAnalyzerTask] 拉取轨迹失败: {}", e);
                result.errors.push(format!("拉取轨迹失败: {e}"));
                result.duration_ms = start.elapsed().as_millis() as u64;
                return result;
            },
        };
        if trajectories.is_empty() {
            tracing::info!("[PatternAnalyzerTask] 无近期轨迹，跳过本轮");
            result.duration_ms = start.elapsed().as_millis() as u64;
            return result;
        }

        // TODO: pattern_analyzer 模块已删除,需重新实现或恢复 analyze_trajectories 函数
        // 临时跳过分析,避免编译错误,不影响其他后台任务执行
        tracing::warn!("[PatternAnalyzerTask] pattern_analyzer 模块已删除,跳过分析 (待恢复)");
        result.errors.push("pattern_analyzer 模块已删除,分析跳过".to_string());
        result.trajectories_analyzed = trajectories.len();
        result.duration_ms = start.elapsed().as_millis() as u64;
        result
    }
}
