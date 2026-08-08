// SPDX-License-Identifier: AGPL-3.0-only

//! Trajectory scoring and analysis services — extracted from DTO methods.
//!
//! All business logic previously embedded in `Trajectory`, `TrajectoryPattern`,
//! and `GeneratedTool` lives here so those types remain pure data carriers.

use chrono::Utc;
use serde_json;

use crate::trajectory_types::{
    MessageRole, RLTrainingEntry, RewardSignal, Trajectory, TrajectoryOutcome, TrajectoryPattern,
    TrajectoryQuality,
};

/// Scoring service for Trajectory quality, value, rewards, and exports.
pub struct TrajectoryScorer;

impl TrajectoryScorer {
    /// Compute quality metrics for a trajectory based on its steps and outcome.
    pub fn compute_quality(
        steps: &[crate::trajectory_types::TrajectoryStep],
        outcome: TrajectoryOutcome,
    ) -> TrajectoryQuality {
        let task_completion = match outcome {
            TrajectoryOutcome::Success => 1.0,
            TrajectoryOutcome::Partial => 0.5,
            TrajectoryOutcome::Failure => 0.0,
            TrajectoryOutcome::Abandoned => 0.2,
        };

        let tool_count = steps.iter().filter(|s| s.tool_calls.is_some()).count();
        let successful_tools = steps
            .iter()
            .filter(|s| {
                s.tool_results.as_ref().map(|r| !r.iter().any(|tr| tr.is_error)).unwrap_or(false)
            })
            .count();
        let tool_efficiency = if tool_count > 0 {
            successful_tools as f64 / tool_count as f64
        } else {
            0.5
        };

        let reasoning_count = steps.iter().filter(|s| s.reasoning.is_some()).count();
        let reasoning_quality = if !steps.is_empty() {
            reasoning_count as f64 / steps.len() as f64 * 0.5 + 0.25
        } else {
            0.25
        };

        let user_satisfaction = match outcome {
            TrajectoryOutcome::Success => 0.9,
            TrajectoryOutcome::Partial => 0.5,
            TrajectoryOutcome::Failure => 0.1,
            TrajectoryOutcome::Abandoned => 0.3,
        };

        let overall = task_completion * 0.4
            + tool_efficiency * 0.2
            + reasoning_quality * 0.15
            + user_satisfaction * 0.25;

        TrajectoryQuality {
            overall: overall.clamp(0.0, 1.0),
            task_completion,
            tool_efficiency,
            reasoning_quality,
            user_satisfaction,
        }
    }

    /// Compute a value score for reinforcement learning feedback.
    pub fn compute_value_score(
        quality: f64,
        outcome: TrajectoryOutcome,
        steps: &[crate::trajectory_types::TrajectoryStep],
    ) -> f64 {
        let mut score = quality * 0.5;

        match outcome {
            TrajectoryOutcome::Success => score += 0.35,
            TrajectoryOutcome::Partial => score += 0.15,
            TrajectoryOutcome::Failure => score -= 0.2,
            TrajectoryOutcome::Abandoned => score -= 0.3,
        }

        let has_reasoning = steps.iter().any(|s| s.reasoning.is_some());
        if has_reasoning {
            score += 0.1;
        }

        let step_count = steps.len();
        if (3..=30).contains(&step_count) {
            score += 0.05;
        }

        score.clamp(0.0, 1.0)
    }

    /// Add a reward signal to a trajectory.
    pub fn add_reward(trajectory: &mut Trajectory, reward: RewardSignal) {
        trajectory.rewards.push(reward);
    }

    /// Increment replay count and update replay timestamp.
    pub fn increment_replay(trajectory: &mut Trajectory) {
        trajectory.replay_count += 1;
        trajectory.last_replay_at = Some(Utc::now());
    }

    /// Export a trajectory as an RL training entry.
    pub fn export_as_rl(trajectory: &Trajectory) -> RLTrainingEntry {
        let prompt = trajectory
            .steps
            .iter()
            .filter(|s| s.role == MessageRole::User)
            .map(|s| s.content.clone())
            .collect::<Vec<_>>()
            .join("\n\n");

        let mut completion = String::new();
        for step in trajectory.steps.iter().filter(|s| s.role == MessageRole::Assistant) {
            completion.push_str(&step.content);
            if let Some(ref tool_calls) = step.tool_calls {
                completion.push_str("\n\n<tool_calls>\n");
                completion.push_str(&serde_json::to_string(tool_calls).unwrap_or_default());
                completion.push_str("\n</tool_calls>\n");
            }
            completion.push_str("\n\n");
        }

        RLTrainingEntry {
            prompt: prompt.chars().take(4000).collect(),
            completion: completion.chars().take(4000).collect(),
            trajectory_id: trajectory.id.clone(),
            topic: trajectory.topic.clone(),
            quality: trajectory.quality.overall,
            value_score: trajectory.value_score,
            rewards: trajectory.rewards.clone(),
        }
    }

    /// Apply scoring to a freshly constructed trajectory.
    ///
    /// Call after `Trajectory::new()` to compute quality and value_score
    /// using the external scorer (since compute_quality / compute_value_score
    /// are no longer called inside the constructor).
    pub fn apply(trajectory: &mut Trajectory) {
        let quality = Self::compute_quality(&trajectory.steps, trajectory.outcome);
        let value_score =
            Self::compute_value_score(quality.overall, trajectory.outcome, &trajectory.steps);
        trajectory.quality = quality;
        trajectory.value_score = value_score;
    }
}

/// Updates a `TrajectoryPattern` from a completed trajectory.
pub struct TrajectoryPatternUpdater;

impl TrajectoryPatternUpdater {
    pub fn update_from_trajectory(pattern: &mut TrajectoryPattern, trajectory: &Trajectory) {
        if !pattern.trajectory_ids.contains(&trajectory.id) {
            pattern.trajectory_ids.push(trajectory.id.clone());
        }

        pattern.frequency = pattern.trajectory_ids.len() as u32;

        let prev_total = (pattern.frequency - 1) as f64;
        let success = match trajectory.outcome {
            TrajectoryOutcome::Success => 1.0,
            TrajectoryOutcome::Partial => 0.5,
            _ => 0.0,
        };

        pattern.success_rate = if prev_total > 0.0 {
            (pattern.success_rate * prev_total + success) / pattern.frequency as f64
        } else {
            success
        };

        let quality_delta = trajectory.quality.overall - pattern.average_quality;
        pattern.average_quality += quality_delta / pattern.frequency as f64;

        let value_delta = trajectory.value_score - pattern.average_value_score;
        pattern.average_value_score += value_delta / pattern.frequency as f64;
    }
}

/// Records usage statistics for generated tools.
pub struct GeneratedToolRecorder;

impl GeneratedToolRecorder {
    /// 记录工具调用成功：增加使用计数，更新成功率
    pub fn record_success(tool: &mut crate::trajectory_types::GeneratedTool) {
        tool.usage_count += 1;
        let prev_total = tool.usage_count.saturating_sub(1) as f64;
        tool.success_rate = if prev_total > 0.0 {
            (tool.success_rate * prev_total + 1.0) / tool.usage_count as f64
        } else {
            1.0
        };
    }

    /// 记录工具调用失败：增加使用计数，更新成功率
    pub fn record_failure(tool: &mut crate::trajectory_types::GeneratedTool) {
        tool.usage_count += 1;
        let prev_total = tool.usage_count.saturating_sub(1) as f64;
        tool.success_rate = if prev_total > 0.0 {
            (tool.success_rate * prev_total + 0.0) / tool.usage_count as f64
        } else {
            0.0
        };
    }
}

/// Scorecard 门禁级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateLevel {
    /// 硬门禁：未通过则阻断上线（质量分 < 0.6 或安全分 < 0.8）
    Hard,
    /// 软门禁：未通过给出警告但不阻断（质量分 < 0.7）
    Soft,
    /// 盲审门禁：匿名评审，不显示分数来源
    Blind,
}

/// Scorecard 门禁结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GateResult {
    pub level: GateLevel,
    pub passed: bool,
    pub quality_score: f64,
    pub safety_score: f64,
    pub efficiency_score: f64,
    pub detail: String,
}

impl TrajectoryScorer {
    /// Scorecard 三档门禁评估
    pub fn evaluate_gate(
        quality: TrajectoryQuality,
        safety_score: f64,
        level: GateLevel,
    ) -> GateResult {
        let q = quality.overall;
        let s = safety_score.clamp(0.0, 1.0);
        let e = quality.tool_efficiency;

        let (passed, detail) = match level {
            GateLevel::Hard => {
                // 硬门禁：质量分 >= 0.6 且 安全分 >= 0.8
                let p = q >= 0.6 && s >= 0.8;
                let d = if p {
                    format!("硬门禁通过：质量分 {:.2} ≥ 0.6，安全分 {:.2} ≥ 0.8", q, s)
                } else {
                    format!("硬门禁未通过：质量分 {:.2} (需 ≥ 0.6)，安全分 {:.2} (需 ≥ 0.8)", q, s)
                };
                (p, d)
            },
            GateLevel::Soft => {
                // 软门禁：质量分 >= 0.7
                let p = q >= 0.7;
                let d = if p {
                    format!("软门禁通过：质量分 {:.2} ≥ 0.7", q)
                } else {
                    format!("软门禁未通过：质量分 {:.2} (需 ≥ 0.7)，建议优化", q)
                };
                (p, d)
            },
            GateLevel::Blind => {
                // 盲审门禁：通过即允许，不阻断，仅记录
                let p = q >= 0.5;
                let d = format!("盲审评估：质量分 {:.2}，已记录供人工审核", q);
                (p, d)
            },
        };

        GateResult { level, passed, quality_score: q, safety_score: s, efficiency_score: e, detail }
    }
}
