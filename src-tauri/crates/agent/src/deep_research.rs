use std::sync::Arc;

use axagent_providers::{ProviderAdapter, ProviderRequestContext};
use axagent_core::types::{ChatContent, ChatMessage, ChatRequest};
use serde::{Deserialize, Serialize};

use crate::ingest_pipeline::{IngestPipeline, IngestSourceType};
use crate::search_provider::SearchProvider;
use crate::web_search::WebSearchProvider;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepResearchConfig {
    pub max_queries: usize,
    pub max_results_per_query: usize,
    pub concurrent_searches: usize,
}

impl Default for DeepResearchConfig {
    fn default() -> Self {
        Self {
            max_queries: 5,
            max_results_per_query: 5,
            concurrent_searches: 3,
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

        let queries = if let (Some(ref adapter), Some(ctx), Some(model)) =
            (&llm_adapter, &llm_ctx, &llm_model)
        {
            self.generate_queries(topic, &context, adapter.as_ref(), &ctx, model).await?
        } else {
            self.default_queries(topic)
        };

        let findings = self.execute_searches(&queries).await;

        let mut pages_created = Vec::new();
        for finding in &findings {
            for result in &finding.results {
                let page_result = self.ingest_result(wiki_id, result).await;
                if let Ok(page_id) = page_result {
                    pages_created.push(page_id);
                }
            }
        }

        Ok(DeepResearchResult {
            topic: topic.to_string(),
            queries_generated: queries,
            findings,
            pages_created,
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
        if let Some(start) = text.find('[') {
            if let Some(end) = text.rfind(']') {
                return Ok(text[start..=end].to_string());
            }
        }
        if let Some(start) = text.find('{') {
            if let Some(end) = text.rfind('}') {
                return Ok(text[start..=end].to_string());
            }
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

    async fn execute_searches(&self, queries: &[ResearchQuery]) -> Vec<ResearchFinding> {
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
                    }
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
            if let Ok(finding) = handle.await {
                findings.push(finding);
            }
        }

        findings
    }

    async fn ingest_result(
        &self,
        wiki_id: &str,
        result: &SearchResult,
    ) -> Result<String, String> {
        let content = format!(
            "# {}\n\n**Source:** [{}]({})\n\n**Research Query:** {}\n\n---\n\n{}",
            result.title,
            result.url,
            result.url,
            result.query,
            result.snippet
        );

        let ingest_result = self.ingest_pipeline.ingest_text(
            wiki_id,
            &content,
            IngestSourceType::WebArticle,
        ).await?;

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