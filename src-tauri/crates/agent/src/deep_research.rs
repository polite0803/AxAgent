use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use axagent_harness::types::{ChatContent, ChatMessage, ChatRequest};
use axagent_harness::{ProviderAdapter, ProviderRequestContext};
use serde::{Deserialize, Serialize};

use crate::ingest_pipeline::{IngestPipeline, IngestSourceType};
use crate::search_provider::SearchProvider;
use crate::web_search::WebSearchProvider;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResearchPhase {
    InitialSearch,
    Analysis,
    GapIdentification,
    DeepeningSearch,
    Synthesis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchRound {
    pub round_number: usize,
    pub queries: Vec<ResearchQuery>,
    pub findings: Vec<ResearchFinding>,
    pub identified_gaps: Vec<String>,
    pub coverage_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorroboratedFinding {
    pub finding_summary: String,
    pub source_urls: Vec<String>,
    pub corroboration_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contradiction {
    pub topic: String,
    pub source_a_url: String,
    pub source_a_claim: String,
    pub source_b_url: String,
    pub source_b_claim: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepResearchConfig {
    pub max_queries: usize,
    pub max_results_per_query: usize,
    pub concurrent_searches: usize,
    pub max_rounds: usize,
    pub coverage_threshold: f32,
    pub enable_gap_analysis: bool,
}

impl Default for DeepResearchConfig {
    fn default() -> Self {
        Self {
            max_queries: 5,
            max_results_per_query: 5,
            concurrent_searches: 3,
            max_rounds: 3,
            coverage_threshold: 0.7,
            enable_gap_analysis: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchQuery {
    pub query: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub query: String,
    pub url: String,
    pub title: String,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchFinding {
    pub query: String,
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepResearchResult {
    pub topic: String,
    pub queries_generated: Vec<ResearchQuery>,
    pub findings: Vec<ResearchFinding>,
    pub pages_created: Vec<String>,
    pub rounds: Vec<ResearchRound>,
    pub total_coverage_score: f32,
    pub corroborated_findings: Vec<CorroboratedFinding>,
    pub contradictions: Vec<Contradiction>,
}

pub struct DeepResearcher {
    config: DeepResearchConfig,
    search_provider: Arc<WebSearchProvider>,
    ingest_pipeline: Arc<IngestPipeline>,
}

impl DeepResearcher {
    pub fn new(
        config: DeepResearchConfig,
        search_provider: Arc<WebSearchProvider>,
        ingest_pipeline: Arc<IngestPipeline>,
    ) -> Self {
        Self {
            config,
            search_provider,
            ingest_pipeline,
        }
    }

    pub async fn research(
        &self,
        wiki_id: &str,
        topic: &str,
        overview_content: Option<&str>,
        llm_adapter: Option<Arc<dyn ProviderAdapter>>,
        llm_ctx: Option<ProviderRequestContext>,
        llm_model: Option<&str>,
    ) -> Result<DeepResearchResult, String> {
        let context = self.build_context(overview_content, topic);
        let mut rounds = Vec::new();
        let mut all_queries = Vec::new();
        let mut all_findings = Vec::new();
        let mut seen_urls: HashSet<String> = HashSet::new();
        let mut current_gaps: Vec<String> = Vec::new();
        let mut total_coverage_score = 0.0_f32;

        for round_num in 1..=self.config.max_rounds {
            let queries = if round_num == 1 {
                if let (Some(adapter), Some(ctx), Some(model)) =
                    (&llm_adapter, &llm_ctx, &llm_model)
                {
                    self.generate_queries(topic, &context, adapter.as_ref(), ctx, model)
                        .await?
                } else {
                    self.default_queries(topic)
                }
            } else {
                if current_gaps.is_empty() {
                    break;
                }
                self.generate_gap_queries(
                    topic,
                    &current_gaps,
                    llm_adapter.as_deref(),
                    llm_ctx.as_ref(),
                    llm_model,
                )
                .await?
            };

            all_queries.extend(queries.clone());

            let findings = self.execute_searches(&queries, &mut seen_urls).await;

            all_findings.extend(findings.clone());

            let (coverage_score, gaps) = self.analyze_coverage(topic, &all_findings, &current_gaps);
            current_gaps = gaps.clone();
            total_coverage_score = coverage_score;

            rounds.push(ResearchRound {
                round_number: round_num,
                queries,
                findings: findings.clone(),
                identified_gaps: gaps,
                coverage_score,
            });

            if coverage_score >= self.config.coverage_threshold {
                break;
            }

            if !self.config.enable_gap_analysis {
                break;
            }

            if current_gaps.is_empty() {
                break;
            }
        }

        let mut pages_created = Vec::new();
        for finding in &all_findings {
            for result in &finding.results {
                let page_result = self.ingest_result(wiki_id, result).await;
                if let Ok(page_id) = page_result {
                    pages_created.push(page_id);
                }
            }
        }

        let corroborated_findings = self.cross_validate_corroboration(&all_findings);
        let contradictions = self.cross_validate_contradictions(&all_findings);

        Ok(DeepResearchResult {
            topic: topic.to_string(),
            queries_generated: all_queries,
            findings: all_findings,
            pages_created,
            rounds,
            total_coverage_score,
            corroborated_findings,
            contradictions,
        })
    }

    fn build_context(&self, overview: Option<&str>, topic: &str) -> String {
        let mut context = String::new();

        if let Some(overview_content) = overview {
            context.push_str("## Wiki Overview\n");
            context.push_str(overview_content);
            context.push_str("\n\n");
        }

        context.push_str("## Research Topic\n");
        context.push_str(topic);

        context
    }

    async fn generate_queries(
        &self,
        topic: &str,
        context: &str,
        adapter: &dyn ProviderAdapter,
        ctx: &ProviderRequestContext,
        model: &str,
    ) -> Result<Vec<ResearchQuery>, String> {
        let prompt = format!(
            r#"Based on the following wiki context and research topic, generate {} effective search queries to explore this topic deeply.

## Context
{context}

## Research Topic
{topic}

## Requirements
1. Generate diverse queries covering different aspects of the topic
2. Include both broad exploratory queries and specific fact-finding queries
3. Vary the search strategy (definitions, comparisons, recent developments, controversies, etc.)
4. Each query should be self-contained and clear

Output JSON array of {{"query": "...", "rationale": "..."}}:
"#,
            self.config.max_queries
        );

        let request = ChatRequest {
            model: model.to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(prompt),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            }],
            stream: false,
            temperature: Some(0.7),
            max_tokens: Some(2048),
            top_p: None,
            tools: None,
            thinking_budget: None,
            use_max_completion_tokens: None,
            thinking_param_style: None,
            api_mode: None,
            instructions: None,
            conversation: None,
            previous_response_id: None,
            store: None,
        };

        let response = adapter
            .chat(ctx, request)
            .await
            .map_err(|e| format!("LLM query generation failed: {}", e))?;

        self.parse_queries(&response.content)
    }

    fn parse_queries(&self, response: &str) -> Result<Vec<ResearchQuery>, String> {
        let json_str = self.extract_json(response)?;

        let parsed: Vec<QueryRaw> = serde_json::from_str(&json_str)
            .map_err(|e| format!("Failed to parse queries JSON: {} - Raw: {}", e, json_str))?;

        Ok(parsed
            .into_iter()
            .map(|q| ResearchQuery {
                query: q.query,
                rationale: q.rationale,
            })
            .collect())
    }

    fn extract_json(&self, text: &str) -> Result<String, String> {
        if let Some(start) = text.find('[')
            && let Some(end) = text.rfind(']')
        {
            return Ok(text[start..=end].to_string());
        }
        if let Some(start) = text.find('{')
            && let Some(end) = text.rfind('}')
        {
            return Ok(text[start..=end].to_string());
        }
        Err("No JSON array or object found in response".to_string())
    }

    fn default_queries(&self, topic: &str) -> Vec<ResearchQuery> {
        vec![
            ResearchQuery {
                query: format!("\"{}\"", topic),
                rationale: "Primary search for the main topic".to_string(),
            },
            ResearchQuery {
                query: format!("\"{}\" definition", topic),
                rationale: "Find fundamental definitions and explanations".to_string(),
            },
            ResearchQuery {
                query: format!("\"{}\" recent developments 2024 2025", topic),
                rationale: "Latest news and developments".to_string(),
            },
            ResearchQuery {
                query: format!("\"{}\" controversy debate", topic),
                rationale: "Explore different perspectives and debates".to_string(),
            },
            ResearchQuery {
                query: format!("\"{}\" examples applications", topic),
                rationale: "Practical applications and examples".to_string(),
            },
        ]
    }

    async fn execute_searches(
        &self,
        queries: &[ResearchQuery],
        seen_urls: &mut HashSet<String>,
    ) -> Vec<ResearchFinding> {
        let mut handles = Vec::new();
        let max_results = self.config.max_results_per_query;

        for q in queries.iter().take(self.config.max_queries) {
            let query = q.query.clone();
            let provider = Arc::clone(&self.search_provider);

            let handle = tokio::spawn(async move {
                let search_query = crate::research_state::SearchQuery::new(query.clone())
                    .with_max_results(max_results);

                match provider.search(&search_query).await {
                    Ok(results) => {
                        let search_results: Vec<SearchResult> = results
                            .into_iter()
                            .map(|r| SearchResult {
                                query: query.clone(),
                                url: r.url,
                                title: r.title,
                                snippet: r.snippet,
                            })
                            .collect();
                        ResearchFinding {
                            query,
                            results: search_results,
                        }
                    },
                    Err(_) => ResearchFinding {
                        query,
                        results: Vec::new(),
                    },
                }
            });

            handles.push(handle);
        }

        let mut findings = Vec::new();
        for handle in handles {
            if let Ok(mut finding) = handle.await {
                finding.results.retain(|r| !seen_urls.contains(&r.url));
                for r in &finding.results {
                    seen_urls.insert(r.url.clone());
                }
                findings.push(finding);
            }
        }

        findings
    }

    fn analyze_coverage(
        &self,
        topic: &str,
        all_findings: &[ResearchFinding],
        _previous_gaps: &[String],
    ) -> (f32, Vec<String>) {
        let total_results: usize = all_findings.iter().map(|f| f.results.len()).sum();
        let unique_urls: HashSet<&str> = all_findings
            .iter()
            .flat_map(|f| f.results.iter())
            .map(|r| r.url.as_str())
            .collect();

        let unique_query_count = all_findings.len();
        let url_diversity = unique_urls.len() as f32;

        let base_coverage = if total_results == 0 {
            0.0
        } else {
            let density = (url_diversity / (total_results as f32)).min(1.0);
            let breadth = (unique_query_count as f32 / self.config.max_queries as f32).min(1.0);
            (density * 0.4 + breadth * 0.6).min(1.0)
        };

        let topic_keywords: Vec<&str> = topic.split_whitespace().collect();
        let mut covered_aspects: HashMap<&str, usize> = HashMap::new();
        for keyword in &topic_keywords {
            covered_aspects.insert(keyword, 0);
        }

        for finding in all_findings {
            for result in &finding.results {
                let lower_snippet = result.snippet.to_lowercase();
                let lower_title = result.title.to_lowercase();
                let combined = format!("{} {}", lower_title, lower_snippet);

                for (keyword, count) in covered_aspects.iter_mut() {
                    if combined.contains(keyword) {
                        *count += 1;
                    }
                }
            }
        }

        let covered_count = covered_aspects.values().filter(|&&c| c > 0).count();
        let keyword_coverage = if topic_keywords.is_empty() {
            1.0
        } else {
            covered_count as f32 / topic_keywords.len() as f32
        };

        let coverage_score = (base_coverage * 0.5 + keyword_coverage * 0.5).min(1.0);

        let mut gaps = Vec::new();

        let aspect_categories = [
            ("definition", vec!["definition", "meaning", "what is", "explained"]),
            ("history", vec!["history", "origin", "background", "timeline"]),
            ("current state", vec!["recent", "current", "latest", "2024", "2025", "today"]),
            (
                "applications",
                vec![
                    "application",
                    "use case",
                    "example",
                    "practice",
                    "implementation",
                ],
            ),
            ("challenges", vec!["challenge", "problem", "limitation", "issue", "controversy"]),
        ];

        let all_text: String = all_findings
            .iter()
            .flat_map(|f| f.results.iter())
            .map(|r| format!("{} {}", r.title, r.snippet).to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");

        for (category, keywords) in &aspect_categories {
            let found = keywords.iter().any(|k| all_text.contains(k));
            if !found {
                gaps.push(format!("{} of {}", category, topic));
            }
        }

        for (keyword, count) in &covered_aspects {
            if *count == 0 {
                gaps.push(format!(
                    "detailed information about \"{}\" in relation to {}",
                    keyword, topic
                ));
            }
        }

        (coverage_score, gaps)
    }

    async fn generate_gap_queries(
        &self,
        topic: &str,
        gaps: &[String],
        llm_adapter: Option<&dyn ProviderAdapter>,
        llm_ctx: Option<&ProviderRequestContext>,
        llm_model: Option<&str>,
    ) -> Result<Vec<ResearchQuery>, String> {
        if let (Some(adapter), Some(ctx), Some(model)) = (llm_adapter, llm_ctx, llm_model) {
            let gaps_text = gaps
                .iter()
                .enumerate()
                .map(|(i, g)| format!("{}. {}", i + 1, g))
                .collect::<Vec<_>>()
                .join("\n");

            let prompt = format!(
                r#"The following knowledge gaps have been identified for the research topic "{}":

{}

Generate up to {} targeted search queries to fill these gaps. Each query should focus on a specific gap.

Output JSON array of {{"query": "...", "rationale": "..."}}:
"#,
                topic, gaps_text, self.config.max_queries
            );

            let request = ChatRequest {
                model: model.to_string(),
                messages: vec![ChatMessage {
                    role: "user".to_string(),
                    content: ChatContent::Text(prompt),
                    tool_calls: None,
                    tool_call_id: None,
                    thinking: None,
                }],
                stream: false,
                temperature: Some(0.7),
                max_tokens: Some(2048),
                top_p: None,
                tools: None,
                thinking_budget: None,
                use_max_completion_tokens: None,
                thinking_param_style: None,
                api_mode: None,
                instructions: None,
                conversation: None,
                previous_response_id: None,
                store: None,
            };

            let response = adapter
                .chat(ctx, request)
                .await
                .map_err(|e| format!("LLM gap query generation failed: {}", e))?;

            self.parse_queries(&response.content)
        } else {
            let queries: Vec<ResearchQuery> = gaps
                .iter()
                .take(self.config.max_queries)
                .map(|gap| ResearchQuery {
                    query: format!("{} {}", topic, gap),
                    rationale: format!("Filling knowledge gap: {}", gap),
                })
                .collect();

            Ok(if queries.is_empty() {
                self.default_gap_queries(topic, gaps)
            } else {
                queries
            })
        }
    }

    fn default_gap_queries(&self, topic: &str, gaps: &[String]) -> Vec<ResearchQuery> {
        let mut queries = Vec::new();

        if gaps.iter().any(|g| g.contains("definition")) {
            queries.push(ResearchQuery {
                query: format!("\"{}\" comprehensive overview guide", topic),
                rationale: "Broad overview to fill definition gap".to_string(),
            });
        }

        if gaps.iter().any(|g| g.contains("history")) {
            queries.push(ResearchQuery {
                query: format!("\"{}\" history origin evolution", topic),
                rationale: "Historical background to fill history gap".to_string(),
            });
        }

        if gaps.iter().any(|g| g.contains("current")) {
            queries.push(ResearchQuery {
                query: format!("\"{}\" latest news updates 2025", topic),
                rationale: "Current state information to fill recency gap".to_string(),
            });
        }

        if gaps.iter().any(|g| g.contains("application")) {
            queries.push(ResearchQuery {
                query: format!("\"{}\" use cases real world examples", topic),
                rationale: "Practical examples to fill applications gap".to_string(),
            });
        }

        if gaps.iter().any(|g| g.contains("challenge")) {
            queries.push(ResearchQuery {
                query: format!("\"{}\" problems limitations criticism", topic),
                rationale: "Critical analysis to fill challenges gap".to_string(),
            });
        }

        if queries.is_empty() {
            queries.push(ResearchQuery {
                query: format!("\"{}\" in depth analysis", topic),
                rationale: "Deep analysis to fill general gaps".to_string(),
            });
        }

        queries.truncate(self.config.max_queries);
        queries
    }

    fn cross_validate_corroboration(
        &self,
        findings: &[ResearchFinding],
    ) -> Vec<CorroboratedFinding> {
        let mut snippet_groups: HashMap<String, Vec<String>> = HashMap::new();

        for finding in findings {
            for result in &finding.results {
                let key = result.title.to_lowercase();
                snippet_groups
                    .entry(key)
                    .or_default()
                    .push(result.url.clone());
            }
        }

        let mut corroborated = Vec::new();
        for (title_key, urls) in snippet_groups {
            if urls.len() > 1 {
                let unique_urls: HashSet<String> = urls.into_iter().collect();
                let url_list: Vec<String> = unique_urls.into_iter().collect();
                let count = url_list.len();
                corroborated.push(CorroboratedFinding {
                    finding_summary: title_key,
                    source_urls: url_list,
                    corroboration_count: count,
                });
            }
        }

        corroborated.sort_by_key(|a| std::cmp::Reverse(a.corroboration_count));
        corroborated
    }

    fn cross_validate_contradictions(&self, findings: &[ResearchFinding]) -> Vec<Contradiction> {
        let contradiction_indicators = [
            ("not", "is"),
            ("false", "true"),
            ("debunked", "proven"),
            ("myth", "fact"),
            ("against", "for"),
            ("disagree", "agree"),
            ("harmful", "beneficial"),
            ("failing", "succeeding"),
        ];

        let mut contradictions = Vec::new();
        let all_results: Vec<&SearchResult> =
            findings.iter().flat_map(|f| f.results.iter()).collect();

        for i in 0..all_results.len() {
            for j in (i + 1)..all_results.len() {
                let a = &all_results[i];
                let b = &all_results[j];

                if a.url == b.url {
                    continue;
                }

                let a_lower = format!("{} {}", a.title, a.snippet).to_lowercase();
                let b_lower = format!("{} {}", b.title, b.snippet).to_lowercase();

                for (neg, pos) in &contradiction_indicators {
                    let a_has_neg = a_lower.contains(neg);
                    let b_has_pos = b_lower.contains(pos);
                    let a_has_pos = a_lower.contains(pos);
                    let b_has_neg = b_lower.contains(neg);

                    if (a_has_neg && b_has_pos) || (a_has_pos && b_has_neg) {
                        contradictions.push(Contradiction {
                            topic: a.query.clone(),
                            source_a_url: a.url.clone(),
                            source_a_claim: a.snippet.clone(),
                            source_b_url: b.url.clone(),
                            source_b_claim: b.snippet.clone(),
                        });
                        break;
                    }
                }
            }
        }

        contradictions.truncate(20);
        contradictions
    }

    async fn ingest_result(&self, wiki_id: &str, result: &SearchResult) -> Result<String, String> {
        let content = format!(
            "# {}\n\n**Source:** [{}]({})\n\n**Research Query:** {}\n\n---\n\n{}",
            result.title, result.url, result.url, result.query, result.snippet
        );

        let ingest_result = self
            .ingest_pipeline
            .ingest_text(wiki_id, &content, IngestSourceType::WebArticle)
            .await?;

        Ok(ingest_result.source_id)
    }
}

#[derive(Debug, Deserialize)]
struct QueryRaw {
    query: String,
    rationale: String,
}

pub struct DeepResearcherBuilder {
    config: DeepResearchConfig,
}

impl DeepResearcherBuilder {
    pub fn new() -> Self {
        Self {
            config: DeepResearchConfig::default(),
        }
    }

    pub fn max_queries(mut self, max: usize) -> Self {
        self.config.max_queries = max;
        self
    }

    pub fn max_results_per_query(mut self, max: usize) -> Self {
        self.config.max_results_per_query = max;
        self
    }

    pub fn concurrent_searches(mut self, concurrent: usize) -> Self {
        self.config.concurrent_searches = concurrent;
        self
    }

    pub fn max_rounds(mut self, rounds: usize) -> Self {
        self.config.max_rounds = rounds;
        self
    }

    pub fn coverage_threshold(mut self, threshold: f32) -> Self {
        self.config.coverage_threshold = threshold;
        self
    }

    pub fn enable_gap_analysis(mut self, enable: bool) -> Self {
        self.config.enable_gap_analysis = enable;
        self
    }

    pub fn build(
        self,
        search_provider: Arc<WebSearchProvider>,
        ingest_pipeline: Arc<IngestPipeline>,
    ) -> DeepResearcher {
        DeepResearcher::new(self.config, search_provider, ingest_pipeline)
    }
}

impl Default for DeepResearcherBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deep_research_config_defaults() {
        let config = DeepResearchConfig::default();
        assert_eq!(config.max_queries, 5);
        assert_eq!(config.max_results_per_query, 5);
        assert_eq!(config.concurrent_searches, 3);
        assert_eq!(config.max_rounds, 3);
        assert!((config.coverage_threshold - 0.7).abs() < f32::EPSILON);
        assert!(config.enable_gap_analysis);
    }

    #[test]
    fn test_research_round_creation() {
        let round = ResearchRound {
            round_number: 1,
            queries: vec![ResearchQuery {
                query: "test query".to_string(),
                rationale: "test rationale".to_string(),
            }],
            findings: vec![],
            identified_gaps: vec!["definition of Rust".to_string()],
            coverage_score: 0.3,
        };
        assert_eq!(round.round_number, 1);
        assert_eq!(round.queries.len(), 1);
        assert!(round.findings.is_empty());
        assert_eq!(round.identified_gaps.len(), 1);
        assert!((round.coverage_score - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_corroborated_finding() {
        let f = CorroboratedFinding {
            finding_summary: "Rust is a systems language".to_string(),
            source_urls: vec!["https://a.com".to_string(), "https://b.com".to_string()],
            corroboration_count: 2,
        };
        assert_eq!(f.corroboration_count, 2);
        assert_eq!(f.source_urls.len(), 2);
    }

    #[test]
    fn test_contradiction() {
        let c = Contradiction {
            topic: "Rust performance".to_string(),
            source_a_url: "https://a.com".to_string(),
            source_a_claim: "Rust is fast".to_string(),
            source_b_url: "https://b.com".to_string(),
            source_b_claim: "Rust is slow".to_string(),
        };
        assert_eq!(c.topic, "Rust performance");
        assert_ne!(c.source_a_url, c.source_b_url);
    }

    #[test]
    fn test_research_query() {
        let q = ResearchQuery {
            query: "what is Rust".to_string(),
            rationale: "understand the basics".to_string(),
        };
        assert_eq!(q.query, "what is Rust");
        assert_eq!(q.rationale, "understand the basics");
    }

    #[test]
    fn test_search_result() {
        let r = SearchResult {
            query: "test".to_string(),
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
            snippet: "Some snippet".to_string(),
        };
        assert_eq!(r.url, "https://example.com");
    }

    #[test]
    fn test_research_finding() {
        let f = ResearchFinding {
            query: "test".to_string(),
            results: vec![SearchResult {
                query: "test".to_string(),
                url: "https://a.com".to_string(),
                title: "A".to_string(),
                snippet: "snippet a".to_string(),
            }],
        };
        assert_eq!(f.results.len(), 1);
    }

    #[test]
    fn test_deep_research_result() {
        let r = DeepResearchResult {
            topic: "Rust".to_string(),
            queries_generated: vec![],
            findings: vec![],
            pages_created: vec![],
            rounds: vec![],
            total_coverage_score: 0.0,
            corroborated_findings: vec![],
            contradictions: vec![],
        };
        assert_eq!(r.topic, "Rust");
        assert!(r.queries_generated.is_empty());
    }

    #[test]
    fn test_deep_researcher_builder_defaults() {
        let builder = DeepResearcherBuilder::new();
        assert_eq!(builder.config.max_queries, 5);
        assert_eq!(builder.config.max_rounds, 3);
    }

    #[test]
    fn test_deep_researcher_builder_custom() {
        let builder = DeepResearcherBuilder::new()
            .max_queries(10)
            .max_results_per_query(8)
            .concurrent_searches(5)
            .max_rounds(5)
            .coverage_threshold(0.9)
            .enable_gap_analysis(false);
        assert_eq!(builder.config.max_queries, 10);
        assert_eq!(builder.config.max_results_per_query, 8);
        assert_eq!(builder.config.concurrent_searches, 5);
        assert_eq!(builder.config.max_rounds, 5);
        assert!((builder.config.coverage_threshold - 0.9).abs() < f32::EPSILON);
        assert!(!builder.config.enable_gap_analysis);
    }

    #[test]
    fn test_deep_researcher_builder_default_trait() {
        let builder = DeepResearcherBuilder::default();
        assert_eq!(builder.config.max_queries, 5);
    }

    #[tokio::test]
    async fn test_analyze_coverage_empty() {
        let db = Arc::new(
            sea_orm::Database::connect(sea_orm::ConnectOptions::new("sqlite::memory:"))
                .await
                .unwrap(),
        );
        let pipeline = Arc::new(IngestPipeline::new(db));
        let searcher = Arc::new(WebSearchProvider::new());
        let researcher = DeepResearcher::new(DeepResearchConfig::default(), searcher, pipeline);
        let (score, gaps) = researcher.analyze_coverage("Rust programming", &[], &[]);
        assert!((score - 0.0).abs() < f32::EPSILON);
        assert!(!gaps.is_empty());
    }

    #[tokio::test]
    async fn test_analyze_coverage_with_findings() {
        let db = Arc::new(
            sea_orm::Database::connect(sea_orm::ConnectOptions::new("sqlite::memory:"))
                .await
                .unwrap(),
        );
        let pipeline = Arc::new(IngestPipeline::new(db));
        let searcher = Arc::new(WebSearchProvider::new());
        let researcher = DeepResearcher::new(DeepResearchConfig::default(), searcher, pipeline);
        let findings = vec![ResearchFinding {
            query: "Rust programming".to_string(),
            results: vec![
                SearchResult {
                    query: "Rust programming".to_string(),
                    url: "https://rust-lang.org".to_string(),
                    title: "Rust Programming Language definition and meaning".to_string(),
                    snippet: "Rust is a systems programming language focused on safety".to_string(),
                },
                SearchResult {
                    query: "Rust programming".to_string(),
                    url: "https://example.com".to_string(),
                    title: "Rust recent developments 2025".to_string(),
                    snippet: "Rust applications in production use cases".to_string(),
                },
            ],
        }];
        let (score, _gaps) = researcher.analyze_coverage("Rust programming", &findings, &[]);
        assert!(score > 0.0);
    }

    #[tokio::test]
    async fn test_analyze_coverage_identifies_gaps() {
        let db = Arc::new(
            sea_orm::Database::connect(sea_orm::ConnectOptions::new("sqlite::memory:"))
                .await
                .unwrap(),
        );
        let pipeline = Arc::new(IngestPipeline::new(db));
        let searcher = Arc::new(WebSearchProvider::new());
        let researcher = DeepResearcher::new(DeepResearchConfig::default(), searcher, pipeline);
        let findings = vec![ResearchFinding {
            query: "obscure topic".to_string(),
            results: vec![SearchResult {
                query: "obscure topic".to_string(),
                url: "https://example.com".to_string(),
                title: "Some result".to_string(),
                snippet: "A brief mention".to_string(),
            }],
        }];
        let (_, gaps) = researcher.analyze_coverage("obscure topic xyz", &findings, &[]);
        assert!(!gaps.is_empty());
    }

    #[tokio::test]
    async fn test_generate_gap_queries_without_llm() {
        let db = Arc::new(
            sea_orm::Database::connect(sea_orm::ConnectOptions::new("sqlite::memory:"))
                .await
                .unwrap(),
        );
        let pipeline = Arc::new(IngestPipeline::new(db));
        let searcher = Arc::new(WebSearchProvider::new());
        let researcher = DeepResearcher::new(DeepResearchConfig::default(), searcher, pipeline);
        let gaps = vec![
            "definition of Rust".to_string(),
            "history of Rust".to_string(),
        ];
        let queries = researcher
            .generate_gap_queries("Rust", &gaps, None, None, None)
            .await
            .unwrap();
        assert!(!queries.is_empty());
        assert!(queries.len() <= 5);
    }

    #[tokio::test]
    async fn test_generate_gap_queries_empty_gaps() {
        let db = Arc::new(
            sea_orm::Database::connect(sea_orm::ConnectOptions::new("sqlite::memory:"))
                .await
                .unwrap(),
        );
        let pipeline = Arc::new(IngestPipeline::new(db));
        let searcher = Arc::new(WebSearchProvider::new());
        let researcher = DeepResearcher::new(DeepResearchConfig::default(), searcher, pipeline);
        let queries = researcher
            .generate_gap_queries("Rust", &[], None, None, None)
            .await
            .unwrap();
        assert!(!queries.is_empty());
    }

    #[tokio::test]
    async fn test_cross_validate_corroboration() {
        let db = Arc::new(
            sea_orm::Database::connect(sea_orm::ConnectOptions::new("sqlite::memory:"))
                .await
                .unwrap(),
        );
        let pipeline = Arc::new(IngestPipeline::new(db));
        let searcher = Arc::new(WebSearchProvider::new());
        let researcher = DeepResearcher::new(DeepResearchConfig::default(), searcher, pipeline);
        let findings = vec![
            ResearchFinding {
                query: "q1".to_string(),
                results: vec![SearchResult {
                    query: "q1".to_string(),
                    url: "https://a.com".to_string(),
                    title: "Rust is safe".to_string(),
                    snippet: "Rust provides memory safety".to_string(),
                }],
            },
            ResearchFinding {
                query: "q2".to_string(),
                results: vec![SearchResult {
                    query: "q2".to_string(),
                    url: "https://b.com".to_string(),
                    title: "Rust is safe".to_string(),
                    snippet: "Rust guarantees safety".to_string(),
                }],
            },
        ];
        let corroborated = researcher.cross_validate_corroboration(&findings);
        assert!(!corroborated.is_empty());
        assert!(corroborated[0].corroboration_count >= 2);
    }

    #[tokio::test]
    async fn test_cross_validate_corroboration_no_overlap() {
        let db = Arc::new(
            sea_orm::Database::connect(sea_orm::ConnectOptions::new("sqlite::memory:"))
                .await
                .unwrap(),
        );
        let pipeline = Arc::new(IngestPipeline::new(db));
        let searcher = Arc::new(WebSearchProvider::new());
        let researcher = DeepResearcher::new(DeepResearchConfig::default(), searcher, pipeline);
        let findings = vec![ResearchFinding {
            query: "q1".to_string(),
            results: vec![SearchResult {
                query: "q1".to_string(),
                url: "https://a.com".to_string(),
                title: "Unique title".to_string(),
                snippet: "Unique content".to_string(),
            }],
        }];
        let corroborated = researcher.cross_validate_corroboration(&findings);
        assert!(corroborated.is_empty());
    }

    #[tokio::test]
    async fn test_cross_validate_contradictions() {
        let db = Arc::new(
            sea_orm::Database::connect(sea_orm::ConnectOptions::new("sqlite::memory:"))
                .await
                .unwrap(),
        );
        let pipeline = Arc::new(IngestPipeline::new(db));
        let searcher = Arc::new(WebSearchProvider::new());
        let researcher = DeepResearcher::new(DeepResearchConfig::default(), searcher, pipeline);
        let findings = vec![ResearchFinding {
            query: "q1".to_string(),
            results: vec![
                SearchResult {
                    query: "q1".to_string(),
                    url: "https://a.com".to_string(),
                    title: "Rust is harmful".to_string(),
                    snippet: "Rust is harmful for productivity".to_string(),
                },
                SearchResult {
                    query: "q1".to_string(),
                    url: "https://b.com".to_string(),
                    title: "Rust is beneficial".to_string(),
                    snippet: "Rust is beneficial for productivity".to_string(),
                },
            ],
        }];
        let contradictions = researcher.cross_validate_contradictions(&findings);
        assert!(!contradictions.is_empty());
    }

    #[tokio::test]
    async fn test_cross_validate_contradictions_same_url_ignored() {
        let db = Arc::new(
            sea_orm::Database::connect(sea_orm::ConnectOptions::new("sqlite::memory:"))
                .await
                .unwrap(),
        );
        let pipeline = Arc::new(IngestPipeline::new(db));
        let searcher = Arc::new(WebSearchProvider::new());
        let researcher = DeepResearcher::new(DeepResearchConfig::default(), searcher, pipeline);
        let findings = vec![ResearchFinding {
            query: "q1".to_string(),
            results: vec![
                SearchResult {
                    query: "q1".to_string(),
                    url: "https://same.com".to_string(),
                    title: "Rust is harmful".to_string(),
                    snippet: "Rust is harmful for productivity".to_string(),
                },
                SearchResult {
                    query: "q1".to_string(),
                    url: "https://same.com".to_string(),
                    title: "Rust is beneficial".to_string(),
                    snippet: "Rust is beneficial for productivity".to_string(),
                },
            ],
        }];
        let contradictions = researcher.cross_validate_contradictions(&findings);
        assert!(contradictions.is_empty());
    }

    #[tokio::test]
    async fn test_cross_validate_contradictions_no_contradictions() {
        let db = Arc::new(
            sea_orm::Database::connect(sea_orm::ConnectOptions::new("sqlite::memory:"))
                .await
                .unwrap(),
        );
        let pipeline = Arc::new(IngestPipeline::new(db));
        let searcher = Arc::new(WebSearchProvider::new());
        let researcher = DeepResearcher::new(DeepResearchConfig::default(), searcher, pipeline);
        let findings = vec![ResearchFinding {
            query: "q1".to_string(),
            results: vec![
                SearchResult {
                    query: "q1".to_string(),
                    url: "https://a.com".to_string(),
                    title: "Rust overview".to_string(),
                    snippet: "Rust is a programming language".to_string(),
                },
                SearchResult {
                    query: "q1".to_string(),
                    url: "https://b.com".to_string(),
                    title: "Rust features".to_string(),
                    snippet: "Rust has many features".to_string(),
                },
            ],
        }];
        let contradictions = researcher.cross_validate_contradictions(&findings);
        assert!(contradictions.is_empty());
    }

    #[tokio::test]
    async fn test_default_queries() {
        let db = Arc::new(
            sea_orm::Database::connect(sea_orm::ConnectOptions::new("sqlite::memory:"))
                .await
                .unwrap(),
        );
        let pipeline = Arc::new(IngestPipeline::new(db));
        let searcher = Arc::new(WebSearchProvider::new());
        let researcher = DeepResearcher::new(DeepResearchConfig::default(), searcher, pipeline);
        let queries = researcher.default_queries("Rust");
        assert_eq!(queries.len(), 5);
        assert!(queries[0].query.contains("Rust"));
    }

    #[tokio::test]
    async fn test_build_context_with_overview() {
        let db = Arc::new(
            sea_orm::Database::connect(sea_orm::ConnectOptions::new("sqlite::memory:"))
                .await
                .unwrap(),
        );
        let pipeline = Arc::new(IngestPipeline::new(db));
        let searcher = Arc::new(WebSearchProvider::new());
        let researcher = DeepResearcher::new(DeepResearchConfig::default(), searcher, pipeline);
        let ctx = researcher.build_context(Some("Rust is a systems language"), "Rust safety");
        assert!(ctx.contains("Wiki Overview"));
        assert!(ctx.contains("Rust is a systems language"));
        assert!(ctx.contains("Research Topic"));
        assert!(ctx.contains("Rust safety"));
    }

    #[tokio::test]
    async fn test_build_context_without_overview() {
        let db = Arc::new(
            sea_orm::Database::connect(sea_orm::ConnectOptions::new("sqlite::memory:"))
                .await
                .unwrap(),
        );
        let pipeline = Arc::new(IngestPipeline::new(db));
        let searcher = Arc::new(WebSearchProvider::new());
        let researcher = DeepResearcher::new(DeepResearchConfig::default(), searcher, pipeline);
        let ctx = researcher.build_context(None, "Rust safety");
        assert!(!ctx.contains("Wiki Overview"));
        assert!(ctx.contains("Research Topic"));
    }

    #[tokio::test]
    async fn test_extract_json_array() {
        let db = Arc::new(
            sea_orm::Database::connect(sea_orm::ConnectOptions::new("sqlite::memory:"))
                .await
                .unwrap(),
        );
        let pipeline = Arc::new(IngestPipeline::new(db));
        let searcher = Arc::new(WebSearchProvider::new());
        let researcher = DeepResearcher::new(DeepResearchConfig::default(), searcher, pipeline);
        let text = r#"Here are the results: [{"query": "test", "rationale": "why"}] done"#;
        let json = researcher.extract_json(text).unwrap();
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
    }

    #[tokio::test]
    async fn test_extract_json_object() {
        let db = Arc::new(
            sea_orm::Database::connect(sea_orm::ConnectOptions::new("sqlite::memory:"))
                .await
                .unwrap(),
        );
        let pipeline = Arc::new(IngestPipeline::new(db));
        let searcher = Arc::new(WebSearchProvider::new());
        let researcher = DeepResearcher::new(DeepResearchConfig::default(), searcher, pipeline);
        let text = r#"Result: {"key": "value"} end"#;
        let json = researcher.extract_json(text).unwrap();
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
    }

    #[tokio::test]
    async fn test_extract_json_no_json() {
        let db = Arc::new(
            sea_orm::Database::connect(sea_orm::ConnectOptions::new("sqlite::memory:"))
                .await
                .unwrap(),
        );
        let pipeline = Arc::new(IngestPipeline::new(db));
        let searcher = Arc::new(WebSearchProvider::new());
        let researcher = DeepResearcher::new(DeepResearchConfig::default(), searcher, pipeline);
        let result = researcher.extract_json("no json here");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_url_deduplication_in_execute_searches() {
        let mut seen_urls: HashSet<String> = HashSet::new();
        seen_urls.insert("https://duplicate.com".to_string());
        let new_urls = vec![
            "https://duplicate.com".to_string(),
            "https://unique.com".to_string(),
        ];
        let deduped: Vec<String> = new_urls
            .into_iter()
            .filter(|u| !seen_urls.contains(u))
            .collect();
        for u in &deduped {
            seen_urls.insert(u.clone());
        }
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0], "https://unique.com");
        assert_eq!(seen_urls.len(), 2);
    }

    #[tokio::test]
    async fn test_default_gap_queries_definition() {
        let db = Arc::new(
            sea_orm::Database::connect(sea_orm::ConnectOptions::new("sqlite::memory:"))
                .await
                .unwrap(),
        );
        let pipeline = Arc::new(IngestPipeline::new(db));
        let searcher = Arc::new(WebSearchProvider::new());
        let researcher = DeepResearcher::new(DeepResearchConfig::default(), searcher, pipeline);
        let gaps = vec!["definition of Rust".to_string()];
        let queries = researcher.default_gap_queries("Rust", &gaps);
        assert!(!queries.is_empty());
        assert!(queries.iter().any(|q| q.query.contains("Rust")));
    }

    #[tokio::test]
    async fn test_default_gap_queries_empty() {
        let db = Arc::new(
            sea_orm::Database::connect(sea_orm::ConnectOptions::new("sqlite::memory:"))
                .await
                .unwrap(),
        );
        let pipeline = Arc::new(IngestPipeline::new(db));
        let searcher = Arc::new(WebSearchProvider::new());
        let researcher = DeepResearcher::new(DeepResearchConfig::default(), searcher, pipeline);
        let queries = researcher.default_gap_queries("Rust", &[]);
        assert!(!queries.is_empty());
        assert!(queries[0].query.contains("Rust"));
    }

    #[test]
    fn test_research_phase_variants() {
        let phases = [
            ResearchPhase::InitialSearch,
            ResearchPhase::Analysis,
            ResearchPhase::GapIdentification,
            ResearchPhase::DeepeningSearch,
            ResearchPhase::Synthesis,
        ];
        assert_eq!(phases.len(), 5);
    }
}
