use crate::error_recovery_engine::ErrorRecoveryEngine;
use crate::research_state::{
    Citation, ResearchConfig, ResearchPhase, ResearchProgress, ResearchReport, ResearchState,
    ResearchStatus, SearchPlan, SearchResult,
};
use crate::search_orchestrator::SearchOrchestrator;
use crate::search_planner::SearchPlanner;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{RwLock, broadcast};

#[derive(Error, Debug)]
pub enum ResearchError {
    #[error("Research not started")]
    NotStarted,
    #[error("Research already completed")]
    AlreadyCompleted,
    #[error("Research failed: {0}")]
    Failed(String),
    #[error("Invalid state transition from {from:?} to {to:?}")]
    InvalidStateTransition {
        from: ResearchStatus,
        to: ResearchStatus,
    },
    #[error("Search planning failed: {0}")]
    PlanningFailed(String),
    #[error("Search execution failed: {0}")]
    SearchFailed(String),
    #[error("Report generation failed: {0}")]
    ReportGenerationFailed(String),
    #[error("LLM generation failed: {0}")]
    LlmFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResearchEvent {
    Started {
        topic: String,
    },
    PhaseChanged {
        from: ResearchPhase,
        to: ResearchPhase,
    },
    SourcesFound {
        count: usize,
    },
    SourceProcessed {
        source_id: String,
    },
    CitationAdded {
        citation_id: String,
    },
    ReportGenerated {
        report_id: String,
    },
    Completed,
    Failed {
        error: String,
    },
    Paused,
    Resumed,
    LlmGenerationStarted {
        phase: String,
    },
    LlmGenerationCompleted {
        phase: String,
    },
}

pub trait LlmContentGenerator: Send + Sync {
    fn generate_outline(
        &self,
        topic: &str,
        context: &str,
    ) -> impl std::future::Future<Output = Result<String, ResearchError>> + Send;
    fn generate_content(
        &self,
        topic: &str,
        outline: &str,
        sources: &str,
    ) -> impl std::future::Future<Output = Result<String, ResearchError>> + Send;
    fn generate_summary(
        &self,
        topic: &str,
        findings: &str,
    ) -> impl std::future::Future<Output = Result<String, ResearchError>> + Send;
}

pub struct ResearchAgent {
    planner: SearchPlanner,
    orchestrator: SearchOrchestrator,
    state: Arc<RwLock<ResearchState>>,
    event_sender: broadcast::Sender<ResearchEvent>,
    content_generator: Option<Arc<DefaultLlmContentGenerator>>,
    error_recovery_engine: Option<Arc<ErrorRecoveryEngine>>,
}

impl ResearchAgent {
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(100);
        Self {
            planner: SearchPlanner::new(),
            orchestrator: SearchOrchestrator::new(),
            state: Arc::new(RwLock::new(ResearchState::new(String::new()))),
            event_sender,
            content_generator: None,
            error_recovery_engine: None,
        }
    }

    pub fn with_config(config: ResearchConfig) -> Self {
        let (event_sender, _) = broadcast::channel(100);
        Self {
            planner: SearchPlanner::new(),
            orchestrator: SearchOrchestrator::new(),
            state: Arc::new(RwLock::new(ResearchState::new(String::new()).with_config(config))),
            event_sender,
            content_generator: None,
            error_recovery_engine: None,
        }
    }

    pub fn with_generator(mut self, generator: Arc<DefaultLlmContentGenerator>) -> Self {
        self.content_generator = Some(generator);
        self
    }

    pub fn with_planner(mut self, planner: SearchPlanner) -> Self {
        self.planner = planner;
        self
    }

    pub fn with_orchestrator(mut self, orchestrator: SearchOrchestrator) -> Self {
        self.orchestrator = orchestrator;
        self
    }

    pub fn with_error_recovery(mut self, engine: Arc<ErrorRecoveryEngine>) -> Self {
        self.error_recovery_engine = Some(engine);
        self
    }

    pub async fn get_state(&self) -> ResearchState {
        self.state.read().await.clone()
    }

    pub async fn get_progress(&self) -> ResearchProgress {
        self.state.read().await.progress.clone()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ResearchEvent> {
        self.event_sender.subscribe()
    }

    fn emit(&self, event: ResearchEvent) {
        let _ = self.event_sender.send(event);
    }

    pub async fn start(&self, topic: String) -> Result<String, ResearchError> {
        let mut state = self.state.write().await;

        if state.status == ResearchStatus::InProgress {
            return Err(ResearchError::AlreadyCompleted);
        }

        state.topic = topic.clone();
        state.status = ResearchStatus::InProgress;
        state.current_phase = ResearchPhase::Planning;
        state.progress = ResearchProgress::new().with_phase(ResearchPhase::Planning);

        self.emit(ResearchEvent::Started {
            topic: topic.clone(),
        });
        tracing::info!("Research started: {}", topic);

        Ok(state.id.clone())
    }

    pub async fn execute_research(&self) -> Result<ResearchReport, ResearchError> {
        let state = self.state.read().await.clone();

        if state.status != ResearchStatus::InProgress {
            return Err(ResearchError::NotStarted);
        }

        drop(state);

        self.planning_phase().await?;
        self.searching_phase().await?;
        self.extraction_phase().await?;
        self.analysis_phase().await?;
        self.synthesis_phase().await?;
        self.reporting_phase().await?;

        let final_state = self.state.read().await.clone();
        let report = self.generate_report(&final_state).await?;

        {
            let mut state = self.state.write().await;
            state.complete();
        }

        self.emit(ResearchEvent::Completed);

        Ok(report)
    }

    async fn planning_phase(&self) -> Result<SearchPlan, ResearchError> {
        self.update_phase(ResearchPhase::Planning).await;

        let topic = self.state.read().await.topic.clone();
        let plan = self.planner.plan(&topic);

        tracing::info!("Planning phase complete, generated {} queries", plan.queries.len());

        Ok(plan)
    }

    async fn searching_phase(&self) -> Result<Vec<SearchResult>, ResearchError> {
        self.update_phase(ResearchPhase::Searching).await;

        let plan = {
            let topic = self.state.read().await.topic.clone();
            self.planner.plan(&topic)
        };

        let results = self
            .orchestrator
            .execute(&plan)
            .await
            .map_err(|e| ResearchError::SearchFailed(e.to_string()))?;

        {
            let mut state = self.state.write().await;
            for result in &results {
                state.add_search_result(result.clone());
            }
        }

        self.emit(ResearchEvent::SourcesFound {
            count: results.len(),
        });
        tracing::info!("Searching phase complete, found {} sources", results.len());

        Ok(results)
    }

    async fn extraction_phase(&self) -> Result<(), ResearchError> {
        self.update_phase(ResearchPhase::Extracting).await;

        let results = self.state.read().await.search_results.clone();
        let max_citations = self.state.read().await.config.max_citations;
        let mut citations_added = 0;

        let mut sorted_results = results.clone();
        sorted_results.sort_by(|a, b| {
            let score_a = a.relevance_score
                + a.credibility_score
                    .unwrap_or(a.source_type.default_credibility());
            let score_b = b.relevance_score
                + b.credibility_score
                    .unwrap_or(b.source_type.default_credibility());
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut seen_urls: std::collections::HashSet<String> = std::collections::HashSet::new();

        for result in sorted_results.iter() {
            if citations_added >= max_citations {
                break;
            }

            let url_normalized = result.url.to_lowercase();
            if seen_urls.contains(&url_normalized) {
                tracing::debug!("Skipping duplicate URL: {}", result.url);
                continue;
            }
            seen_urls.insert(url_normalized);

            let citation =
                Citation::new(result.url.clone(), result.title.clone(), result.source_type)
                    .with_credibility(
                        result
                            .credibility_score
                            .unwrap_or(result.source_type.default_credibility()),
                    );

            {
                let mut state = self.state.write().await;
                state.add_citation(citation.clone());
            }

            citations_added += 1;
            self.emit(ResearchEvent::CitationAdded {
                citation_id: citation.id.clone(),
            });
        }

        tracing::info!(
            "Extraction phase complete, added {} citations (from {} total results)",
            citations_added,
            results.len()
        );

        Ok(())
    }

    async fn analysis_phase(&self) -> Result<(), ResearchError> {
        self.update_phase(ResearchPhase::Analyzing).await;

        let citations = self.state.read().await.citations.clone();

        let citation_count = citations.len();
        for idx in 0..citation_count {
            tracing::debug!("Analyzing citation {} of {}", idx + 1, citation_count);
            let mut state = self.state.write().await;
            state.progress.increment_sources_processed();
        }

        tracing::info!("Analysis phase complete, processed {} sources", citations.len());

        Ok(())
    }

    async fn synthesis_phase(&self) -> Result<(), ResearchError> {
        self.update_phase(ResearchPhase::Synthesizing).await;

        let citations = self.state.read().await.citations.clone();
        let topic = self.state.read().await.topic.clone();

        tracing::info!(
            "Synthesis phase complete for topic '{}' with {} citations",
            topic,
            citations.len()
        );

        Ok(())
    }

    async fn reporting_phase(&self) -> Result<ResearchReport, ResearchError> {
        self.update_phase(ResearchPhase::Reporting).await;

        let state = self.state.read().await.clone();
        let report = self.generate_report(&state).await?;

        self.emit(ResearchEvent::ReportGenerated {
            report_id: report.id.clone(),
        });

        tracing::info!("Reporting phase complete, report_id: {}", report.id);

        Ok(report)
    }

    async fn generate_report(
        &self,
        state: &ResearchState,
    ) -> Result<ResearchReport, ResearchError> {
        let mut report = ResearchReport::new(state.topic.clone());

        let outline = self.generate_outline(state).await?;
        report = report.with_outline(outline);

        let content = self.generate_content(state).await?;
        report = report.with_content(content);

        report = report.with_citations(state.citations.clone());

        let summary = self.generate_summary(state).await?;
        report = report.with_summary(summary);

        Ok(report)
    }

    async fn generate_outline(
        &self,
        state: &ResearchState,
    ) -> Result<crate::research_state::ReportOutline, ResearchError> {
        use crate::research_state::{OutlineSection, ReportOutline};

        if let Some(ref generator) = self.content_generator {
            self.emit(ResearchEvent::LlmGenerationStarted {
                phase: "outline".to_string(),
            });

            let context = self.build_research_context(state);
            let outline_json = generator.generate_outline(&state.topic, &context).await?;

            self.emit(ResearchEvent::LlmGenerationCompleted {
                phase: "outline".to_string(),
            });

            if let Ok(outline) = serde_json::from_str::<Vec<OutlineSection>>(&outline_json) {
                let mut report_outline =
                    ReportOutline::new().with_title(format!("关于「{}」的研究报告", state.topic));
                for section in outline {
                    report_outline = report_outline.add_section(section);
                }
                return Ok(report_outline);
            }
        }

        let sections = [
            OutlineSection::new("摘要".to_string())
                .with_description("研究主题的简要概述".to_string()),
            OutlineSection::new("背景介绍".to_string())
                .with_description("研究主题的背景信息".to_string()),
            OutlineSection::new("主要发现".to_string())
                .with_description("从多个来源中提取的主要发现".to_string()),
            OutlineSection::new("分析讨论".to_string())
                .with_description("对发现进行深入分析".to_string()),
            OutlineSection::new("结论".to_string()).with_description("研究结论和建议".to_string()),
            OutlineSection::new("参考文献".to_string())
                .with_description("所有引用的来源".to_string()),
        ];

        let outline = ReportOutline::new()
            .with_title(format!("关于「{}」的研究报告", state.topic))
            .add_section(sections[0].clone())
            .add_section(sections[1].clone())
            .add_section(sections[2].clone())
            .add_section(sections[3].clone())
            .add_section(sections[4].clone())
            .add_section(sections[5].clone());

        Ok(outline)
    }

    async fn generate_content(&self, state: &ResearchState) -> Result<String, ResearchError> {
        if let Some(ref generator) = self.content_generator {
            self.emit(ResearchEvent::LlmGenerationStarted {
                phase: "content".to_string(),
            });

            let sources = self.format_sources_for_llm(state);
            let outline = format!("{:?}", state.topic);

            let content = generator
                .generate_content(&state.topic, &outline, &sources)
                .await?;

            self.emit(ResearchEvent::LlmGenerationCompleted {
                phase: "content".to_string(),
            });

            return Ok(content);
        }

        let mut content = String::new();

        content.push_str(&format!("# 关于「{}」的研究报告\n\n", state.topic));

        content.push_str("## 摘要\n\n");
        content.push_str(&format!(
            "本报告基于对 {} 个来源的研究，对「{}」进行了深入分析。\n\n",
            state.citations.len(),
            state.topic
        ));

        content.push_str("## 背景介绍\n\n");
        content.push_str(&format!(
            "以下是从多个可靠来源收集的关于「{}」的背景信息。\n\n",
            state.topic
        ));

        content.push_str("## 主要发现\n\n");
        for (idx, result) in state.search_results.iter().take(5).enumerate() {
            content.push_str(&format!("### 发现 {}: {}\n\n", idx + 1, result.title));
            content.push_str(&format!("{}\n\n", result.snippet));
        }

        content.push_str("## 分析讨论\n\n");
        content.push_str("基于以上发现，我们可以得出以下分析结论...\n\n");

        content.push_str("## 结论\n\n");
        content.push_str(&format!(
            "通过对 {} 个来源的深入研究和分析，我们对「{}」有了更全面的认识。\n\n",
            state.citations.len(),
            state.topic
        ));

        content.push_str("## 参考文献\n\n");
        for (idx, citation) in state.citations.iter().enumerate() {
            content.push_str(&format!(
                "[{}] {} - {}\n",
                idx + 1,
                citation.source_title,
                citation.source_url
            ));
        }

        Ok(content)
    }

    async fn generate_summary(&self, state: &ResearchState) -> Result<String, ResearchError> {
        if let Some(ref generator) = self.content_generator {
            self.emit(ResearchEvent::LlmGenerationStarted {
                phase: "summary".to_string(),
            });

            let findings = self.format_findings_for_llm(state);
            let summary = generator.generate_summary(&state.topic, &findings).await?;

            self.emit(ResearchEvent::LlmGenerationCompleted {
                phase: "summary".to_string(),
            });

            return Ok(summary);
        }

        Ok(format!(
            "本研究通过搜索和分析 {} 个来源，对「{}」进行了系统性研究。\
            主要发现了 {} 条相关信息，并生成了包含 {} 个引用的研究报告。",
            state.search_results.len(),
            state.topic,
            state.search_results.len(),
            state.citations.len()
        ))
    }

    fn build_research_context(&self, state: &ResearchState) -> String {
        let sources_summary = state
            .search_results
            .iter()
            .take(10)
            .map(|r| format!("- {} ({})", r.title, r.url))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "Topic: {}\nNumber of sources: {}\nNumber of citations: {}\n\nKey sources:\n{}",
            state.topic,
            state.search_results.len(),
            state.citations.len(),
            sources_summary
        )
    }

    fn format_sources_for_llm(&self, state: &ResearchState) -> String {
        state
            .search_results
            .iter()
            .take(20)
            .map(|r| {
                format!(
                    "Source: {}\nURL: {}\nType: {:?}\nContent: {}\n---\n",
                    r.title, r.url, r.source_type, r.snippet
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn format_findings_for_llm(&self, state: &ResearchState) -> String {
        let findings = state
            .search_results
            .iter()
            .take(10)
            .map(|r| format!("- {}", r.title))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "Research topic: {}\n\nKey findings:\n{}\n\nTotal sources analyzed: {}\nTotal citations: {}",
            state.topic,
            findings,
            state.search_results.len(),
            state.citations.len()
        )
    }

    async fn update_phase(&self, new_phase: ResearchPhase) {
        let (current_phase, progress) = {
            let state = self.state.read().await;
            (state.current_phase, state.progress.clone())
        };

        if current_phase != new_phase {
            {
                let mut state = self.state.write().await;
                state.current_phase = new_phase;
                state.progress = progress.with_phase(new_phase);
            }

            self.emit(ResearchEvent::PhaseChanged {
                from: current_phase,
                to: new_phase,
            });
        }
    }
}

impl Default for ResearchAgent {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DefaultLlmContentGenerator {
    llm_adapter: Option<Arc<dyn axagent_harness::ProviderAdapter>>,
    ctx: Option<axagent_harness::ProviderRequestContext>,
}

impl Default for DefaultLlmContentGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultLlmContentGenerator {
    pub fn new() -> Self {
        Self {
            llm_adapter: None,
            ctx: None,
        }
    }

    pub fn with_llm(
        mut self,
        adapter: Arc<dyn axagent_harness::ProviderAdapter>,
        ctx: axagent_harness::ProviderRequestContext,
    ) -> Self {
        self.llm_adapter = Some(adapter);
        self.ctx = Some(ctx);
        self
    }

    async fn call_llm(&self, system: &str, user: &str) -> Result<String, ResearchError> {
        use axagent_core::types::{ChatContent, ChatMessage, ChatRequest};

        match (&self.llm_adapter, &self.ctx) {
            (Some(adapter), Some(ctx)) => {
                let request = ChatRequest {
                    model: "gpt-4o".to_string(),
                    messages: vec![
                        ChatMessage {
                            role: "system".to_string(),
                            content: ChatContent::Text(system.to_string()),
                            tool_calls: None,
                            tool_call_id: None,
                            thinking: None,
                        },
                        ChatMessage {
                            role: "user".to_string(),
                            content: ChatContent::Text(user.to_string()),
                            tool_calls: None,
                            tool_call_id: None,
                            thinking: None,
                        },
                    ],
                    temperature: Some(0.7),
                    max_tokens: Some(4096),
                    stream: false,
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
                    .map_err(|e| ResearchError::LlmFailed(e.to_string()))?;

                Ok(response.content)
            },
            _ => Err(ResearchError::LlmFailed("No LLM adapter configured".to_string())),
        }
    }
}

impl LlmContentGenerator for DefaultLlmContentGenerator {
    async fn generate_outline(&self, topic: &str, context: &str) -> Result<String, ResearchError> {
        let system = r#"你是一个任务分解专家。根据提供的研究主题和上下文信息，生成详细的研究报告大纲。

要求：
1. 大纲应包含6-8个主要章节
2. 每个章节需要包含2-3个子节
3. 使用JSON格式输出，格式如下:
{
  "sections": [
    {"title": "章节标题", "description": "章节内容概述", "subsections": [
      {"title": "子节标题", "description": "子节内容概述"}
    ]}
  ]
}"#;

        let user = format!("研究主题: {}\n\n上下文信息:\n{}", topic, context);

        if let Ok(response) = self.call_llm(system, &user).await {
            Ok(response)
        } else {
            let sections = serde_json::json!([
                {"title": format!("{} - 摘要", topic), "description": "研究主题的简要概述"},
                {"title": format!("{} - 背景介绍", topic), "description": "研究主题的背景信息"},
                {"title": "主要发现", "description": "从多个来源中提取的主要发现"},
                {"title": "分析讨论", "description": "对发现进行深入分析"},
                {"title": "结论", "description": "研究结论和建议"},
                {"title": "参考文献", "description": "所有引用的来源"}
            ]);
            Ok(serde_json::to_string(&sections).unwrap_or_default())
        }
    }

    async fn generate_content(
        &self,
        topic: &str,
        outline: &str,
        sources: &str,
    ) -> Result<String, ResearchError> {
        let system = r#"你是一个专业的研究报告撰写专家。根据提供的大纲和来源信息，生成完整的研究报告内容。

要求：
1. 内容应详尽、深入，覆盖大纲的所有要点
2. 适当引用来源信息，使用[来源描述]格式标注引用
3. 保持学术写作风格，逻辑清晰
4. 输出完整的Markdown格式报告"#;

        let user = format!("研究主题: {}\n\n大纲:\n{}\n\n来源信息:\n{}", topic, outline, sources);

        self.call_llm(system, &user).await
    }

    async fn generate_summary(&self, topic: &str, findings: &str) -> Result<String, ResearchError> {
        let system = r#"你是一个研究总结专家。根据提供的研究发现，生成简洁准确的研究总结。"#;

        let user = format!("研究主题: {}\n\n研究发现:\n{}", topic, findings);

        self.call_llm(system, &user).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::research_state::{Citation, ResearchConfig, SearchResult, SourceType};

    #[tokio::test]
    async fn test_research_agent_creation() {
        let agent = ResearchAgent::new();
        assert!(agent.content_generator.is_none());
    }

    #[tokio::test]
    async fn test_research_agent_with_generator() {
        let agent =
            ResearchAgent::new().with_generator(Arc::new(DefaultLlmContentGenerator::new()));
        assert!(agent.content_generator.is_some());
    }

    #[tokio::test]
    async fn test_research_agent_state_transitions() {
        let agent = ResearchAgent::new();

        let result = agent.start("Test topic".to_string()).await;
        assert!(result.is_ok());

        let state = agent.get_state().await;
        assert_eq!(state.status, ResearchStatus::InProgress);
        assert_eq!(state.topic, "Test topic");
    }

    #[tokio::test]
    async fn test_default_llm_generator() {
        let generator = DefaultLlmContentGenerator::new();

        let outline = generator.generate_outline("test", "context").await;
        let content = generator
            .generate_content("test", "outline", "sources")
            .await;
        let summary = generator.generate_summary("test", "findings").await;

        assert!(outline.is_ok(), "outline fallback should succeed");
        let _ = (content, summary);
    }

    #[tokio::test]
    async fn test_research_agent_default() {
        let agent = ResearchAgent::default();
        let state = agent.get_state().await;
        assert_eq!(state.status, ResearchStatus::Pending);
    }

    #[tokio::test]
    async fn test_research_agent_with_config() {
        let config = ResearchConfig {
            max_sources: 10,
            max_citations: 5,
            parallel_searches: 3,
            include_credibility_check: false,
            report_format: crate::research_state::ReportFormat::Html,
        };
        let agent = ResearchAgent::with_config(config);
        let state = agent.get_state().await;
        assert_eq!(state.config.max_sources, 10);
        assert_eq!(state.config.max_citations, 5);
    }

    #[tokio::test]
    async fn test_research_agent_with_planner() {
        let planner = SearchPlanner::new();
        let agent = ResearchAgent::new().with_planner(planner);
        let state = agent.get_state().await;
        assert_eq!(state.status, ResearchStatus::Pending);
    }

    #[tokio::test]
    async fn test_research_agent_with_orchestrator() {
        let orchestrator = SearchOrchestrator::new();
        let agent = ResearchAgent::new().with_orchestrator(orchestrator);
        let state = agent.get_state().await;
        assert_eq!(state.status, ResearchStatus::Pending);
    }

    #[tokio::test]
    async fn test_research_agent_start_sets_topic() {
        let agent = ResearchAgent::new();
        let id = agent.start("AI safety".to_string()).await.unwrap();
        assert!(!id.is_empty());
        let state = agent.get_state().await;
        assert_eq!(state.topic, "AI safety");
        assert_eq!(state.status, ResearchStatus::InProgress);
        assert_eq!(state.current_phase, ResearchPhase::Planning);
    }

    #[tokio::test]
    async fn test_research_agent_start_twice_errors() {
        let agent = ResearchAgent::new();
        agent.start("topic1".to_string()).await.unwrap();
        let result = agent.start("topic2".to_string()).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ResearchError::AlreadyCompleted));
    }

    #[tokio::test]
    async fn test_research_agent_execute_not_started() {
        let agent = ResearchAgent::new();
        let result = agent.execute_research().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ResearchError::NotStarted));
    }

    #[tokio::test]
    async fn test_research_agent_subscribe() {
        let agent = ResearchAgent::new();
        let mut receiver = agent.subscribe();
        agent.start("test topic".to_string()).await.unwrap();
        let event = receiver.try_recv().unwrap();
        match event {
            ResearchEvent::Started { topic } => {
                assert_eq!(topic, "test topic");
            },
            _ => panic!("Expected Started event"),
        }
    }

    #[tokio::test]
    async fn test_research_agent_get_progress() {
        let agent = ResearchAgent::new();
        let progress = agent.get_progress().await;
        assert_eq!(progress.phase, ResearchPhase::Planning);
    }

    #[tokio::test]
    async fn test_research_agent_update_phase_emits_event() {
        let agent = ResearchAgent::new();
        let mut receiver = agent.subscribe();
        agent.start("test".to_string()).await.unwrap();
        let _ = receiver.try_recv();
        agent.update_phase(ResearchPhase::Searching).await;
        let event = receiver.try_recv().unwrap();
        match event {
            ResearchEvent::PhaseChanged { from, to } => {
                assert_eq!(from, ResearchPhase::Planning);
                assert_eq!(to, ResearchPhase::Searching);
            },
            _ => panic!("Expected PhaseChanged event"),
        }
    }

    #[tokio::test]
    async fn test_research_agent_update_phase_same_no_event() {
        let agent = ResearchAgent::new();
        let mut receiver = agent.subscribe();
        agent.start("test".to_string()).await.unwrap();
        let _ = receiver.try_recv();
        agent.update_phase(ResearchPhase::Planning).await;
        assert!(receiver.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_research_agent_planning_phase() {
        let agent = ResearchAgent::new();
        agent.start("Rust programming".to_string()).await.unwrap();
        let plan = agent.planning_phase().await.unwrap();
        assert!(!plan.queries.is_empty());
    }

    #[tokio::test]
    async fn test_research_agent_build_research_context() {
        let agent = ResearchAgent::new();
        let mut state = ResearchState::new("Test topic".to_string());
        state.add_search_result(SearchResult::new(
            SourceType::Web,
            "https://example.com".to_string(),
            "Example Source".to_string(),
            "A test snippet".to_string(),
        ));
        let context = agent.build_research_context(&state);
        assert!(context.contains("Test topic"));
        assert!(context.contains("Example Source"));
        assert!(context.contains("https://example.com"));
    }

    #[tokio::test]
    async fn test_research_agent_build_research_context_empty() {
        let agent = ResearchAgent::new();
        let state = ResearchState::new("Empty topic".to_string());
        let context = agent.build_research_context(&state);
        assert!(context.contains("Empty topic"));
        assert!(context.contains("Number of sources: 0"));
    }

    #[tokio::test]
    async fn test_research_agent_format_sources_for_llm() {
        let agent = ResearchAgent::new();
        let mut state = ResearchState::new("Topic".to_string());
        state.add_search_result(SearchResult::new(
            SourceType::Web,
            "https://example.com".to_string(),
            "Source Title".to_string(),
            "Some content".to_string(),
        ));
        let formatted = agent.format_sources_for_llm(&state);
        assert!(formatted.contains("Source Title"));
        assert!(formatted.contains("https://example.com"));
        assert!(formatted.contains("Web"));
    }

    #[tokio::test]
    async fn test_research_agent_format_findings_for_llm() {
        let agent = ResearchAgent::new();
        let mut state = ResearchState::new("Topic".to_string());
        state.add_search_result(SearchResult::new(
            SourceType::Academic,
            "https://paper.com".to_string(),
            "Research Paper".to_string(),
            "Abstract".to_string(),
        ));
        let findings = agent.format_findings_for_llm(&state);
        assert!(findings.contains("Topic"));
        assert!(findings.contains("Research Paper"));
    }

    #[tokio::test]
    async fn test_research_agent_generate_content_no_generator() {
        let agent = ResearchAgent::new();
        let mut state = ResearchState::new("Test Topic".to_string());
        state.add_search_result(SearchResult::new(
            SourceType::Web,
            "https://example.com".to_string(),
            "Example".to_string(),
            "Snippet".to_string(),
        ));
        state.add_citation(Citation::new(
            "https://example.com".to_string(),
            "Example".to_string(),
            SourceType::Web,
        ));
        let content = agent.generate_content(&state).await.unwrap();
        assert!(content.contains("Test Topic"));
        assert!(content.contains("Example"));
    }

    #[tokio::test]
    async fn test_research_agent_generate_summary_no_generator() {
        let agent = ResearchAgent::new();
        let mut state = ResearchState::new("Summary Topic".to_string());
        state.add_search_result(SearchResult::new(
            SourceType::Web,
            "https://example.com".to_string(),
            "Example".to_string(),
            "Snippet".to_string(),
        ));
        state.add_citation(Citation::new(
            "https://example.com".to_string(),
            "Example".to_string(),
            SourceType::Web,
        ));
        let summary = agent.generate_summary(&state).await.unwrap();
        assert!(summary.contains("Summary Topic"));
        assert!(summary.contains("1"));
    }

    #[tokio::test]
    async fn test_research_agent_generate_outline_no_generator() {
        let agent = ResearchAgent::new();
        let state = ResearchState::new("Outline Topic".to_string());
        let outline = agent.generate_outline(&state).await.unwrap();
        assert!(!outline.title.is_empty());
        assert!(!outline.sections.is_empty());
        assert!(outline.sections.len() >= 6);
    }

    #[tokio::test]
    async fn test_research_error_display() {
        let err = ResearchError::NotStarted;
        assert!(err.to_string().contains("not started"));

        let err = ResearchError::AlreadyCompleted;
        assert!(err.to_string().contains("already completed"));

        let err = ResearchError::Failed("test error".to_string());
        assert!(err.to_string().contains("test error"));

        let err = ResearchError::PlanningFailed("plan err".to_string());
        assert!(err.to_string().contains("plan err"));

        let err = ResearchError::SearchFailed("search err".to_string());
        assert!(err.to_string().contains("search err"));

        let err = ResearchError::ReportGenerationFailed("report err".to_string());
        assert!(err.to_string().contains("report err"));

        let err = ResearchError::LlmFailed("llm err".to_string());
        assert!(err.to_string().contains("llm err"));
    }

    #[tokio::test]
    async fn test_research_error_invalid_state_transition() {
        let err = ResearchError::InvalidStateTransition {
            from: ResearchStatus::Pending,
            to: ResearchStatus::Completed,
        };
        assert!(err.to_string().contains("Invalid state transition"));
    }

    #[tokio::test]
    async fn test_research_event_serialization() {
        let event = ResearchEvent::Started {
            topic: "test".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("test"));

        let event = ResearchEvent::PhaseChanged {
            from: ResearchPhase::Planning,
            to: ResearchPhase::Searching,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("PhaseChanged"));

        let event = ResearchEvent::SourcesFound { count: 5 };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("5"));

        let event = ResearchEvent::CitationAdded {
            citation_id: "cit-1".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("cit-1"));

        let event = ResearchEvent::ReportGenerated {
            report_id: "rep-1".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("rep-1"));

        let event = ResearchEvent::Completed;
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Completed"));

        let event = ResearchEvent::Failed {
            error: "something went wrong".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("something went wrong"));

        let event = ResearchEvent::Paused;
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Paused"));

        let event = ResearchEvent::Resumed;
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Resumed"));
    }

    #[tokio::test]
    async fn test_default_llm_content_generator_default() {
        let generator = DefaultLlmContentGenerator::default();
        let result = generator.generate_outline("topic", "ctx").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_default_llm_content_generator_no_adapter() {
        let generator = DefaultLlmContentGenerator::new();
        let content = generator.generate_content("t", "o", "s").await;
        assert!(content.is_err());
        let summary = generator.generate_summary("t", "f").await;
        assert!(summary.is_err());
    }

    #[tokio::test]
    async fn test_research_agent_extraction_phase_dedup() {
        let agent = ResearchAgent::new();
        agent.start("test".to_string()).await.unwrap();
        {
            let mut state = agent.state.write().await;
            state.add_search_result(
                SearchResult::new(
                    SourceType::Web,
                    "https://example.com/page1".to_string(),
                    "Page 1".to_string(),
                    "Content 1".to_string(),
                )
                .with_relevance(0.9)
                .with_credibility(0.8),
            );
            state.add_search_result(
                SearchResult::new(
                    SourceType::Web,
                    "https://EXAMPLE.COM/page1".to_string(),
                    "Page 1 Dup".to_string(),
                    "Content 1 Dup".to_string(),
                )
                .with_relevance(0.8)
                .with_credibility(0.7),
            );
        }
        agent.extraction_phase().await.unwrap();
        let state = agent.get_state().await;
        assert_eq!(state.citations.len(), 1);
    }

    #[tokio::test]
    async fn test_research_agent_analysis_phase() {
        let agent = ResearchAgent::new();
        agent.start("test".to_string()).await.unwrap();
        {
            let mut state = agent.state.write().await;
            state.add_citation(Citation::new(
                "https://a.com".to_string(),
                "A".to_string(),
                SourceType::Web,
            ));
            state.add_citation(Citation::new(
                "https://b.com".to_string(),
                "B".to_string(),
                SourceType::Academic,
            ));
        }
        agent.analysis_phase().await.unwrap();
        let state = agent.get_state().await;
        assert_eq!(state.progress.sources_processed, 2);
    }

    #[tokio::test]
    async fn test_research_agent_synthesis_phase() {
        let agent = ResearchAgent::new();
        agent.start("test".to_string()).await.unwrap();
        agent.synthesis_phase().await.unwrap();
    }

    #[tokio::test]
    async fn test_research_agent_generate_report() {
        let agent = ResearchAgent::new();
        let mut state = ResearchState::new("Report Topic".to_string());
        state.add_citation(Citation::new(
            "https://example.com".to_string(),
            "Example".to_string(),
            SourceType::Web,
        ));
        let report = agent.generate_report(&state).await.unwrap();
        assert_eq!(report.topic, "Report Topic");
        assert!(!report.content.is_empty());
        assert!(!report.summary.is_empty());
        assert_eq!(report.citations.len(), 1);
    }

    #[tokio::test]
    async fn test_research_agent_format_sources_for_llm_many() {
        let agent = ResearchAgent::new();
        let mut state = ResearchState::new("Topic".to_string());
        for i in 0..25 {
            state.add_search_result(SearchResult::new(
                SourceType::Web,
                format!("https://example.com/{}", i),
                format!("Source {}", i),
                format!("Snippet {}", i),
            ));
        }
        let formatted = agent.format_sources_for_llm(&state);
        assert!(formatted.contains("Source 0"));
        assert!(formatted.contains("Source 19"));
    }

    #[tokio::test]
    async fn test_research_agent_build_research_context_many_sources() {
        let agent = ResearchAgent::new();
        let mut state = ResearchState::new("Topic".to_string());
        for i in 0..15 {
            state.add_search_result(SearchResult::new(
                SourceType::Web,
                format!("https://example.com/{}", i),
                format!("Source {}", i),
                format!("Snippet {}", i),
            ));
        }
        let context = agent.build_research_context(&state);
        assert!(context.contains("Source 0"));
        assert!(context.contains("Source 9"));
    }
}
