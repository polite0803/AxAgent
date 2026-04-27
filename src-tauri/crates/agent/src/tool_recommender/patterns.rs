use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePattern {
    pub pattern_id: String,
    pub task_signature: String,
    pub tools_used: Vec<String>,
    pub usage_count: u32,
    pub success_rate: f32,
    pub avg_duration_ms: u64,
    pub last_used: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalPattern {
    pub pattern_signature: String,
    pub frequency: u32,
    pub avg_effectiveness: f32,
    pub task_categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePatternDB {
    pub patterns: HashMap<String, Vec<UsagePattern>>,
    pub global_patterns: Vec<GlobalPattern>,
}

impl UsagePatternDB {
    pub fn new() -> Self {
        Self {
            patterns: HashMap::new(),
            global_patterns: Vec::new(),
        }
    }

    pub fn add_pattern(&mut self, user_id: &str, pattern: UsagePattern) {
        self.patterns
            .entry(user_id.to_string())
            .or_default()
            .push(pattern);
    }

    pub fn get_user_patterns(&self, user_id: &str) -> Vec<&UsagePattern> {
        self.patterns
            .get(user_id)
            .map(|p| p.iter().collect())
            .unwrap_or_default()
    }

    pub fn get_global_patterns(&self) -> &[GlobalPattern] {
        &self.global_patterns
    }

    pub fn add_global_pattern(&mut self, pattern: GlobalPattern) {
        self.global_patterns.push(pattern);
    }

    pub fn find_similar_patterns(&self, task_signature: &str) -> Vec<&UsagePattern> {
        let mut similar = Vec::new();
        let sig_lower = task_signature.to_lowercase();

        for patterns in self.patterns.values() {
            for pattern in patterns {
                if pattern.task_signature.to_lowercase().contains(&sig_lower)
                    || sig_lower.contains(&pattern.task_signature.to_lowercase())
                {
                    similar.push(pattern);
                }
            }
        }

        similar.sort_by(|a, b| {
            b.success_rate
                .partial_cmp(&a.success_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        similar
    }

    pub fn update_pattern_success(&mut self, pattern_id: &str, success: bool) {
        for patterns in self.patterns.values_mut() {
            if let Some(pattern) = patterns.iter_mut().find(|p| p.pattern_id == pattern_id) {
                let total = pattern.usage_count as f32;
                let current_success = pattern.success_rate * total;
                pattern.success_rate = if success {
                    (current_success + 1.0) / (total + 1.0)
                } else {
                    current_success / (total + 1.0)
                };
                pattern.usage_count += 1;
                pattern.last_used = Utc::now();
                break;
            }
        }
    }
}

impl Default for UsagePatternDB {
    fn default() -> Self {
        Self::new()
    }
}
