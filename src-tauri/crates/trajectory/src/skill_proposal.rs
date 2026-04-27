//! Skill Proposal Service - Analyzes trajectories and proposes skill creation/evolution
//!
//! This module provides the bridge between trajectory learning and skill generation.
//! It analyzes completed agent sessions and proposes:
//! - New skills for successful multi-step workflows
//! - Skill improvements based on failure patterns
//! - Skill template generation from successful trajectories

use crate::skill::SkillProposal;
use crate::skill_manager::SkillCreationParams;
use crate::storage::TrajectoryStorage;
use crate::trajectory::{Trajectory, TrajectoryOutcome};
use std::collections::HashMap;
use std::sync::Arc;

const MIN_STEPS_FOR_PROPOSAL: usize = 3;
const MIN_SUCCESSFUL_TRAJECTORIES: usize = 2;
const MAX_PROPOSALS_STORED: usize = 50;

pub struct SkillProposalService {
    #[allow(dead_code)]
    storage: Arc<TrajectoryStorage>,
    recent_proposals: Vec<SkillProposal>,
    topic_trajectory_count: HashMap<String, usize>,
}

impl SkillProposalService {
    pub fn new(storage: Arc<TrajectoryStorage>) -> Self {
        Self {
            storage,
            recent_proposals: Vec::new(),
            topic_trajectory_count: HashMap::new(),
        }
    }

    pub fn analyze_and_propose(&mut self, trajectory: &Trajectory) -> Option<SkillProposal> {
        let step_count = trajectory.steps.len();
        let has_tool_usage = trajectory.steps.iter().any(|s| s.tool_calls.is_some());

        if step_count < MIN_STEPS_FOR_PROPOSAL || !has_tool_usage {
            return None;
        }

        let topic_key = trajectory.topic.to_lowercase();

        let count = self
            .topic_trajectory_count
            .entry(topic_key.clone())
            .or_insert(0);
        *count += 1;

        let should_propose = match trajectory.outcome {
            TrajectoryOutcome::Success => *count >= MIN_SUCCESSFUL_TRAJECTORIES,
            TrajectoryOutcome::Failure | TrajectoryOutcome::Abandoned => {
                *count >= 1 && step_count >= 5
            }
            TrajectoryOutcome::Partial => *count >= MIN_SUCCESSFUL_TRAJECTORIES,
        };

        if !should_propose {
            return None;
        }

        let similar_proposals: Vec<String> = self
            .recent_proposals
            .iter()
            .filter(|p| p.similar_skills.contains(&topic_key))
            .map(|p| p.suggested_name.clone())
            .collect();

        if similar_proposals.len() >= 2 {
            return None;
        }

        let proposal = self.generate_proposal(trajectory, &topic_key);

        self.recent_proposals.push(proposal.clone());
        if self.recent_proposals.len() > MAX_PROPOSALS_STORED {
            self.recent_proposals.remove(0);
        }

        Some(proposal)
    }

    fn generate_proposal(&self, trajectory: &Trajectory, topic_key: &str) -> SkillProposal {
        let suggested_name = self.generate_skill_name(trajectory);
        let suggested_content = self.generate_skill_content(trajectory);
        let confidence = self.calculate_confidence(trajectory);

        let trigger_event = match trajectory.outcome {
            TrajectoryOutcome::Success => "successful_multi_step_workflow".to_string(),
            TrajectoryOutcome::Failure => "failed_workflow_needing_improvement".to_string(),
            TrajectoryOutcome::Partial => "partial_success_requiring_refinement".to_string(),
            TrajectoryOutcome::Abandoned => "incomplete_workflow".to_string(),
        };

        SkillProposal {
            task_description: trajectory.topic.clone(),
            suggested_name,
            suggested_content,
            confidence,
            trigger_event,
            similar_skills: vec![topic_key.to_string()],
        }
    }

    fn generate_skill_name(&self, trajectory: &Trajectory) -> String {
        let topic = &trajectory.topic;
        let words: Vec<&str> = topic.split_whitespace().collect();

        if words.len() <= 3 {
            slugify(topic)
        } else {
            let first_two = words.iter().take(2).copied().collect::<Vec<_>>().join(" ");
            slugify(&first_two)
        }
    }

    fn generate_skill_content(&self, trajectory: &Trajectory) -> String {
        let mut content = format!("# {}\n\n", trajectory.topic);
        content += &format!("slug: {}\n\n", slugify(&trajectory.topic));

        content += "## Overview\n";
        content += &format!("This skill handles: {}. ", trajectory.topic);
        content += &format!(
            "Outcome: {:?}, Duration: {}ms\n\n",
            trajectory.outcome, trajectory.duration_ms
        );

        let tool_steps: Vec<_> = trajectory
            .steps
            .iter()
            .filter(|s| s.tool_calls.is_some())
            .collect();

        if !tool_steps.is_empty() {
            content += "## Procedure\n";
            for (i, step) in tool_steps.iter().enumerate() {
                if let Some(ref calls) = step.tool_calls {
                    for call in calls {
                        content += &format!(
                            "{}. Use `{}` with args: {}\n",
                            i + 1,
                            call.name,
                            truncate_args(&call.arguments, 200)
                        );
                    }
                }
            }
            content += "\n";
        }

        if let Some(ref summary) = trajectory.patterns.first() {
            content += &format!("## Patterns\n{}\n\n", summary);
        }

        content += &format!(
            "## Quality\n- Task completion: {:.1}%\n- Tool efficiency: {:.1}%\n",
            trajectory.quality.task_completion * 100.0,
            trajectory.quality.tool_efficiency * 100.0
        );

        content
    }

    fn calculate_confidence(&self, trajectory: &Trajectory) -> f64 {
        let mut confidence = 0.5;

        confidence += trajectory.quality.overall * 0.3;

        let tool_count = trajectory
            .steps
            .iter()
            .filter(|s| s.tool_calls.is_some())
            .count();
        if tool_count >= 3 {
            confidence += 0.1;
        }

        match trajectory.outcome {
            TrajectoryOutcome::Success => confidence += 0.15,
            TrajectoryOutcome::Partial => confidence += 0.05,
            _ => {}
        }

        confidence.min(1.0)
    }

    pub fn get_proposals(&self) -> Vec<SkillProposal> {
        self.recent_proposals.clone()
    }

    pub fn clear_proposal(&mut self, task_description: &str) {
        self.recent_proposals
            .retain(|p| p.task_description != task_description);
    }

    pub fn get_proposal_by_name(&self, name: &str) -> Option<&SkillProposal> {
        self.recent_proposals
            .iter()
            .find(|p| p.suggested_name == name)
    }
}

fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>()
}

fn truncate_args(args: &str, max_len: usize) -> String {
    if args.len() <= max_len {
        args.to_string()
    } else {
        format!("{}...", &args[..max_len])
    }
}

pub fn create_skill_from_proposal(proposal: &SkillProposal) -> SkillCreationParams {
    SkillCreationParams {
        name: proposal.suggested_name.clone(),
        description: proposal.task_description.clone(),
        content: proposal.suggested_content.clone(),
        category: Some("auto-generated".to_string()),
        tags: Some(vec![
            proposal.trigger_event.clone(),
            "auto-evolved".to_string(),
        ]),
        platforms: None,
    }
}
