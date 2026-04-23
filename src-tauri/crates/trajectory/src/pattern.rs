//! Pattern learning and extraction module

use crate::trajectory::{Trajectory, TrajectoryOutcome, TrajectoryPattern};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternConfig {
    pub min_frequency: u32,
    pub min_success_rate: f64,
    pub max_patterns: usize,
    pub pattern_types: Vec<PatternType>,
}

impl Default for PatternConfig {
    fn default() -> Self {
        Self {
            min_frequency: 3,
            min_success_rate: 0.5,
            max_patterns: 100,
            pattern_types: vec![
                PatternType::ToolSequence,
                PatternType::ReasoningChain,
                PatternType::ErrorRecovery,
                PatternType::UserInteraction,
                PatternType::ContextSwitch,
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternType {
    ToolSequence,
    ReasoningChain,
    ErrorRecovery,
    UserInteraction,
    ContextSwitch,
    MultiStep,
    GoalOriented,
    Exploratory,
}

impl PatternType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PatternType::ToolSequence => "tool_sequence",
            PatternType::ReasoningChain => "reasoning_chain",
            PatternType::ErrorRecovery => "error_recovery",
            PatternType::UserInteraction => "user_interaction",
            PatternType::ContextSwitch => "context_switch",
            PatternType::MultiStep => "multi_step",
            PatternType::GoalOriented => "goal_oriented",
            PatternType::Exploratory => "exploratory",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "tool_sequence" => PatternType::ToolSequence,
            "reasoning_chain" => PatternType::ReasoningChain,
            "error_recovery" => PatternType::ErrorRecovery,
            "user_interaction" => PatternType::UserInteraction,
            "context_switch" => PatternType::ContextSwitch,
            "multi_step" => PatternType::MultiStep,
            "goal_oriented" => PatternType::GoalOriented,
            "exploratory" => PatternType::Exploratory,
            _ => PatternType::ToolSequence,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPattern {
    pub pattern_type: PatternType,
    pub name: String,
    pub description: String,
    pub trajectory_count: usize,
    pub success_rate: f64,
    pub avg_quality: f64,
    pub steps: Vec<PatternStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternStep {
    pub step_index: usize,
    pub role: String,
    pub content_preview: String,
    pub has_tool_call: bool,
    pub tool_name: Option<String>,
    pub has_reasoning: bool,
}

impl DetectedPattern {
    pub fn from_trajectories(
        trajectories: &[&Trajectory],
        pattern_type: PatternType,
    ) -> Option<Self> {
        if trajectories.len() < 2 {
            return None;
        }

        let avg_quality: f64 =
            trajectories.iter().map(|t| t.quality.overall).sum::<f64>() / trajectories.len() as f64;

        let success_count = trajectories
            .iter()
            .filter(|t| t.outcome == TrajectoryOutcome::Success)
            .count();
        let success_rate = success_count as f64 / trajectories.len() as f64;

        let steps = Self::extract_common_steps(trajectories)?;

        let name = Self::generate_pattern_name(&steps, pattern_type);
        let description = Self::generate_description(&steps, pattern_type, success_rate);

        Some(DetectedPattern {
            pattern_type,
            name,
            description,
            trajectory_count: trajectories.len(),
            success_rate,
            avg_quality,
            steps,
        })
    }

    fn extract_common_steps(trajectories: &[&Trajectory]) -> Option<Vec<PatternStep>> {
        if trajectories.is_empty() {
            return None;
        }

        let first_steps = &trajectories[0].steps;
        if first_steps.is_empty() {
            return None;
        }

        let mut common_steps = Vec::new();

        for (i, first_step) in first_steps.iter().enumerate() {
            let mut matching_count = 0;
            let mut has_tool = first_step.tool_calls.is_some();
            let mut tool_name = first_step
                .tool_calls
                .as_ref()
                .and_then(|c| c.first().map(|tc| tc.name.clone()));
            let mut has_reasoning = first_step.reasoning.is_some();

            for trajectory in trajectories.iter().skip(1) {
                if let Some(step) = trajectory.steps.get(i) {
                    if step.role == first_step.role {
                        matching_count += 1;
                        has_tool |= step.tool_calls.is_some();
                        if let Some(ref tc) = step.tool_calls.as_ref().and_then(|c| c.first()) {
                            if tool_name.as_ref() != Some(&tc.name) {
                                tool_name = None;
                            }
                        }
                        has_reasoning |= step.reasoning.is_some();
                    }
                }
            }

            if matching_count >= trajectories.len() / 2 {
                common_steps.push(PatternStep {
                    step_index: i,
                    role: format!("{:?}", first_step.role).to_lowercase(),
                    content_preview: first_step.content.chars().take(50).collect(),
                    has_tool_call: has_tool,
                    tool_name,
                    has_reasoning,
                });
            }
        }

        if common_steps.is_empty() {
            None
        } else {
            Some(common_steps)
        }
    }

    fn generate_pattern_name(steps: &[PatternStep], pattern_type: PatternType) -> String {
        let tool_steps: Vec<_> = steps.iter().filter(|s| s.has_tool_call).collect();

        if let Some(first_tool) = tool_steps.first() {
            if let Some(ref name) = first_tool.tool_name {
                return format!("{}-pattern", name);
            }
        }

        format!("{}-{}", pattern_type.as_str(), steps.len())
    }

    fn generate_description(
        steps: &[PatternStep],
        pattern_type: PatternType,
        success_rate: f64,
    ) -> String {
        let tool_count = steps.iter().filter(|s| s.has_tool_call).count();
        let reasoning_count = steps.iter().filter(|s| s.has_reasoning).count();

        format!(
            "A {} pattern with {} steps, {} tool calls, {} reasoning steps. Historical success rate: {:.0}%",
            pattern_type.as_str(),
            steps.len(),
            tool_count,
            reasoning_count,
            success_rate * 100.0
        )
    }

    pub fn to_trajectory_pattern(&self) -> TrajectoryPattern {
        let mut pattern = TrajectoryPattern::new(
            self.name.clone(),
            self.description.clone(),
            self.pattern_type.as_str().to_string(),
        );

        pattern.frequency = self.trajectory_count as u32;
        pattern.success_rate = self.success_rate;
        pattern.average_quality = self.avg_quality;

        pattern
    }
}

pub struct PatternLearner {
    config: PatternConfig,
    learned_patterns: HashMap<String, TrajectoryPattern>,
}

impl Default for PatternLearner {
    fn default() -> Self {
        Self::new(PatternConfig::default())
    }
}

impl PatternLearner {
    pub fn new(config: PatternConfig) -> Self {
        Self {
            config,
            learned_patterns: HashMap::new(),
        }
    }

    pub fn learn_from_trajectory(&mut self, trajectory: &Trajectory) -> Vec<TrajectoryPattern> {
        let mut new_patterns = Vec::new();

        let tool_sequence = self.extract_tool_sequence(trajectory);
        if let Some(pattern) = tool_sequence {
            new_patterns.push(pattern);
        }

        let reasoning_chain = self.extract_reasoning_chain(trajectory);
        if let Some(pattern) = reasoning_chain {
            new_patterns.push(pattern);
        }

        let error_recovery = self.extract_error_recovery(trajectory);
        if let Some(pattern) = error_recovery {
            new_patterns.push(pattern);
        }

        let multi_step = self.extract_multi_step(trajectory);
        if let Some(pattern) = multi_step {
            new_patterns.push(pattern);
        }

        for pattern in &new_patterns {
            self.learned_patterns
                .insert(pattern.name.clone(), pattern.clone());
        }

        new_patterns
    }

    fn extract_tool_sequence(&self, trajectory: &Trajectory) -> Option<TrajectoryPattern> {
        let tool_names: Vec<_> = trajectory
            .steps
            .iter()
            .filter_map(|s| {
                s.tool_calls
                    .as_ref()
                    .and_then(|c| c.first().map(|tc| tc.name.clone()))
            })
            .collect();

        if tool_names.len() < 2 {
            return None;
        }

        let sequence_key = tool_names.join("->");

        let pattern_key = format!("tool_seq_{}", sequence_key);
        let pattern = self.learned_patterns.get(&pattern_key);

        if let Some(existing) = pattern {
            let mut updated = existing.clone();
            if !updated.trajectory_ids.contains(&trajectory.id) {
                updated.trajectory_ids.push(trajectory.id.clone());
            }
            updated.frequency = updated.trajectory_ids.len() as u32;
            let success = match trajectory.outcome {
                TrajectoryOutcome::Success => 1.0,
                TrajectoryOutcome::Partial => 0.5,
                _ => 0.0,
            };
            updated.success_rate = (updated.success_rate * (updated.frequency - 1) as f64
                + success)
                / updated.frequency as f64;
            return Some(updated);
        }

        let description = format!(
            "Tool sequence: {} ({} steps)",
            sequence_key,
            tool_names.len()
        );

        let mut pattern = TrajectoryPattern::new(
            format!("tool-{}", tool_names[0]),
            description,
            PatternType::ToolSequence.as_str().to_string(),
        );

        pattern.trajectory_ids.push(trajectory.id.clone());

        Some(pattern)
    }

    fn extract_reasoning_chain(&self, trajectory: &Trajectory) -> Option<TrajectoryPattern> {
        let reasoning_steps: Vec<_> = trajectory
            .steps
            .iter()
            .filter(|s| s.reasoning.is_some())
            .collect();

        if reasoning_steps.len() < 2 {
            return None;
        }

        let reasoning_preview: String = reasoning_steps
            .iter()
            .take(3)
            .filter_map(|s| {
                s.reasoning
                    .as_ref()
                    .map(|r| r.chars().take(30).collect::<String>())
            })
            .collect::<Vec<_>>()
            .join(" -> ");

        let description = format!(
            "Reasoning chain with {} reasoning steps: {}",
            reasoning_steps.len(),
            reasoning_preview
        );

        let mut pattern = TrajectoryPattern::new(
            format!(
                "reasoning-chain-{}",
                trajectory.id.chars().take(8).collect::<String>()
            ),
            description,
            PatternType::ReasoningChain.as_str().to_string(),
        );

        pattern.trajectory_ids.push(trajectory.id.clone());

        Some(pattern)
    }

    fn extract_error_recovery(&self, trajectory: &Trajectory) -> Option<TrajectoryPattern> {
        let mut error_indices = Vec::new();
        let mut recovery_indices = Vec::new();

        for (i, step) in trajectory.steps.iter().enumerate() {
            if let Some(ref results) = step.tool_results {
                if results.iter().any(|r| r.is_error) {
                    error_indices.push(i);
                }
            }

            if error_indices.len() > recovery_indices.len() {
                if let Some(ref results) = step.tool_results {
                    if results.iter().any(|r| !r.is_error) && !error_indices.contains(&i) {
                        recovery_indices.push(i);
                    }
                }
            }
        }

        if error_indices.is_empty() || recovery_indices.is_empty() {
            return None;
        }

        let mut recovery_distance: Vec<usize> = Vec::new();
        for &e in &error_indices {
            let mut min_dist = 999;
            for &r in &recovery_indices {
                if r > e && r - e < min_dist {
                    min_dist = r - e;
                }
            }
            if min_dist < 999 {
                recovery_distance.push(min_dist);
            }
        }

        let avg_recovery_steps: f64 =
            recovery_distance.iter().sum::<usize>() as f64 / recovery_distance.len() as f64;

        let description = format!(
            "Error recovery pattern: {} errors, avg recovery in {:.1} steps",
            error_indices.len(),
            avg_recovery_steps
        );

        let mut pattern = TrajectoryPattern::new(
            "error-recovery".to_string(),
            description,
            PatternType::ErrorRecovery.as_str().to_string(),
        );

        pattern.trajectory_ids.push(trajectory.id.clone());
        pattern.reward_profile = vec![
            (crate::trajectory::RewardType::ErrorRecovery, 0.8),
            (crate::trajectory::RewardType::ToolEfficiency, 0.6),
        ];

        Some(pattern)
    }

    fn extract_multi_step(&self, trajectory: &Trajectory) -> Option<TrajectoryPattern> {
        let tool_call_count = trajectory
            .steps
            .iter()
            .filter(|s| s.tool_calls.is_some())
            .count();

        if tool_call_count < 3 {
            return None;
        }

        let description = format!(
            "Multi-step pattern with {} tool calls across {} steps",
            tool_call_count,
            trajectory.steps.len()
        );

        let mut pattern = TrajectoryPattern::new(
            format!("multi-step-{}", tool_call_count),
            description,
            PatternType::MultiStep.as_str().to_string(),
        );

        pattern.trajectory_ids.push(trajectory.id.clone());

        Some(pattern)
    }

    pub fn get_patterns_by_type(&self, pattern_type: PatternType) -> Vec<&TrajectoryPattern> {
        self.learned_patterns
            .values()
            .filter(|p| p.pattern_type == pattern_type.as_str())
            .collect()
    }

    pub fn get_high_value_patterns(&self, min_success_rate: f64) -> Vec<&TrajectoryPattern> {
        self.learned_patterns
            .values()
            .filter(|p| {
                p.success_rate >= min_success_rate && p.frequency >= self.config.min_frequency
            })
            .collect()
    }

    pub fn find_similar_trajectories(
        &self,
        trajectory: &Trajectory,
    ) -> Vec<(&TrajectoryPattern, f64)> {
        let mut similarities = Vec::new();

        let trajectory_tools: Vec<_> = trajectory
            .steps
            .iter()
            .filter_map(|s| {
                s.tool_calls
                    .as_ref()
                    .and_then(|c| c.first().map(|tc| tc.name.clone()))
            })
            .collect();

        for pattern in self.learned_patterns.values() {
            if pattern.pattern_type != PatternType::ToolSequence.as_str() {
                continue;
            }

            let pattern_tools: Vec<_> = pattern
                .description
                .split("Tool sequence: ")
                .nth(1)
                .map(|s| {
                    s.split("->")
                        .map(|t| t.trim().to_string())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            if pattern_tools.is_empty() {
                continue;
            }

            let overlap = trajectory_tools
                .iter()
                .filter(|t| pattern_tools.contains(t))
                .count();

            let similarity = overlap as f64 / pattern_tools.len().max(1) as f64;

            if similarity > 0.5 {
                similarities.push((pattern, similarity));
            }
        }

        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        similarities
    }

    pub fn update_from_batch(&mut self, trajectories: &[Trajectory]) -> Vec<TrajectoryPattern> {
        let mut all_patterns = Vec::new();

        for trajectory in trajectories {
            let patterns = self.learn_from_trajectory(trajectory);
            all_patterns.extend(patterns);
        }

        all_patterns
    }

    pub fn get_statistics(&self) -> PatternStatistics {
        let total_patterns = self.learned_patterns.len();

        let mut by_type: HashMap<String, usize> = HashMap::new();
        let mut high_value_count = 0;
        let mut avg_success_rate = 0.0;

        for pattern in self.learned_patterns.values() {
            *by_type.entry(pattern.pattern_type.clone()).or_insert(0) += 1;
            avg_success_rate += pattern.success_rate;

            if pattern.success_rate >= 0.7 && pattern.frequency >= 5 {
                high_value_count += 1;
            }
        }

        avg_success_rate /= total_patterns.max(1) as f64;

        PatternStatistics {
            total_patterns,
            by_type,
            high_value_patterns: high_value_count,
            average_success_rate: avg_success_rate,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternStatistics {
    pub total_patterns: usize,
    pub by_type: HashMap<String, usize>,
    pub high_value_patterns: usize,
    pub average_success_rate: f64,
}

pub struct CrossSessionLearner {
    pattern_learner: PatternLearner,
    session_patterns: HashMap<String, Vec<String>>,
}

impl Default for CrossSessionLearner {
    fn default() -> Self {
        Self::new()
    }
}

impl CrossSessionLearner {
    pub fn new() -> Self {
        Self {
            pattern_learner: PatternLearner::default(),
            session_patterns: HashMap::new(),
        }
    }

    pub fn learn_from_sessions(
        &mut self,
        trajectories_by_session: HashMap<String, Vec<Trajectory>>,
    ) -> Vec<TrajectoryPattern> {
        let mut all_patterns = Vec::new();

        for (session_id, trajectories) in &trajectories_by_session {
            let mut session_pattern_ids = Vec::new();

            for trajectory in trajectories {
                let patterns = self.pattern_learner.learn_from_trajectory(trajectory);
                session_pattern_ids.extend(patterns.iter().map(|p| p.id.clone()));
                all_patterns.extend(patterns);
            }

            self.session_patterns
                .insert(session_id.clone(), session_pattern_ids);
        }

        let cross_session_patterns = self.extract_cross_session_patterns(&trajectories_by_session);
        all_patterns.extend(cross_session_patterns);

        all_patterns
    }

    fn extract_cross_session_patterns(
        &self,
        trajectories_by_session: &HashMap<String, Vec<Trajectory>>,
    ) -> Vec<TrajectoryPattern> {
        let mut tool_sequences: HashMap<String, Vec<&Trajectory>> = HashMap::new();

        for trajectories in trajectories_by_session.values() {
            for trajectory in trajectories {
                let tools: String = trajectory
                    .steps
                    .iter()
                    .filter_map(|s| {
                        s.tool_calls
                            .as_ref()
                            .and_then(|c| c.first().map(|tc| tc.name.clone()))
                    })
                    .collect::<Vec<_>>()
                    .join("->");

                if !tools.is_empty() {
                    tool_sequences.entry(tools).or_default().push(trajectory);
                }
            }
        }

        let mut patterns = Vec::new();

        for (sequence, trajectories) in tool_sequences {
            if trajectories.len() >= 2 {
                let avg_quality: f64 = trajectories.iter().map(|t| t.quality.overall).sum::<f64>()
                    / trajectories.len() as f64;

                if avg_quality >= 0.6 {
                    patterns.push(TrajectoryPattern {
                        id: Uuid::new_v4().to_string(),
                        name: format!("cross-session-{}", trajectories.len()),
                        description: format!(
                            "Cross-session pattern: {} appeared in {} sessions with avg quality {:.2}",
                            sequence,
                            trajectories.len(),
                            avg_quality
                        ),
                        pattern_type: PatternType::MultiStep.as_str().to_string(),
                        trajectory_ids: trajectories.iter().map(|t| t.id.clone()).collect(),
                        frequency: trajectories.len() as u32,
                        success_rate: trajectories
                            .iter()
                            .filter(|t| t.outcome == TrajectoryOutcome::Success)
                            .count() as f64
                            / trajectories.len() as f64,
                        average_quality: avg_quality,
                        average_value_score: trajectories.iter().map(|t| t.value_score).sum::<f64>()
                            / trajectories.len() as f64,
                        reward_profile: Vec::new(),
                        created_at: Utc::now(),
                    });
                }
            }
        }

        patterns
    }

    pub fn get_cross_session_insights(&self) -> Vec<CrossSessionInsight> {
        let mut insights = Vec::new();

        let pattern_freq: HashMap<&str, usize> =
            self.session_patterns
                .values()
                .flatten()
                .fold(HashMap::new(), |mut acc, id| {
                    *acc.entry(id.as_str()).or_insert(0) += 1;
                    acc
                });

        let high_freq: Vec<_> = pattern_freq
            .iter()
            .filter(|(_, &count)| count >= 3)
            .collect();

        if !high_freq.is_empty() {
            insights.push(CrossSessionInsight {
                insight_type: "recurring_pattern".to_string(),
                description: format!(
                    "Found {} patterns that appear across multiple sessions",
                    high_freq.len()
                ),
                patterns: high_freq.iter().map(|(id, _)| (*id).to_string()).collect(),
                confidence: 0.8,
            });
        }

        insights
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossSessionInsight {
    pub insight_type: String,
    pub description: String,
    pub patterns: Vec<String>,
    pub confidence: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trajectory::{MessageRole, ToolCall, ToolResult, TrajectoryStep};

    fn create_test_trajectory(tools: Vec<&str>, has_error: bool) -> Trajectory {
        let steps: Vec<TrajectoryStep> = tools
            .iter()
            .enumerate()
            .map(|(i, &tool_name)| {
                let tool_calls = if i == 0 {
                    Some(vec![ToolCall {
                        id: format!("call_{}", i),
                        name: tool_name.to_string(),
                        arguments: "{}".to_string(),
                    }])
                } else {
                    None
                };

                let tool_results = if i == 1 {
                    Some(vec![ToolResult {
                        tool_use_id: format!("call_{}", i - 1),
                        tool_name: tool_name.to_string(),
                        output: "result".to_string(),
                        is_error: if i == 1 { has_error } else { false },
                    }])
                } else {
                    None
                };

                TrajectoryStep {
                    timestamp_ms: (i as u64 + 1) * 1000,
                    role: if i == 0 {
                        MessageRole::User
                    } else {
                        MessageRole::Assistant
                    },
                    content: format!("Step {}", i),
                    reasoning: if i == 1 {
                        Some("Thinking...".to_string())
                    } else {
                        None
                    },
                    tool_calls,
                    tool_results,
                }
            })
            .collect();

        Trajectory::new(
            format!("session_{}", tools.join("-")),
            "user".to_string(),
            "Test task".to_string(),
            "Test summary".to_string(),
            TrajectoryOutcome::Success,
            5000,
            steps,
        )
    }

    #[test]
    fn test_tool_sequence_extraction() {
        let mut learner = PatternLearner::default();
        let trajectory = create_test_trajectory(vec!["read_file", "edit_file"], false);

        let patterns = learner.learn_from_trajectory(&trajectory);
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_pattern_statistics() {
        let learner = PatternLearner::default();
        let stats = learner.get_statistics();

        assert_eq!(stats.total_patterns, 0);
        assert!(stats.by_type.is_empty());
    }
}
