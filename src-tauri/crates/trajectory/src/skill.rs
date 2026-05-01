//! Skill optimization closed-loop module
//!
//! Provides skill creation, improvement, and management capabilities

use crate::trajectory::{Trajectory, TrajectoryOutcome};
use axagent_core::types::{ChatTool, ChatToolFunction};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub content: String,
    pub category: String,
    pub tags: Vec<String>,
    pub platforms: Vec<String>,
    pub scenarios: Vec<String>,
    pub quality_score: f64,
    pub success_rate: f64,
    pub avg_execution_time_ms: u64,
    pub total_usages: u32,
    pub successful_usages: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub metadata: SkillMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub hermes: HermesMetadata,
    pub references: Vec<SkillReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HermesMetadata {
    pub tags: Vec<String>,
    pub category: String,
    pub fallback_for_toolsets: Vec<String>,
    pub requires_toolsets: Vec<String>,
    pub config: Vec<SkillConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill_dependencies: Option<Vec<SkillDependency>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDependency {
    pub name: String,
    pub version_constraint: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyValidationResult {
    pub satisfied: bool,
    pub missing_dependencies: Vec<SkillDependency>,
    pub satisfied_dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillConfig {
    pub key: String,
    pub description: String,
    pub default: String,
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillReference {
    pub path: String,
    pub content: String,
}

impl Default for SkillMetadata {
    fn default() -> Self {
        Self {
            hermes: HermesMetadata {
                tags: Vec::new(),
                category: "general".to_string(),
                fallback_for_toolsets: Vec::new(),
                requires_toolsets: Vec::new(),
                config: Vec::new(),
                ..Default::default()
            },
            references: Vec::new(),
        }
    }
}

impl Default for HermesMetadata {
    fn default() -> Self {
        Self {
            tags: Vec::new(),
            category: "general".to_string(),
            fallback_for_toolsets: Vec::new(),
            requires_toolsets: Vec::new(),
            config: Vec::new(),
            source_kind: None,
            source_ref: None,
            commit: None,
            skill_dependencies: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillOutcome {
    Success,
    Partial,
    Failure,
}

impl From<TrajectoryOutcome> for SkillOutcome {
    fn from(outcome: TrajectoryOutcome) -> Self {
        match outcome {
            TrajectoryOutcome::Success => SkillOutcome::Success,
            TrajectoryOutcome::Partial => SkillOutcome::Partial,
            TrajectoryOutcome::Failure => SkillOutcome::Failure,
            TrajectoryOutcome::Abandoned => SkillOutcome::Failure,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExecution {
    pub skill_id: String,
    pub timestamp: DateTime<Utc>,
    pub outcome: SkillOutcome,
    pub execution_time_ms: u64,
    pub context: SkillContext,
    pub input_args: Option<serde_json::Value>,
    pub output_result: Option<serde_json::Value>,
    pub feedback: Option<String>,
    pub error_message: Option<String>,
}

impl SkillExecution {
    pub fn with_args(mut self, args: serde_json::Value) -> Self {
        self.input_args = Some(args);
        self
    }

    pub fn with_result(mut self, result: serde_json::Value) -> Self {
        self.output_result = Some(result);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillContext {
    pub user_input: String,
    pub task_type: String,
    pub complexity: TaskComplexity,
    pub entities: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskComplexity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillModification {
    pub modification_type: ModificationType,
    pub old_content: Option<String>,
    pub new_content: String,
    pub reason: String,
    pub confidence: f64,
    pub validation_result: Option<ValidationResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModificationType {
    ContentPatch,
    DescriptionUpdate,
    ExampleAddition,
    LogicRevision,
    StepRefinement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub success: bool,
    pub quality_delta: f64,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillProposal {
    pub task_description: String,
    pub suggested_name: String,
    pub suggested_content: String,
    pub confidence: f64,
    pub trigger_event: String,
    pub similar_skills: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SkillEvolution {
    pub skill_id: String,
    pub version: String,
    pub modifications: Vec<SkillModification>,
    pub outcome: EvolutionOutcome,
    pub metrics_delta: MetricsDelta,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EvolutionOutcome {
    Improved,
    Degraded,
    Unchanged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsDelta {
    pub success_rate_delta: f64,
    pub avg_execution_time_delta: i64,
    pub quality_score_delta: f64,
}

impl Skill {
    pub fn new(name: String, description: String, content: String, category: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            version: "1.0.0".to_string(),
            content,
            category,
            tags: Vec::new(),
            platforms: Vec::new(),
            scenarios: Vec::new(),
            quality_score: 0.5,
            success_rate: 0.0,
            avg_execution_time_ms: 0,
            total_usages: 0,
            successful_usages: 0,
            created_at: now,
            updated_at: now,
            last_used_at: None,
            metadata: SkillMetadata::default(),
        }
    }

    pub fn from_trajectory(trajectory: &Trajectory, name: String) -> Self {
        let content = Self::generate_content_from_trajectory(trajectory);
        let mut skill = Self::new(
            name,
            format!("Skill for: {}", trajectory.topic),
            content,
            "auto-generated".to_string(),
        );
        skill.quality_score = trajectory.quality.overall;
        skill.success_rate = match trajectory.outcome {
            TrajectoryOutcome::Success => 1.0,
            TrajectoryOutcome::Partial => 0.5,
            TrajectoryOutcome::Failure | TrajectoryOutcome::Abandoned => 0.0,
        };
        skill
    }

    fn generate_content_from_trajectory(trajectory: &Trajectory) -> String {
        let mut content = format!("# {}\n\n", trajectory.topic);
        content += &format!("slug: {}\n\n", slugify(&trajectory.topic));

        content += "## When to Use\n";
        content += &format!("This skill is triggered when: {}\n\n", trajectory.topic);

        content += "## Procedure\n";
        for (i, step) in trajectory.steps.iter().enumerate() {
            if step.role == crate::trajectory::MessageRole::Assistant {
                if let Some(ref tool_calls) = step.tool_calls {
                    for tc in tool_calls {
                        content += &format!("{}. Use {} with args: {}\n", i, tc.name, tc.arguments);
                    }
                }
            }
        }
        content += "\n";

        content += "## Pitfalls\n";
        match trajectory.outcome {
            TrajectoryOutcome::Failure => {
                content += "- An error occurred during execution, ensure proper error handling\n";
            },
            _ => {
                content += "- No known issues\n";
            },
        }

        content += "\n## Verification\n";
        content += &format!("Result: {}\n", trajectory.summary);

        content
    }

    pub fn record_execution(&mut self, execution: &SkillExecution) {
        self.total_usages += 1;
        match execution.outcome {
            SkillOutcome::Success => {
                self.successful_usages += 1;
            },
            SkillOutcome::Partial => {
                self.successful_usages += 1;
            },
            SkillOutcome::Failure => {},
        }

        let n = self.total_usages as f64;
        self.success_rate = self.successful_usages as f64 / n;

        let exec_time = execution.execution_time_ms as f64;
        self.avg_execution_time_ms =
            ((self.avg_execution_time_ms as f64 * (n - 1.0)) + exec_time / n) as u64;

        self.last_used_at = Some(execution.timestamp);
    }

    pub fn needs_improvement(&self, min_usages: u32, success_threshold: f64) -> bool {
        self.total_usages >= min_usages && self.success_rate < success_threshold
    }

    pub fn bump_version(&mut self) {
        let parts: Vec<&str> = self.version.split('.').collect();
        let major: u32 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(1);
        let minor: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        self.version = format!("{}.{}.0", major, minor + 1);
        self.updated_at = Utc::now();
    }

    pub fn to_tool_definition(&self) -> ChatTool {
        let parameters = self.generate_tool_parameters();
        ChatTool {
            r#type: "function".to_string(),
            function: ChatToolFunction {
                name: format!("skill_{}", self.name.replace(' ', "_").to_lowercase()),
                description: Some(self.description.clone()),
                parameters: Some(parameters),
            },
        }
    }

    fn generate_tool_parameters(&self) -> serde_json::Value {
        let mut props = serde_json::Map::new();
        props.insert(
            "input".to_string(),
            serde_json::json!({
                "type": "object",
                "description": "The input task or query for this skill",
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "The specific task to execute"
                    },
                    "context": {
                        "type": "object",
                        "description": "Additional context for the task",
                        "properties": {
                            "goal": {
                                "type": "string",
                                "description": "The overall goal to achieve"
                            },
                            "constraints": {
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "Any constraints or requirements"
                            }
                        }
                    }
                },
                "required": vec!["task"]
            }),
        );
        if !self.metadata.hermes.config.is_empty() {
            if let Some(input_obj) = props.get_mut("input").and_then(|v| v.as_object_mut()) {
                if let Some(props_obj) = input_obj
                    .get_mut("properties")
                    .and_then(|v| v.as_object_mut())
                {
                    for config in &self.metadata.hermes.config {
                        props_obj.insert(
                            config.key.clone(),
                            serde_json::json!({
                                "type": "string",
                                "description": config.description,
                                "default": config.default
                            }),
                        );
                    }
                }
            }
        }
        serde_json::json!({
            "type": "object",
            "properties": props,
            "required": vec!["input"]
        })
    }

    pub fn extract_scenarios_from_content(&self) -> Vec<String> {
        let mut scenarios = Vec::new();
        for line in self.content.lines() {
            let lower = line.to_lowercase();
            if lower.starts_with("when:") || lower.starts_with("scenario:") {
                let scenario = line.split(':').nth(1).map(|s| s.trim().to_string());
                if let Some(s) = scenario {
                    scenarios.push(s);
                }
            }
        }
        if scenarios.is_empty() && !self.scenarios.is_empty() {
            return self.scenarios.clone();
        }
        scenarios
    }

    pub fn validate_dependencies(&self, installed_skills: &[String]) -> DependencyValidationResult {
        let dependencies = self.metadata.hermes.skill_dependencies.as_ref();
        let deps = match dependencies {
            Some(d) => d,
            None => {
                return DependencyValidationResult {
                    satisfied: true,
                    missing_dependencies: Vec::new(),
                    satisfied_dependencies: Vec::new(),
                }
            },
        };

        let mut missing = Vec::new();
        let mut satisfied = Vec::new();

        for dep in deps {
            if installed_skills.iter().any(|s| s == &dep.name) {
                satisfied.push(dep.name.clone());
            } else if dep.required {
                missing.push(dep.clone());
            }
        }

        DependencyValidationResult {
            satisfied: missing.is_empty(),
            missing_dependencies: missing,
            satisfied_dependencies: satisfied,
        }
    }
}

fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .collect::<String>()
        .split_whitespace()
        .take(5)
        .collect::<Vec<_>>()
        .join("-")
}

pub struct SkillOptimizer {
    min_usages_for_analysis: u32,
    #[allow(dead_code)]
    improvement_threshold: f64,
    #[allow(dead_code)]
    quality_threshold: f64,
}

impl Default for SkillOptimizer {
    fn default() -> Self {
        Self {
            min_usages_for_analysis: 5,
            improvement_threshold: 0.5,
            quality_threshold: 0.3,
        }
    }
}

impl SkillOptimizer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn analyze_skill(&self, skill: &Skill) -> Option<SkillAnalysis> {
        if skill.total_usages < self.min_usages_for_analysis {
            return None;
        }

        let mut suggestions = Vec::new();
        let mut impact;

        if skill.success_rate < 0.3 {
            suggestions.push("Consider simplifying the skill procedure".to_string());
            suggestions.push("Add more error handling guidance".to_string());
            impact = Impact::High;
        } else if skill.success_rate < 0.5 {
            suggestions.push("Review and clarify ambiguous steps".to_string());
            suggestions.push("Add verification checkpoints".to_string());
            impact = Impact::Medium;
        } else {
            suggestions.push("Skill is performing well, no major changes needed".to_string());
            impact = Impact::Low;
        }

        if skill.avg_execution_time_ms > 30000 {
            suggestions.push(
                "The skill may be too complex - consider splitting into smaller skills".to_string(),
            );
            impact = Impact::High;
        }

        Some(SkillAnalysis {
            skill_id: skill.id.clone(),
            suggestions,
            estimated_impact: impact,
            reason: format!(
                "Success rate: {:.0}% over {} executions",
                skill.success_rate * 100.0,
                skill.total_usages
            ),
        })
    }

    pub fn propose_improvement(
        &self,
        skill: &Skill,
        _failed_trajectories: &[&Trajectory],
    ) -> Option<SkillModification> {
        let analysis = self.analyze_skill(skill)?;

        if analysis.suggestions.is_empty() {
            return None;
        }

        let modification_type;
        let new_content = skill.content.clone();
        let reason;

        if analysis.suggestions.iter().any(|s| s.contains("simplif")) {
            modification_type = ModificationType::ContentPatch;
            reason = "Simplify skill procedure based on failure analysis".to_string();
        } else if analysis
            .suggestions
            .iter()
            .any(|s| s.contains("error handling"))
        {
            modification_type = ModificationType::LogicRevision;
            reason = "Add error handling guidance".to_string();
        } else if analysis
            .suggestions
            .iter()
            .any(|s| s.contains("verification"))
        {
            modification_type = ModificationType::StepRefinement;
            reason = "Add verification checkpoints".to_string();
        } else {
            modification_type = ModificationType::DescriptionUpdate;
            reason = "General improvement".to_string();
        }

        Some(SkillModification {
            modification_type,
            old_content: Some(skill.content.clone()),
            new_content,
            reason,
            confidence: analysis.estimated_impact.to_confidence(),
            validation_result: None,
        })
    }

    pub fn validate_modification(
        &self,
        skill: &Skill,
        modification: &SkillModification,
        test_trajectories: &[&Trajectory],
    ) -> ValidationResult {
        let mut issues = Vec::new();
        let mut improvements: f64 = 0.0;

        let original_rate = skill.success_rate;

        let test_outcomes: Vec<_> = test_trajectories
            .iter()
            .filter_map(|t| {
                if t.topic.to_lowercase().contains(&skill.name.to_lowercase()) {
                    Some(t.outcome)
                } else {
                    None
                }
            })
            .collect();

        if !test_outcomes.is_empty() {
            let successful = test_outcomes
                .iter()
                .filter(|o| matches!(o, TrajectoryOutcome::Success | TrajectoryOutcome::Partial))
                .count();
            let new_rate = successful as f64 / test_outcomes.len() as f64;
            improvements = new_rate - original_rate;
        }

        if modification.new_content.len() < skill.content.len() / 2 {
            issues.push("Modification may have removed too much content".to_string());
        }

        if modification.new_content.len() > skill.content.len() * 3 {
            issues.push("Modification significantly increases skill complexity".to_string());
        }

        ValidationResult {
            success: issues.is_empty() && improvements >= 0.0,
            quality_delta: improvements,
            issues,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Impact {
    High,
    Medium,
    Low,
}

impl Impact {
    fn to_confidence(self) -> f64 {
        match self {
            Impact::High => 0.9,
            Impact::Medium => 0.7,
            Impact::Low => 0.5,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillAnalysis {
    pub skill_id: String,
    pub suggestions: Vec<String>,
    pub estimated_impact: Impact,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillAnalytics {
    pub total_executions: u32,
    pub success_rate: f64,
    pub avg_execution_time_ms: f64,
    pub recent_executions: u32,
}

pub struct SkillCreator {
    min_tool_calls: usize,
    complexity_threshold: TaskComplexity,
}

impl Default for SkillCreator {
    fn default() -> Self {
        Self {
            min_tool_calls: 3,
            complexity_threshold: TaskComplexity::Medium,
        }
    }
}

impl SkillCreator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn should_create_skill(&self, trajectory: &Trajectory) -> bool {
        if trajectory.steps.len() < self.min_tool_calls {
            return false;
        }

        let tool_call_count = trajectory
            .steps
            .iter()
            .filter(|s| s.tool_calls.is_some())
            .count();

        if tool_call_count < self.min_tool_calls {
            return false;
        }

        let complexity = self.assess_complexity(trajectory);
        complexity >= self.complexity_threshold
    }

    fn assess_complexity(&self, trajectory: &Trajectory) -> TaskComplexity {
        let tool_count: usize = trajectory
            .steps
            .iter()
            .filter_map(|s| s.tool_calls.as_ref().map(|c| c.len()))
            .sum();

        let has_error = trajectory.steps.iter().any(|s| {
            s.tool_results
                .as_ref()
                .is_some_and(|r| r.iter().any(|tr| tr.is_error))
        });

        let has_reasoning = trajectory.steps.iter().any(|s| s.reasoning.is_some());

        let complexity_score = tool_count as f64 * 0.3
            + if has_error { 0.3 } else { 0.0 }
            + if has_reasoning { 0.2 } else { 0.0 }
            + trajectory.steps.len() as f64 * 0.01;

        if complexity_score >= 3.0 {
            TaskComplexity::High
        } else if complexity_score >= 1.5 {
            TaskComplexity::Medium
        } else {
            TaskComplexity::Low
        }
    }

    pub fn create_proposal(&self, trajectory: &Trajectory) -> SkillProposal {
        let task_description = trajectory.topic.clone();
        let suggested_name = self.generate_skill_name(&task_description);
        let suggested_content = Skill::generate_content_from_trajectory(trajectory);

        SkillProposal {
            task_description,
            suggested_name,
            suggested_content,
            confidence: trajectory.quality.overall,
            trigger_event: format!(
                "Complex task with {} tool calls, outcome: {:?}",
                trajectory
                    .steps
                    .iter()
                    .filter_map(|s| s.tool_calls.as_ref().map(|c| c.len()))
                    .sum::<usize>(),
                trajectory.outcome
            ),
            similar_skills: Vec::new(),
        }
    }

    fn generate_skill_name(&self, topic: &str) -> String {
        let words: Vec<&str> = topic.split_whitespace().take(3).collect();
        let base = words.join("-");

        if base.len() < 3 {
            format!("skill-{}", slugify(topic))
        } else {
            slugify(&base)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_creation() {
        let _creator = SkillCreator::new();

        let skill = Skill::new(
            "test-skill".to_string(),
            "A test skill".to_string(),
            "# Test\n\nContent".to_string(),
            "testing".to_string(),
        );

        assert_eq!(skill.version, "1.0.0");
        assert_eq!(skill.total_usages, 0);
    }

    #[test]
    fn test_skill_execution_recording() {
        let mut skill = Skill::new(
            "test".to_string(),
            "test".to_string(),
            "content".to_string(),
            "test".to_string(),
        );

        let execution = SkillExecution {
            skill_id: skill.id.clone(),
            timestamp: Utc::now(),
            outcome: SkillOutcome::Success,
            execution_time_ms: 1000,
            context: SkillContext {
                user_input: "test".to_string(),
                task_type: "test".to_string(),
                complexity: TaskComplexity::Low,
                entities: Vec::new(),
            },
            input_args: None,
            output_result: None,
            feedback: None,
            error_message: None,
        };

        skill.record_execution(&execution);
        assert_eq!(skill.total_usages, 1);
        assert_eq!(skill.successful_usages, 1);
        assert_eq!(skill.success_rate, 1.0);
    }

    #[test]
    fn test_skill_needs_improvement() {
        let mut skill = Skill::new(
            "test".to_string(),
            "test".to_string(),
            "content".to_string(),
            "test".to_string(),
        );

        skill.total_usages = 10;
        skill.successful_usages = 3;

        assert!(skill.needs_improvement(5, 0.5));
        assert!(!skill.needs_improvement(5, 0.2));
    }
}
