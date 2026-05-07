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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pattern(id: &str, signature: &str, success_rate: f32) -> UsagePattern {
        UsagePattern {
            pattern_id: id.to_string(),
            task_signature: signature.to_string(),
            tools_used: vec!["tool_a".to_string()],
            usage_count: 1,
            success_rate,
            avg_duration_ms: 100,
            last_used: Utc::now(),
        }
    }

    #[test]
    fn test_usage_pattern_db_new() {
        let db = UsagePatternDB::new();
        assert!(db.patterns.is_empty());
        assert!(db.global_patterns.is_empty());
    }

    #[test]
    fn test_usage_pattern_db_add_pattern() {
        let mut db = UsagePatternDB::new();
        let pattern = make_pattern("p1", "code_generation", 0.8);
        db.add_pattern("user1", pattern);
        assert_eq!(db.patterns.len(), 1);
        assert!(db.patterns.contains_key("user1"));
    }

    #[test]
    fn test_usage_pattern_db_add_multiple_users() {
        let mut db = UsagePatternDB::new();
        db.add_pattern("user1", make_pattern("p1", "sig1", 0.8));
        db.add_pattern("user2", make_pattern("p2", "sig2", 0.9));
        assert_eq!(db.patterns.len(), 2);
    }

    #[test]
    fn test_usage_pattern_db_add_multiple_patterns_per_user() {
        let mut db = UsagePatternDB::new();
        db.add_pattern("user1", make_pattern("p1", "sig1", 0.8));
        db.add_pattern("user1", make_pattern("p2", "sig2", 0.9));
        assert_eq!(db.patterns.len(), 1);
        assert_eq!(db.get_user_patterns("user1").len(), 2);
    }

    #[test]
    fn test_usage_pattern_db_get_user_patterns() {
        let mut db = UsagePatternDB::new();
        db.add_pattern("user1", make_pattern("p1", "sig1", 0.8));
        let patterns = db.get_user_patterns("user1");
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].pattern_id, "p1");
    }

    #[test]
    fn test_usage_pattern_db_get_user_patterns_nonexistent() {
        let db = UsagePatternDB::new();
        let patterns = db.get_user_patterns("nonexistent");
        assert!(patterns.is_empty());
    }

    #[test]
    fn test_usage_pattern_db_find_similar_patterns() {
        let mut db = UsagePatternDB::new();
        db.add_pattern("user1", make_pattern("p1", "code_generation", 0.8));
        db.add_pattern("user2", make_pattern("p2", "data_analysis", 0.9));
        let similar = db.find_similar_patterns("code");
        assert!(!similar.is_empty());
    }

    #[test]
    fn test_usage_pattern_db_find_similar_patterns_sorted_by_success() {
        let mut db = UsagePatternDB::new();
        db.add_pattern("user1", make_pattern("p1", "code_generation", 0.6));
        db.add_pattern("user2", make_pattern("p2", "code_refactoring", 0.9));
        let similar = db.find_similar_patterns("code");
        assert!(similar.len() >= 2);
        assert!(similar[0].success_rate >= similar[1].success_rate);
    }

    #[test]
    fn test_usage_pattern_db_find_similar_patterns_no_match() {
        let mut db = UsagePatternDB::new();
        db.add_pattern("user1", make_pattern("p1", "code_generation", 0.8));
        let similar = db.find_similar_patterns("web_browsing");
        assert!(similar.is_empty());
    }

    #[test]
    fn test_usage_pattern_db_update_pattern_success() {
        let mut db = UsagePatternDB::new();
        db.add_pattern("user1", make_pattern("p1", "sig1", 1.0));
        db.update_pattern_success("p1", true);
        let patterns = db.get_user_patterns("user1");
        assert_eq!(patterns[0].usage_count, 2);
    }

    #[test]
    fn test_usage_pattern_db_update_pattern_failure() {
        let mut db = UsagePatternDB::new();
        db.add_pattern("user1", make_pattern("p1", "sig1", 1.0));
        db.update_pattern_success("p1", false);
        let patterns = db.get_user_patterns("user1");
        assert_eq!(patterns[0].usage_count, 2);
        assert!(patterns[0].success_rate < 1.0);
    }

    #[test]
    fn test_usage_pattern_db_update_nonexistent() {
        let mut db = UsagePatternDB::new();
        db.update_pattern_success("nonexistent", true);
        assert!(db.patterns.is_empty());
    }

    #[test]
    fn test_usage_pattern_db_add_global_pattern() {
        let mut db = UsagePatternDB::new();
        let gp = GlobalPattern {
            pattern_signature: "code_gen_pattern".to_string(),
            frequency: 10,
            avg_effectiveness: 0.85,
            task_categories: vec!["code".to_string()],
        };
        db.add_global_pattern(gp);
        assert_eq!(db.get_global_patterns().len(), 1);
    }

    #[test]
    fn test_usage_pattern_db_default() {
        let db = UsagePatternDB::default();
        assert!(db.patterns.is_empty());
    }

    #[test]
    fn test_usage_pattern_serialization() {
        let pattern = make_pattern("p1", "sig1", 0.85);
        let json = serde_json::to_string(&pattern).unwrap();
        let de: UsagePattern = serde_json::from_str(&json).unwrap();
        assert_eq!(de.pattern_id, "p1");
        assert!((de.success_rate - 0.85).abs() < 0.001);
    }

    #[test]
    fn test_global_pattern_serialization() {
        let gp = GlobalPattern {
            pattern_signature: "test_sig".to_string(),
            frequency: 5,
            avg_effectiveness: 0.75,
            task_categories: vec!["cat1".to_string()],
        };
        let json = serde_json::to_string(&gp).unwrap();
        let de: GlobalPattern = serde_json::from_str(&json).unwrap();
        assert_eq!(de.frequency, 5);
    }
}
