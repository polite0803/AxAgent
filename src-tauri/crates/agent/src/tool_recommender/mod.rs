pub mod analyzer;
pub mod engine;
pub mod patterns;

pub use analyzer::{ContextAnalyzer, Entity, EntityType, TaskContext, TaskType};
pub use engine::{AlternativeSet, ToolRecommendation, ToolRecommender, ToolScore};
pub use patterns::{GlobalPattern, UsagePattern, UsagePatternDB};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ToolId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub id: ToolId,
    pub name: String,
    pub description: String,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolIndex {
    pub tools: HashMap<ToolId, Tool>,
    pub category_index: HashMap<String, Vec<ToolId>>,
}

impl ToolIndex {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            category_index: HashMap::new(),
        }
    }

    pub fn search(&self, query: &str) -> Vec<&Tool> {
        let query_lower = query.to_lowercase();
        self.tools
            .values()
            .filter(|tool| {
                tool.name.to_lowercase().contains(&query_lower)
                    || tool.description.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    pub fn get_by_category(&self, category: &str) -> Vec<&Tool> {
        self.category_index
            .get(category)
            .map(|ids| ids.iter().filter_map(|id| self.tools.get(id)).collect())
            .unwrap_or_default()
    }
}

impl Default for ToolIndex {
    fn default() -> Self {
        Self::new()
    }
}
