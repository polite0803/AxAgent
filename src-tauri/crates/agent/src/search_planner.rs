use crate::research_state::{SearchPlan, SearchQuery, SourceType};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPlannerConfig {
    pub max_queries_per_phase: usize,
    pub include_academic: bool,
    pub include_wikipedia: bool,
    pub include_github: bool,
}

impl Default for SearchPlannerConfig {
    fn default() -> Self {
        Self {
            max_queries_per_phase: 5,
            include_academic: true,
            include_wikipedia: true,
            include_github: false,
        }
    }
}

pub struct SearchPlanner {
    config: SearchPlannerConfig,
}

impl SearchPlanner {
    pub fn new() -> Self {
        Self {
            config: SearchPlannerConfig::default(),
        }
    }

    pub fn with_config(mut self, config: SearchPlannerConfig) -> Self {
        self.config = config;
        self
    }

    pub fn plan(&self, topic: &str) -> SearchPlan {
        let queries = self.generate_queries(topic);
        SearchPlan::new(queries)
    }

    pub fn plan_with_depth(&self, topic: &str, depth: ResearchDepth) -> SearchPlan {
        let queries = self.generate_queries_with_depth(topic, depth);
        let parallel_groups = self.create_parallel_groups(&queries);

        SearchPlan::new(queries).with_parallel_groups(parallel_groups)
    }

    fn generate_queries(&self, topic: &str) -> Vec<SearchQuery> {
        let base_queries = self.generate_base_queries(topic);
        let aspect_queries = self.generate_aspect_queries(topic);
        let source_queries = self.generate_source_specific_queries(topic);

        let mut all_queries: Vec<SearchQuery> = Vec::new();
        all_queries.extend(base_queries);
        all_queries.extend(aspect_queries);
        all_queries.extend(source_queries);

        all_queries.truncate(self.config.max_queries_per_phase * 3);
        all_queries
    }

    fn generate_queries_with_depth(&self, topic: &str, depth: ResearchDepth) -> Vec<SearchQuery> {
        let mut queries = Vec::new();

        queries.push(
            SearchQuery::new(format!("\"{}\"", topic))
                .with_sources(vec![SourceType::Wikipedia, SourceType::Web])
                .with_max_results(5),
        );

        match depth {
            ResearchDepth::Surface => {
                queries.extend(self.generate_surface_queries(topic));
            },
            ResearchDepth::Standard => {
                queries.extend(self.generate_base_queries(topic));
                queries.extend(self.generate_aspect_queries(topic));
            },
            ResearchDepth::Deep => {
                queries.extend(self.generate_base_queries(topic));
                queries.extend(self.generate_aspect_queries(topic));
                queries.extend(self.generate_source_specific_queries(topic));
                queries.extend(self.generate_comparative_queries(topic));
            },
        }

        queries.truncate(self.config.max_queries_per_phase * 4);
        queries
    }

    fn generate_base_queries(&self, topic: &str) -> Vec<SearchQuery> {
        vec![
            SearchQuery::new(topic.to_string())
                .with_sources(vec![SourceType::Web])
                .with_max_results(10),
            SearchQuery::new(format!("{} definition", topic))
                .with_sources(vec![SourceType::Wikipedia])
                .with_max_results(3),
            SearchQuery::new(format!("{} overview", topic))
                .with_sources(vec![SourceType::Web, SourceType::Documentation])
                .with_max_results(5),
        ]
    }

    fn generate_aspect_queries(&self, topic: &str) -> Vec<SearchQuery> {
        let aspects = [
            "history",
            "development",
            "applications",
            "advantages",
            "disadvantages",
            "challenges",
            "future",
            "trends",
        ];

        aspects
            .iter()
            .map(|aspect| {
                SearchQuery::new(format!("{} {}", topic, aspect))
                    .with_sources(vec![SourceType::Web])
                    .with_max_results(5)
            })
            .collect()
    }

    fn generate_source_specific_queries(&self, topic: &str) -> Vec<SearchQuery> {
        let mut queries = Vec::new();

        if self.config.include_academic {
            queries.push(
                SearchQuery::new(format!("{} research paper", topic))
                    .with_sources(vec![SourceType::Academic])
                    .with_max_results(5),
            );
        }

        if self.config.include_wikipedia {
            queries.push(
                SearchQuery::new(topic.to_string())
                    .with_sources(vec![SourceType::Wikipedia])
                    .with_max_results(3),
            );
        }

        if self.config.include_github {
            queries.push(
                SearchQuery::new(format!("{} repository", topic))
                    .with_sources(vec![SourceType::GitHub])
                    .with_max_results(5),
            );
        }

        queries
    }

    fn generate_comparative_queries(&self, topic: &str) -> Vec<SearchQuery> {
        vec![
            SearchQuery::new(format!("{} vs alternative", topic))
                .with_sources(vec![SourceType::Web])
                .with_max_results(5),
            SearchQuery::new(format!("{} comparison", topic))
                .with_sources(vec![SourceType::Web])
                .with_max_results(5),
        ]
    }

    fn generate_surface_queries(&self, topic: &str) -> Vec<SearchQuery> {
        vec![SearchQuery::new(topic.to_string())
            .with_sources(vec![SourceType::Wikipedia, SourceType::Web])
            .with_max_results(3)]
    }

    fn create_parallel_groups(&self, queries: &[SearchQuery]) -> Vec<Vec<String>> {
        let mut groups: Vec<Vec<String>> = Vec::new();
        let mut current_group: Vec<String> = Vec::new();
        let mut seen_sources: HashSet<SourceType> = HashSet::new();

        for query in queries {
            let has_new_source = query.source_types.iter().any(|s| !seen_sources.contains(s));

            if has_new_source && current_group.len() >= 2 {
                groups.push(current_group.clone());
                current_group.clear();
                seen_sources.clear();
            }

            current_group.push(query.id.clone());
            for source in &query.source_types {
                seen_sources.insert(*source);
            }
        }

        if !current_group.is_empty() {
            groups.push(current_group);
        }

        groups
    }

    pub fn refine_plan(&self, plan: &SearchPlan, feedback: &str) -> SearchPlan {
        let refined_queries = self.refine_queries(&plan.queries, feedback);
        let parallel_groups = self.create_parallel_groups(&refined_queries);

        SearchPlan::new(refined_queries).with_parallel_groups(parallel_groups)
    }

    fn refine_queries(&self, queries: &[SearchQuery], feedback: &str) -> Vec<SearchQuery> {
        let feedback_lower = feedback.to_lowercase();

        let mut refined: Vec<SearchQuery> = queries
            .iter()
            .filter(|q| {
                !feedback_lower.contains("remove")
                    || !feedback_lower.contains(&q.query.to_lowercase())
            })
            .cloned()
            .collect();

        if feedback_lower.contains("more academic") || feedback_lower.contains("学术") {
            refined.push(
                SearchQuery::new(format!(
                    "{} research paper",
                    queries[0].query.split(' ').collect::<Vec<_>>().join(" ")
                ))
                .with_sources(vec![SourceType::Academic])
                .with_max_results(10),
            );
        }

        if feedback_lower.contains("more technical") || feedback_lower.contains("技术") {
            refined.push(
                SearchQuery::new(format!("{} technical documentation", queries[0].query))
                    .with_sources(vec![SourceType::Documentation, SourceType::GitHub])
                    .with_max_results(5),
            );
        }

        refined
    }
}

impl Default for SearchPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResearchDepth {
    Surface,
    Standard,
    Deep,
}

impl ResearchDepth {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResearchDepth::Surface => "surface",
            ResearchDepth::Standard => "standard",
            ResearchDepth::Deep => "deep",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ResearchDepth::Surface => "浅层",
            ResearchDepth::Standard => "标准",
            ResearchDepth::Deep => "深度",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_planning() {
        let planner = SearchPlanner::new();
        let plan = planner.plan("Rust programming language");

        assert!(!plan.queries.is_empty());
        assert!(!plan.parallel_groups.is_empty());
    }

    #[test]
    fn test_depth_based_planning() {
        let planner = SearchPlanner::new();

        let surface_plan = planner.plan_with_depth("AI", ResearchDepth::Surface);
        let deep_plan = planner.plan_with_depth("AI", ResearchDepth::Deep);

        assert!(surface_plan.queries.len() <= deep_plan.queries.len());
    }

    #[test]
    fn test_parallel_groups() {
        let planner = SearchPlanner::new();
        let plan = planner.plan("machine learning");

        for group in &plan.parallel_groups {
            assert!(!group.is_empty());
        }
    }
}
