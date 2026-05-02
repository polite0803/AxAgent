use crate::tool_recommender::analyzer::{EntityType, TaskContext, TaskType};
use crate::tool_recommender::patterns::UsagePatternDB;
use crate::tool_recommender::{Tool, ToolId, ToolIndex};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolScore {
    pub tool_id: String,
    pub tool_name: String,
    pub score: f32,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeSet {
    pub description: String,
    pub tools: Vec<String>,
    pub tradeoffs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRecommendation {
    pub tools: Vec<ToolScore>,
    pub reasoning: String,
    pub confidence: f32,
    pub alternatives: Vec<AlternativeSet>,
}

pub struct ToolRecommender {
    pub tool_index: ToolIndex,
    pub usage_patterns: UsagePatternDB,
    pub similarity_model: SimilarityModel,
}

impl ToolRecommender {
    pub fn new() -> Self {
        let mut tool_index = ToolIndex::new();

        tool_index.tools.insert(
            ToolId("web_search".to_string()),
            Tool {
                id: ToolId("web_search".to_string()),
                name: "Web Search".to_string(),
                description: "Search the web for information".to_string(),
                categories: vec!["information_retrieval".to_string(), "web".to_string()],
            },
        );

        tool_index.tools.insert(
            ToolId("file_read".to_string()),
            Tool {
                id: ToolId("file_read".to_string()),
                name: "File Read".to_string(),
                description: "Read contents from a file".to_string(),
                categories: vec!["file_operation".to_string(), "io".to_string()],
            },
        );

        tool_index.tools.insert(
            ToolId("file_write".to_string()),
            Tool {
                id: ToolId("file_write".to_string()),
                name: "File Write".to_string(),
                description: "Write content to a file".to_string(),
                categories: vec!["file_operation".to_string(), "io".to_string()],
            },
        );

        tool_index.tools.insert(
            ToolId("code_executor".to_string()),
            Tool {
                id: ToolId("code_executor".to_string()),
                name: "Code Executor".to_string(),
                description: "Execute code in various languages".to_string(),
                categories: vec!["code_generation".to_string(), "execution".to_string()],
            },
        );

        tool_index.tools.insert(
            ToolId("browser".to_string()),
            Tool {
                id: ToolId("browser".to_string()),
                name: "Browser".to_string(),
                description: "Control a web browser".to_string(),
                categories: vec!["web_interaction".to_string(), "automation".to_string()],
            },
        );

        tool_index.tools.insert(
            ToolId("data_analysis".to_string()),
            Tool {
                id: ToolId("data_analysis".to_string()),
                name: "Data Analysis".to_string(),
                description: "Analyze and process data".to_string(),
                categories: vec!["data_analysis".to_string(), "processing".to_string()],
            },
        );

        for (id, tool) in &tool_index.tools {
            for category in &tool.categories {
                tool_index
                    .category_index
                    .entry(category.clone())
                    .or_default()
                    .push(id.clone());
            }
        }

        Self {
            tool_index,
            usage_patterns: UsagePatternDB::new(),
            similarity_model: SimilarityModel::new(),
        }
    }

    pub fn recommend(&self, context: &TaskContext) -> ToolRecommendation {
        let candidates = self.tool_index.search(&context.task_description);
        let scored = self.score_candidates(&candidates, context);
        let ranked = self.rank_tools(scored);
        let reasoning = self.generate_reasoning(&ranked, context);
        let confidence = self.calculate_confidence(&ranked);
        let alternatives = self.generate_alternatives(&ranked);

        ToolRecommendation {
            tools: ranked,
            reasoning,
            confidence,
            alternatives,
        }
    }

    fn score_candidates(&self, candidates: &[&Tool], context: &TaskContext) -> Vec<ToolScore> {
        candidates
            .iter()
            .map(|tool| {
                let relevance = self.calculate_relevance(tool, context);
                let efficiency = self.estimate_efficiency(tool, context);
                let compatibility = self.check_compatibility(tool, context);
                let score = relevance * 0.4 + efficiency * 0.3 + compatibility * 0.3;

                let mut reasons = Vec::new();
                if relevance > 0.7 {
                    reasons.push("High relevance to task".to_string());
                }
                if efficiency > 0.7 {
                    reasons.push("Efficient for this type of task".to_string());
                }
                if compatibility > 0.7 {
                    reasons.push("Compatible with your constraints".to_string());
                }

                ToolScore {
                    tool_id: tool.id.0.clone(),
                    tool_name: tool.name.clone(),
                    score,
                    reasons,
                }
            })
            .collect()
    }

    fn calculate_relevance(&self, tool: &Tool, context: &TaskContext) -> f32 {
        let mut score: f32 = 0.0;

        match context.task_type {
            TaskType::InformationRetrieval => {
                if tool
                    .categories
                    .contains(&"information_retrieval".to_string())
                {
                    score += 0.5;
                }
                if tool.categories.contains(&"web".to_string()) {
                    score += 0.3;
                }
            },
            TaskType::CodeGeneration => {
                if tool.categories.contains(&"code_generation".to_string()) {
                    score += 0.5;
                }
                if tool.categories.contains(&"execution".to_string()) {
                    score += 0.3;
                }
            },
            TaskType::DataAnalysis => {
                if tool.categories.contains(&"data_analysis".to_string()) {
                    score += 0.5;
                }
                if tool.categories.contains(&"processing".to_string()) {
                    score += 0.3;
                }
            },
            TaskType::FileOperation => {
                if tool.categories.contains(&"file_operation".to_string()) {
                    score += 0.6;
                }
            },
            TaskType::WebInteraction => {
                if tool.categories.contains(&"web_interaction".to_string()) {
                    score += 0.5;
                }
                if tool.categories.contains(&"automation".to_string()) {
                    score += 0.3;
                }
            },
            TaskType::ContentCreation | TaskType::ProblemSolving => {
                score += 0.3;
            },
        }

        for entity in &context.entities {
            match entity.entity_type {
                EntityType::Url if tool.categories.contains(&"web".to_string()) => {
                    score += 0.2;
                },
                EntityType::FilePath if tool.categories.contains(&"file_operation".to_string()) => {
                    score += 0.2;
                },
                EntityType::Language if tool.categories.contains(&"code_generation".to_string()) => {
                    score += 0.2;
                },
                _ => {},
            }
        }

        score.min(1.0_f32)
    }

    fn estimate_efficiency(&self, tool: &Tool, _context: &TaskContext) -> f32 {
        match tool.id.0.as_str() {
            "web_search" => 0.9,
            "file_read" => 0.95,
            "file_write" => 0.9,
            "code_executor" => 0.7,
            "browser" => 0.6,
            "data_analysis" => 0.75,
            _ => 0.5,
        }
    }

    fn check_compatibility(&self, tool: &Tool, context: &TaskContext) -> f32 {
        let mut score: f32 = 0.8;

        for constraint in &context.constraints {
            if constraint.constraint_type.as_str() == "speed" {
                if constraint.value == "fast" && tool.id.0 == "browser" {
                    score -= 0.3;
                }
            }
        }

        score.clamp(0.0_f32, 1.0_f32)
    }

    fn rank_tools(&self, mut scored: Vec<ToolScore>) -> Vec<ToolScore> {
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored
    }

    fn generate_reasoning(&self, ranked: &[ToolScore], context: &TaskContext) -> String {
        if ranked.is_empty() {
            return "No suitable tools found for this task.".to_string();
        }

        let top_tool = &ranked[0];
        let task_type_str = match context.task_type {
            TaskType::InformationRetrieval => "information retrieval",
            TaskType::CodeGeneration => "code generation",
            TaskType::DataAnalysis => "data analysis",
            TaskType::FileOperation => "file operation",
            TaskType::WebInteraction => "web interaction",
            TaskType::ContentCreation => "content creation",
            TaskType::ProblemSolving => "problem solving",
        };

        format!(
            "For {} tasks, '{}' is recommended with a confidence score of {:.1}%. {}",
            task_type_str,
            top_tool.tool_name,
            top_tool.score * 100.0,
            if top_tool.reasons.is_empty() {
                "This tool matches your task requirements.".to_string()
            } else {
                top_tool.reasons.join(". ")
            }
        )
    }

    fn calculate_confidence(&self, ranked: &[ToolScore]) -> f32 {
        if ranked.is_empty() {
            return 0.0;
        }

        let top_score = ranked[0].score;
        if ranked.len() > 1 {
            let second_score = ranked[1].score;
            let gap = top_score - second_score;
            top_score * (0.5 + gap * 0.5)
        } else {
            top_score
        }
    }

    fn generate_alternatives(&self, ranked: &[ToolScore]) -> Vec<AlternativeSet> {
        if ranked.len() < 2 {
            return Vec::new();
        }

        let mut alternatives = Vec::new();

        if ranked.len() >= 2 {
            alternatives.push(AlternativeSet {
                description: "Alternative approach".to_string(),
                tools: vec![ranked[1].tool_id.clone()],
                tradeoffs: vec![
                    "May have lower relevance score".to_string(),
                    "Consider if primary tool fails".to_string(),
                ],
            });
        }

        alternatives
    }
}

impl Default for ToolRecommender {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SimilarityModel;

impl SimilarityModel {
    fn new() -> Self {
        Self
    }

    #[allow(dead_code)]
    fn calculate_similarity(&self, text1: &str, text2: &str) -> f32 {
        let text1_lower = text1.to_lowercase();
        let text2_lower = text2.to_lowercase();
        let words1: std::collections::HashSet<_> = text1_lower.split_whitespace().collect();
        let words2: std::collections::HashSet<_> = text2_lower.split_whitespace().collect();

        if words1.is_empty() || words2.is_empty() {
            return 0.0;
        }

        let intersection = words1.intersection(&words2).count() as f32;
        let union = words1.union(&words2).count() as f32;

        intersection / union
    }
}
