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
use tokio::sync::{broadcast, RwLock};

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
    Started { topic: String },
    PhaseChanged { from: ResearchPhase, to: ResearchPhase },
    SourcesFound { count: usize },
    SourceProcessed { source_id: String },
    CitationAdded { citation_id: String },
    ReportGenerated { report_id: String },
    Completed,
    Failed { error: String },
    Paused,
    Resumed,
    LlmGenerationStarted { phase: String },
    LlmGenerationCompleted { phase: String },
}

pub trait LlmContentGenerator: Send + Sync {
    fn generate_outline(&self, topic: &str, context: &str) -> impl std::future::Future<Output = Result<String, ResearchError>> + Send;
    fn generate_content(&self, topic: &str, outline: &str, sources: &str) -> impl std::future::Future<Output = Result<String, ResearchError>> + Send;
    fn generate_summary(&self, topic: &str, findings: &str) -> impl std::future::Future<Output = Result<String, ResearchError>> + Send;
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
            state: Arc::new(RwLock::new(
                ResearchState::new(String::new()).with_config(config),
            )),
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

        self.emit(ResearchEvent::Started { topic: topic.clone() });
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

        tracing::info!(
            "Planning phase complete, generated {} queries",
            plan.queries.len()
        );

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

        self.emit(ResearchEvent::SourcesFound { count: results.len() });
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
                + a.credibility_score.unwrap_or(a.source_type.default_credibility());
            let score_b = b.relevance_score
                + b.credibility_score.unwrap_or(b.source_type.default_credibility());
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
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

        tracing::info!(
            "Analysis phase complete, processed {} sources",
            citations.len()
        );

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

    async fn generate_outline(&self, state: &ResearchState) -> Result<crate::research_state::ReportOutline, ResearchError> {
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
                let mut report_outline = ReportOutline::new()
                    .with_title(format!("关于「{}」的研究报告", state.topic));
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
            OutlineSection::new("结论".to_string())
                .with_description("研究结论和建议".to_string()),
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

            let content = generator.generate_content(&state.topic, &outline, &sources).await?;

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
    llm_adapter: Option<Arc<dyn axagent_providers::ProviderAdapter>>,
    ctx: Option<axagent_providers::ProviderRequestContext>,
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
        adapter: Arc<dyn axagent_providers::ProviderAdapter>,
        ctx: axagent_providers::ProviderRequestContext,
    ) -> Self {
        self.llm_adapter = Some(adapter);
        self.ctx = Some(ctx);
        self
    }

    async fn call_llm(&self, system: &str, user: &str) -> Result<String, ResearchError> {
        use axagent_core::types::{ChatRequest, ChatMessage, ChatContent};

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
                        },
                        ChatMessage {
                            role: "user".to_string(),
                            content: ChatContent::Text(user.to_string()),
                            tool_calls: None,
                            tool_call_id: None,
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

                let response = adapter.chat(ctx, request).await
                    .map_err(|e| ResearchError::LlmFailed(e.to_string()))?;

                Ok(response.content)
            }
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

    async fn generate_content(&self, topic: &str, outline: &str, sources: &str) -> Result<String, ResearchError> {
        let system = r#"你是一个专业的研究报告撰写专家。根据提供的大纲和来源信息，生成完整的研究报告内容。

要求：
1. 内容应详尽、深入，覆盖大纲的所有要点
2. 适当引用来源信息，使用[来源描述]格式标注引用
3. 保持学术写作风格，逻辑清晰
4. 输出完整的Markdown格式报告"#;

        let user = format!(
            "研究主题: {}\n\n大纲:\n{}\n\n来源信息:\n{}",
            topic, outline, sources
        );

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

    #[tokio::test]
    async fn test_research_agent_creation() {
        let agent = ResearchAgent::new();
        assert!(agent.content_generator.is_none());
    }

    #[tokio::test]
    async fn test_research_agent_with_generator() {
        let agent = ResearchAgent::new()
            .with_generator(Arc::new(DefaultLlmContentGenerator::new()));
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
        assert!(outline.is_ok());

        let content = generator.generate_content("test", "outline", "sources").await;
        assert!(content.is_ok());

        let summary = generator.generate_summary("test", "findings").await;
        assert!(summary.is_ok());
    }
}
