// SPDX-License-Identifier: AGPL-3.0-only

//! Environment co-evolution for dynamic benchmarking
//!
//! Provides adaptive difficulty adjustment and task generation that co-evolves
//! with agent capabilities, ensuring continuous challenge and growth.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DifficultyLevel {
    Easy,
    Medium,
    Hard,
    Expert,
}

impl DifficultyLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            DifficultyLevel::Easy => "easy",
            DifficultyLevel::Medium => "medium",
            DifficultyLevel::Hard => "hard",
            DifficultyLevel::Expert => "expert",
        }
    }

    pub fn from_difficulty_score(score: f64) -> Self {
        if score < 0.25 {
            DifficultyLevel::Easy
        } else if score < 0.5 {
            DifficultyLevel::Medium
        } else if score < 0.75 {
            DifficultyLevel::Hard
        } else {
            DifficultyLevel::Expert
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTemplate {
    pub id: String,
    pub category: String,
    pub difficulty: DifficultyLevel,
    pub prompt_template: String,
    pub expected_patterns: Vec<String>,
}

impl TaskTemplate {
    pub fn new(
        category: String,
        difficulty: DifficultyLevel,
        prompt_template: String,
        expected_patterns: Vec<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            category,
            difficulty,
            prompt_template,
            expected_patterns,
        }
    }

    pub fn instantiate(&self, difficulty_score: f64) -> TaskTemplate {
        let mut task = self.clone();
        task.id = Uuid::new_v4().to_string();
        task.difficulty = DifficultyLevel::from_difficulty_score(difficulty_score);
        task.prompt_template = task.prompt_template.replace(
            "{difficulty}",
            DifficultyLevel::from_difficulty_score(difficulty_score).as_str(),
        );
        task
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoevolutionConfig {
    pub initial_difficulty: f64,
    pub difficulty_step: f64,
    pub max_difficulty: f64,
    pub performance_window: usize,
    pub target_success_rate: f64,
}

impl Default for CoevolutionConfig {
    fn default() -> Self {
        Self {
            initial_difficulty: 0.1,
            difficulty_step: 0.05,
            max_difficulty: 1.0,
            performance_window: 5,
            target_success_rate: 0.7,
        }
    }
}

pub struct CoevolutionEnvironment {
    config: CoevolutionConfig,
    difficulty_level: f64,
    agent_performance_history: Vec<f64>,
    task_templates: Vec<TaskTemplate>,
    generated_tasks: Vec<TaskTemplate>,
    category_performance: HashMap<String, Vec<f64>>,
}

impl CoevolutionEnvironment {
    pub fn new(config: CoevolutionConfig) -> Self {
        let initial_difficulty = config.initial_difficulty;
        Self {
            config,
            difficulty_level: initial_difficulty,
            agent_performance_history: Vec::new(),
            task_templates: Vec::new(),
            generated_tasks: Vec::new(),
            category_performance: HashMap::new(),
        }
    }

    pub fn task_count(&self) -> usize {
        self.generated_tasks.len()
    }

    pub fn generate_task(&mut self) -> TaskTemplate {
        let weak_category = self.find_weakest_category();

        let template = if let Some(category) = &weak_category {
            self.task_templates
                .iter()
                .find(|t| &t.category == category)
                .or_else(|| self.task_templates.first())
        } else {
            self.task_templates.first()
        };

        let task = if let Some(tmpl) = template {
            tmpl.instantiate(self.difficulty_level)
        } else {
            TaskTemplate::new(
                weak_category.unwrap_or_else(|| "general".to_string()),
                DifficultyLevel::from_difficulty_score(self.difficulty_level),
                format!(
                    "Complete a {} difficulty task in {{difficulty}} mode",
                    DifficultyLevel::from_difficulty_score(self.difficulty_level).as_str()
                ),
                vec!["completion".to_string()],
            )
        };

        self.generated_tasks.push(task.clone());
        task
    }

    fn find_weakest_category(&self) -> Option<String> {
        if self.task_templates.is_empty() {
            return None;
        }

        let categories: std::collections::HashSet<&str> = self
            .task_templates
            .iter()
            .map(|t| t.category.as_str())
            .collect();

        let mut weakest: Option<&str> = None;
        let mut weakest_score = f64::MAX;

        for category in &categories {
            let score = self
                .category_performance
                .get(*category)
                .map(|perf| {
                    if perf.is_empty() {
                        0.0
                    } else {
                        perf.iter().sum::<f64>() / perf.len() as f64
                    }
                })
                .unwrap_or(0.0);

            if score < weakest_score {
                weakest_score = score;
                weakest = Some(category);
            }
        }

        weakest.map(|s| s.to_string())
    }

    pub fn update_performance(&mut self, success_rate: f64) {
        self.agent_performance_history.push(success_rate);

        if self.should_increase_difficulty() {
            self.difficulty_level = (self.difficulty_level + self.config.difficulty_step)
                .min(self.config.max_difficulty);
        } else if self.should_decrease_difficulty() {
            self.difficulty_level = (self.difficulty_level - self.config.difficulty_step).max(0.0);
        }
    }

    pub fn update_category_performance(&mut self, category: &str, success_rate: f64) {
        self.category_performance
            .entry(category.to_string())
            .or_default()
            .push(success_rate);
        self.update_performance(success_rate);
    }

    pub fn should_increase_difficulty(&self) -> bool {
        if self.agent_performance_history.len() < self.config.performance_window {
            return false;
        }

        let recent_start = self
            .agent_performance_history
            .len()
            .saturating_sub(self.config.performance_window);
        let recent: Vec<f64> = self.agent_performance_history[recent_start..].to_vec();

        let avg = recent.iter().sum::<f64>() / recent.len() as f64;
        avg > self.config.target_success_rate
            && recent.iter().all(|&r| r > self.config.target_success_rate)
    }

    pub fn should_decrease_difficulty(&self) -> bool {
        if self.agent_performance_history.len() < self.config.performance_window {
            return false;
        }

        let recent_start = self
            .agent_performance_history
            .len()
            .saturating_sub(self.config.performance_window);
        let recent: Vec<f64> = self.agent_performance_history[recent_start..].to_vec();

        let avg = recent.iter().sum::<f64>() / recent.len() as f64;
        avg < self.config.target_success_rate * 0.5
    }

    pub fn get_difficulty_level(&self) -> DifficultyLevel {
        DifficultyLevel::from_difficulty_score(self.difficulty_level)
    }

    pub fn add_task_template(&mut self, template: TaskTemplate) {
        self.task_templates.push(template);
    }

    pub fn difficulty_level(&self) -> f64 {
        self.difficulty_level
    }

    pub fn performance_history(&self) -> &[f64] {
        &self.agent_performance_history
    }

    pub fn generated_tasks(&self) -> &[TaskTemplate] {
        &self.generated_tasks
    }

    pub fn config(&self) -> &CoevolutionConfig {
        &self.config
    }
}

impl Default for CoevolutionEnvironment {
    fn default() -> Self {
        Self::new(CoevolutionConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_difficulty_level_from_score() {
        assert_eq!(DifficultyLevel::from_difficulty_score(0.1), DifficultyLevel::Easy);
        assert_eq!(DifficultyLevel::from_difficulty_score(0.3), DifficultyLevel::Medium);
        assert_eq!(DifficultyLevel::from_difficulty_score(0.6), DifficultyLevel::Hard);
        assert_eq!(DifficultyLevel::from_difficulty_score(0.8), DifficultyLevel::Expert);
    }

    #[test]
    fn test_difficulty_level_as_str() {
        assert_eq!(DifficultyLevel::Easy.as_str(), "easy");
        assert_eq!(DifficultyLevel::Medium.as_str(), "medium");
        assert_eq!(DifficultyLevel::Hard.as_str(), "hard");
        assert_eq!(DifficultyLevel::Expert.as_str(), "expert");
    }

    #[test]
    fn test_task_template_creation() {
        let template = TaskTemplate::new(
            "coding".to_string(),
            DifficultyLevel::Medium,
            "Fix a {difficulty} bug".to_string(),
            vec!["compilation".to_string(), "tests_pass".to_string()],
        );
        assert_eq!(template.category, "coding");
        assert_eq!(template.difficulty, DifficultyLevel::Medium);
        assert_eq!(template.expected_patterns.len(), 2);
    }

    #[test]
    fn test_task_template_instantiate() {
        let template = TaskTemplate::new(
            "coding".to_string(),
            DifficultyLevel::Easy,
            "Fix a {difficulty} bug".to_string(),
            vec!["tests_pass".to_string()],
        );
        let instance = template.instantiate(0.6);
        assert_ne!(instance.id, template.id);
        assert_eq!(instance.difficulty, DifficultyLevel::Hard);
        assert!(instance.prompt_template.contains("hard"));
    }

    #[test]
    fn test_coevolution_initial_difficulty() {
        let env = CoevolutionEnvironment::default();
        assert!((env.difficulty_level() - 0.1).abs() < 1e-6);
        assert_eq!(env.get_difficulty_level(), DifficultyLevel::Easy);
    }

    #[test]
    fn test_coevolution_add_template() {
        let mut env = CoevolutionEnvironment::default();
        let template = TaskTemplate::new(
            "debugging".to_string(),
            DifficultyLevel::Easy,
            "Debug a {difficulty} issue".to_string(),
            vec!["error_found".to_string()],
        );
        env.add_task_template(template);
        let task = env.generate_task();
        assert_eq!(task.category, "debugging");
    }

    #[test]
    fn test_coevolution_increase_difficulty() {
        let config = CoevolutionConfig {
            initial_difficulty: 0.1,
            difficulty_step: 0.05,
            max_difficulty: 1.0,
            performance_window: 3,
            target_success_rate: 0.7,
        };
        let mut env = CoevolutionEnvironment::new(config);
        env.add_task_template(TaskTemplate::new(
            "coding".to_string(),
            DifficultyLevel::Easy,
            "Solve {difficulty} problem".to_string(),
            vec!["pass".to_string()],
        ));

        for _ in 0..3 {
            env.update_performance(0.9);
        }

        assert!(env.difficulty_level() > 0.1);
    }

    #[test]
    fn test_coevolution_decrease_difficulty() {
        let config = CoevolutionConfig {
            initial_difficulty: 0.5,
            difficulty_step: 0.05,
            max_difficulty: 1.0,
            performance_window: 3,
            target_success_rate: 0.7,
        };
        let mut env = CoevolutionEnvironment::new(config);

        for _ in 0..3 {
            env.update_performance(0.1);
        }

        assert!(env.difficulty_level() < 0.5);
    }

    #[test]
    fn test_should_increase_difficulty_insufficient_data() {
        let env = CoevolutionEnvironment::default();
        assert!(!env.should_increase_difficulty());
    }

    #[test]
    fn test_should_decrease_difficulty_insufficient_data() {
        let env = CoevolutionEnvironment::default();
        assert!(!env.should_decrease_difficulty());
    }

    #[test]
    fn test_coevolution_generate_task_no_templates() {
        let mut env = CoevolutionEnvironment::default();
        let task = env.generate_task();
        assert_eq!(task.category, "general");
    }

    #[test]
    fn test_coevolution_weak_category_targeting() {
        let mut env = CoevolutionEnvironment::default();
        env.add_task_template(TaskTemplate::new(
            "coding".to_string(),
            DifficultyLevel::Easy,
            "Code {difficulty}".to_string(),
            vec!["pass".to_string()],
        ));
        env.add_task_template(TaskTemplate::new(
            "debugging".to_string(),
            DifficultyLevel::Easy,
            "Debug {difficulty}".to_string(),
            vec!["found".to_string()],
        ));

        env.update_category_performance("coding", 0.9);
        env.update_category_performance("debugging", 0.2);

        let task = env.generate_task();
        assert_eq!(task.category, "debugging");
    }

    #[test]
    fn test_coevolution_max_difficulty_cap() {
        let config = CoevolutionConfig {
            initial_difficulty: 0.95,
            difficulty_step: 0.1,
            max_difficulty: 1.0,
            performance_window: 2,
            target_success_rate: 0.7,
        };
        let mut env = CoevolutionEnvironment::new(config);

        for _ in 0..5 {
            env.update_performance(0.9);
        }

        assert!(env.difficulty_level() <= 1.0);
    }
}
