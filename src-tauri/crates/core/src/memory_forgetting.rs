use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub namespace: String,
    pub importance: f64,
    pub memory_strength: f64,
    pub created_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
    pub access_count: u32,
    pub tags: Vec<String>,
    pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgettingConfig {
    pub decay_rate: f64,
    pub minimum_strength: f64,
    pub importance_weight: f64,
    pub recency_weight: f64,
    pub frequency_weight: f64,
    pub max_memories_per_namespace: usize,
    pub reinforcement_factor: f64,
}

impl Default for ForgettingConfig {
    fn default() -> Self {
        Self {
            decay_rate: 0.3,
            minimum_strength: 0.1,
            importance_weight: 0.4,
            recency_weight: 0.3,
            frequency_weight: 0.3,
            max_memories_per_namespace: 1000,
            reinforcement_factor: 0.2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgettingStats {
    pub total_evaluated: usize,
    pub forgotten: usize,
    pub retained: usize,
    pub reinforced: usize,
    pub average_strength: f64,
}

pub struct MemoryForgettingEngine {
    config: ForgettingConfig,
}

impl MemoryForgettingEngine {
    pub fn new(config: ForgettingConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self::new(ForgettingConfig::default())
    }

    pub fn calculate_strength(&self, entry: &MemoryEntry) -> f64 {
        let now = Utc::now();
        let elapsed_hours = (now - entry.last_accessed_at).num_seconds() as f64 / 3600.0;

        let stability =
            1.0 + entry.importance * 10.0 + (entry.access_count as f64).ln().max(0.0) * 2.0;

        let retention = (-self.config.decay_rate * elapsed_hours / stability).exp();

        let recency_score = retention;
        let importance_score = entry.importance;
        let frequency_score = (entry.access_count as f64).ln_1p() / 10.0;

        let combined = self.config.recency_weight * recency_score
            + self.config.importance_weight * importance_score
            + self.config.frequency_weight * frequency_score.min(1.0);

        combined.clamp(0.0, 1.0)
    }

    pub fn reinforce(&self, entry: &mut MemoryEntry) {
        entry.memory_strength = (entry.memory_strength + self.config.reinforcement_factor).min(1.0);
        entry.last_accessed_at = Utc::now();
        entry.access_count += 1;
    }

    pub fn evaluate_forgetting(&self, entries: &[MemoryEntry]) -> (Vec<String>, Vec<MemoryEntry>) {
        let mut to_forget = Vec::new();
        let mut to_retain = Vec::new();

        for entry in entries {
            let strength = self.calculate_strength(entry);
            if strength < self.config.minimum_strength {
                to_forget.push(entry.id.clone());
            } else {
                let mut updated = entry.clone();
                updated.memory_strength = strength;
                to_retain.push(updated);
            }
        }

        (to_forget, to_retain)
    }

    pub fn prune_namespace(&self, entries: &mut Vec<MemoryEntry>) -> Vec<String> {
        if entries.len() <= self.config.max_memories_per_namespace {
            return Vec::new();
        }

        for entry in entries.iter_mut() {
            entry.memory_strength = self.calculate_strength(entry);
        }

        entries.sort_by(|a, b| {
            b.memory_strength
                .partial_cmp(&a.memory_strength)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let to_remove_count = entries.len() - self.config.max_memories_per_namespace;
        let forgotten: Vec<String> = entries
            .iter()
            .rev()
            .take(to_remove_count)
            .map(|e| e.id.clone())
            .collect();

        entries.truncate(self.config.max_memories_per_namespace);
        forgotten
    }

    pub fn get_stats(&self, entries: &[MemoryEntry]) -> ForgettingStats {
        let total = entries.len();
        let (to_forget, retained) = self.evaluate_forgetting(entries);
        let avg_strength = if total > 0 {
            entries
                .iter()
                .map(|e| self.calculate_strength(e))
                .sum::<f64>()
                / total as f64
        } else {
            0.0
        };

        ForgettingStats {
            total_evaluated: total,
            forgotten: to_forget.len(),
            retained: retained.len(),
            reinforced: 0,
            average_strength: avg_strength,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(id: &str, importance: f64, hours_ago: i64, access_count: u32) -> MemoryEntry {
        MemoryEntry {
            id: id.to_string(),
            content: format!("Memory {}", id),
            namespace: "test".to_string(),
            importance,
            memory_strength: 1.0,
            created_at: Utc::now() - Duration::hours(hours_ago),
            last_accessed_at: Utc::now() - Duration::hours(hours_ago),
            access_count,
            tags: vec![],
            embedding: None,
        }
    }

    #[test]
    fn test_recent_important_memory_retained() {
        let engine = MemoryForgettingEngine::with_default_config();
        let entry = make_entry("1", 0.9, 1, 5);
        let strength = engine.calculate_strength(&entry);
        assert!(
            strength > 0.5,
            "Recent important memory should have high strength: {}",
            strength
        );
    }

    #[test]
    fn test_old_unimportant_memory_forgotten() {
        let engine = MemoryForgettingEngine::with_default_config();
        let entry = make_entry("1", 0.1, 720, 0);
        let strength = engine.calculate_strength(&entry);
        assert!(strength < 0.2, "Old unimportant memory should have low strength: {}", strength);
    }

    #[test]
    fn test_reinforcement_increases_strength() {
        let engine = MemoryForgettingEngine::with_default_config();
        let mut entry = make_entry("1", 0.5, 24, 1);
        let before = entry.memory_strength;
        engine.reinforce(&mut entry);
        assert!(entry.memory_strength > before);
        assert_eq!(entry.access_count, 2);
    }

    #[test]
    fn test_evaluate_forgetting() {
        let engine = MemoryForgettingEngine::with_default_config();
        let entries = vec![
            make_entry("1", 0.9, 1, 5),
            make_entry("2", 0.1, 720, 0),
            make_entry("3", 0.5, 24, 2),
        ];
        let (to_forget, to_retain) = engine.evaluate_forgetting(&entries);
        assert!(to_forget.contains(&"2".to_string()));
        assert_eq!(to_retain.len(), 2);
    }

    #[test]
    fn test_prune_namespace() {
        let config = ForgettingConfig {
            max_memories_per_namespace: 2,
            ..Default::default()
        };
        let engine = MemoryForgettingEngine::new(config);
        let mut entries = vec![
            make_entry("1", 0.9, 1, 5),
            make_entry("2", 0.1, 720, 0),
            make_entry("3", 0.5, 24, 2),
        ];
        let forgotten = engine.prune_namespace(&mut entries);
        assert_eq!(forgotten.len(), 1);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_get_stats() {
        let engine = MemoryForgettingEngine::with_default_config();
        let entries = vec![make_entry("1", 0.9, 1, 5), make_entry("2", 0.1, 720, 0)];
        let stats = engine.get_stats(&entries);
        assert_eq!(stats.total_evaluated, 2);
        assert!(stats.average_strength > 0.0);
    }
}
