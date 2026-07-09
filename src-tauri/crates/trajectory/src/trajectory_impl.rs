// SPDX-License-Identifier: AGPL-3.0-only

//! Extension methods for trajectory DTOs — business logic migrated from harness.
//!
//! These extension traits provide methods that involve business rules,
//! scoring algorithms, and cross-type wiring logic that does not belong
//! in the pure DTO definitions of `axagent_harness::trajectory_types`.

use axagent_harness::trajectory_types::{
    ReplayContext, RewardCategory, Trajectory, TrajectoryBuilder, TrajectoryOutcome, TrajectoryStep,
};

/// Business logic extension for `ReplayContext`.
pub trait ReplayContextExt {
    /// Evaluate the replay quality based on deviations, step progress, and
    /// accumulated rewards.
    fn evaluate(&mut self);
}

impl ReplayContextExt for ReplayContext {
    fn evaluate(&mut self) {
        let mut score = 0.5;

        if self.deviations.is_empty() {
            score += 0.3;
        } else {
            score -= (self.deviations.len() as f64 * 0.05).min(0.25);
        }

        let step_progress =
            self.current_step as f64 / self.original_trajectory.steps.len().max(1) as f64;

        if step_progress > 0.5 && self.original_trajectory.outcome == TrajectoryOutcome::Success {
            score += 0.2;
        }

        score += self.accumulated_reward * 0.1;

        self.evaluation = score.clamp(0.0, 1.0);
    }
}

/// Business logic extension for `TrajectoryBuilder`.
pub trait TrajectoryBuilderExt {
    /// Consume the builder and produce a `Trajectory`.
    fn build(
        self,
        topic: String,
        summary: String,
        outcome: TrajectoryOutcome,
        duration_ms: u64,
    ) -> Trajectory;
}

impl TrajectoryBuilderExt for TrajectoryBuilder {
    fn build(
        self,
        topic: String,
        summary: String,
        outcome: TrajectoryOutcome,
        duration_ms: u64,
    ) -> Trajectory {
        Trajectory::new(
            self.session_id,
            self.user_id,
            topic,
            summary,
            outcome,
            duration_ms,
            self.steps,
        )
    }
}

/// Business logic extension for `RewardCategory`.
pub trait RewardCategoryExt {
    /// Weight used in reward normalization.
    fn weight(&self) -> f64;
    /// Human-readable label.
    fn label(&self) -> &'static str;
}

impl RewardCategoryExt for RewardCategory {
    fn weight(&self) -> f64 {
        match self {
            RewardCategory::Correctness => 0.30,
            RewardCategory::Coherence => 0.20,
            RewardCategory::Completeness => 0.25,
            RewardCategory::Efficiency => 0.15,
            RewardCategory::Safety => 0.10,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            RewardCategory::Correctness => "correctness",
            RewardCategory::Coherence => "coherence",
            RewardCategory::Completeness => "completeness",
            RewardCategory::Efficiency => "efficiency",
            RewardCategory::Safety => "safety",
        }
    }
}
