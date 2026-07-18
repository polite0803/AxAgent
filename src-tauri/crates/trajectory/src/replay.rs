// SPDX-License-Identifier: AGPL-3.0-only

//! TrajectoryReplayer — ReplayExecutor 的 trajectory crate 实现。
//!
//! 基于 `axagent_harness::replay_executor` 契约，提供：
//! - `replay_by_id`：从存储加载 golden trajectory 后回放
//! - `replay`：直接对比两条轨迹（trait 方法）
//! - `replay_suite`：批量回放回归样本集（trait 默认实现）
//!
//! ## 架构角色
//!
//! `trajectory` crate 属于 implementor 层，可依赖 `harness`（契约）和 `entities`（数据定义）。
//! `TrajectoryReplayer` 通过 `TrajectoryStorage` 加载轨迹，不依赖 consumer crate。
//!
//! ## 后续扩展点
//!
//! - 注入 `HarnessToolExecutor` 实现真实工具重放（当前仅结构对比）
//! - 新增 replay_runs 表持久化回放结果（当前仅在内存生成报告）

use std::sync::Arc;

use async_trait::async_trait;

use axagent_harness::replay_executor::{
    ReplayExecutor, ReplayOptions, ReplayReport, build_replay_report, compare_trajectories,
};
use axagent_harness::trajectory_types::Trajectory;

use crate::storage::TrajectoryStorage;

/// 轨迹回放执行器 — 基于 `TrajectoryStorage` 加载 golden trajectory 并对比。
///
/// ## 用法
///
/// ```ignore
/// use axagent_trajectory::TrajectoryReplayer;
/// use axagent_harness::replay_executor::{ReplayExecutor, ReplayOptions};
///
/// let replayer = TrajectoryReplayer::new(storage);
/// let report = replayer.replay_by_id("golden-001", &current_trajectory, &ReplayOptions::default()).await?;
/// if report.passed {
///     println!("regression test passed");
/// }
/// ```
pub struct TrajectoryReplayer {
    /// 轨迹存储，用于按 ID 加载 golden trajectory
    storage: Arc<TrajectoryStorage>,
}

impl TrajectoryReplayer {
    /// 创建回放执行器，注入轨迹存储。
    pub fn new(storage: Arc<TrajectoryStorage>) -> Self {
        Self { storage }
    }

    /// 按 ID 加载 golden trajectory 并与 current 对比。
    ///
    /// 便利方法：内部调用 `ReplayExecutor::replay`。
    /// 若 golden trajectory 不存在，返回错误。
    pub async fn replay_by_id(
        &self,
        golden_id: &str,
        current: &Trajectory,
        options: &ReplayOptions,
    ) -> Result<ReplayReport, String> {
        let golden = self
            .storage
            .get_trajectory(golden_id)
            .await
            .map_err(|e| format!("failed to load golden trajectory {golden_id}: {e}"))?
            .ok_or_else(|| format!("golden trajectory {golden_id} not found"))?;

        self.replay(&golden, current, options).await
    }
}

#[async_trait]
impl ReplayExecutor for TrajectoryReplayer {
    async fn replay(
        &self,
        golden: &Trajectory,
        current: &Trajectory,
        options: &ReplayOptions,
    ) -> Result<ReplayReport, String> {
        let started = chrono::Utc::now().timestamp_millis();

        // 委托给 harness 层纯函数执行对比逻辑
        let deviations = compare_trajectories(golden, current, options);

        let finished = chrono::Utc::now().timestamp_millis();
        let report = build_replay_report(golden, deviations, options, started, finished);

        tracing::info!(
            trajectory_id = %report.trajectory_id,
            passed = report.passed,
            evaluation = report.evaluation,
            deviation_count = report.deviations.len(),
            "replay completed"
        );

        Ok(report)
    }

    // replay_suite 使用 trait 默认实现（逐个回放）
}

// ── 单元测试 ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::replay_executor::{DeviationKind, GoldenTrajectory, RegressionSuite};
    use axagent_harness::trajectory_types::{
        MessageRole, TrajectoryOutcome, TrajectoryQuality, TrajectoryStep,
    };

    fn make_step(role: MessageRole, content: &str) -> TrajectoryStep {
        TrajectoryStep {
            timestamp_ms: 0,
            role,
            content: content.to_string(),
            reasoning: None,
            tool_calls: None,
            tool_results: None,
        }
    }

    fn make_trajectory(id: &str, steps: Vec<TrajectoryStep>) -> Trajectory {
        Trajectory {
            id: id.to_string(),
            session_id: "test_session".to_string(),
            user_id: "test_user".to_string(),
            topic: "test".to_string(),
            summary: "test trajectory".to_string(),
            outcome: TrajectoryOutcome::Success,
            duration_ms: 1000,
            quality: TrajectoryQuality::default(),
            value_score: 0.8,
            patterns: vec![],
            steps,
            rewards: vec![],
            created_at: chrono::Utc::now(),
            replay_count: 0,
            last_replay_at: None,
        }
    }

    /// 无存储的轻量测试：直接调用 trait 方法（不依赖 replay_by_id）
    async fn replay_direct(
        golden: &Trajectory,
        current: &Trajectory,
        options: &ReplayOptions,
    ) -> ReplayReport {
        // 不构造 TrajectoryReplayer（需要 storage），直接调用纯函数
        let started = chrono::Utc::now().timestamp_millis();
        let deviations = compare_trajectories(golden, current, options);
        let finished = chrono::Utc::now().timestamp_millis();
        build_replay_report(golden, deviations, options, started, finished)
    }

    #[tokio::test]
    async fn test_replay_identical_trajectories() {
        let steps = vec![
            make_step(MessageRole::User, "hello"),
            make_step(MessageRole::Assistant, "hi there"),
        ];
        let golden = make_trajectory("g1", steps.clone());
        let current = make_trajectory("c1", steps);
        let options = ReplayOptions::default();

        let report = replay_direct(&golden, &current, &options).await;
        assert!(report.passed);
        assert_eq!(report.total_steps, 2);
        assert_eq!(report.matched_steps, 2);
        assert!(report.deviations.is_empty());
    }

    #[tokio::test]
    async fn test_replay_with_deviation() {
        let golden = make_trajectory(
            "g1",
            vec![
                make_step(MessageRole::User, "hello"),
                make_step(MessageRole::Assistant, "hi there"),
            ],
        );
        let current = make_trajectory(
            "c1",
            vec![
                make_step(MessageRole::User, "hello"),
                make_step(MessageRole::Assistant, "goodbye"), // content 不同
            ],
        );
        let options = ReplayOptions::default();

        let report = replay_direct(&golden, &current, &options).await;
        assert!(!report.passed, "should fail with content mismatch");
        assert_eq!(report.deviations.len(), 1);
        assert_eq!(report.deviations[0].kind, DeviationKind::ContentMismatch);
        assert_eq!(report.matched_steps, 1); // 第 0 步匹配
    }

    #[tokio::test]
    async fn test_replay_with_max_deviations_tolerance() {
        let golden = make_trajectory(
            "g1",
            vec![make_step(MessageRole::User, "a"), make_step(MessageRole::Assistant, "b")],
        );
        let current = make_trajectory(
            "c1",
            vec![
                make_step(MessageRole::User, "x"),      // deviation 1
                make_step(MessageRole::Assistant, "y"), // deviation 2
            ],
        );
        let options = ReplayOptions {
            max_deviations: 2, // 容忍 2 个偏差
            ..Default::default()
        };

        let report = replay_direct(&golden, &current, &options).await;
        assert!(report.passed, "should pass with 2 deviations <= max=2");
        assert_eq!(report.deviations.len(), 2);
        assert_eq!(report.matched_steps, 0);
    }

    #[tokio::test]
    async fn test_replay_suite_via_trait_default() {
        let golden1 = GoldenTrajectory::new(
            make_trajectory("g1", vec![make_step(MessageRole::User, "hello")]),
            TrajectoryOutcome::Success,
        );
        let golden2 = GoldenTrajectory::new(
            make_trajectory("g2", vec![make_step(MessageRole::User, "world")]),
            TrajectoryOutcome::Success,
        );

        let current_matched = make_trajectory("c1", vec![make_step(MessageRole::User, "hello")]);
        let current_mismatched =
            make_trajectory("c2", vec![make_step(MessageRole::User, "different")]);

        let suite = RegressionSuite::new("test_suite".to_string(), ReplayOptions::default())
            .with_golden(golden1)
            .with_golden(golden2);

        // 直接使用纯函数模拟 suite 回放（不依赖 TrajectoryReplayer 的 storage）
        let options = &suite.options;
        let mut reports = Vec::new();
        let current_trajectories = [&current_matched, &current_mismatched];
        for (g, c) in suite.golden_trajectories.iter().zip(current_trajectories.iter()) {
            let deviations = compare_trajectories(&g.trajectory, c, options);
            let report = build_replay_report(&g.trajectory, deviations, options, 0, 1);
            reports.push(report);
        }

        let passed_count = reports.iter().filter(|r| r.passed).count();
        let failed_count = reports.len() - passed_count;
        assert_eq!(passed_count, 1, "one golden should match, one should not");
        assert_eq!(failed_count, 1);
    }
}
