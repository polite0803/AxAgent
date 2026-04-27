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
}

pub struct ResearchAgent {
    planner: SearchPlanner,
    orchestrator: SearchOrchestrator,
    state: Arc<RwLock<ResearchState>>,
    event_sender: broadcast::Sender<ResearchEvent>,
}

impl ResearchAgent {
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(100);
        Self {
            planner: SearchPlanner::new(),
            orchestrator: SearchOrchestrator::new(),
            state: Arc::new(RwLock::new(ResearchState::new(String::new()))),
            event_sender,
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
        }
    }

    pub fn with_planner(mut self, planner: SearchPlanner) -> Self {
        self.planner = planner;
        self
    }

    pub fn with_orchestrator(mut self, orchestrator: SearchOrchestrator) -> Self {
        self.orchestrator = orchestrator;
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

        let mut state = self.state.write().await;
        state.complete();
        drop(state);

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

        self.emit(ResearchEvent::SourcesFound {
            count: results.len(),
        });
        tracing::info!("Searching phase complete, found {} sources", results.len());

        Ok(results)
    }

    async fn extraction_phase(&self) -> Result<(), ResearchError> {
        self.update_phase(ResearchPhase::Extracting).await;

        let results = self.state.read().await.search_results.clone();
        let mut citations_added = 0;

        for result in results
            .iter()
            .take(self.state.read().await.config.max_citations)
        {
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
            "Extraction phase complete, added {} citations",
            citations_added
        );

        Ok(())
    }

    async fn analysis_phase(&self) -> Result<(), ResearchError> {
        self.update_phase(ResearchPhase::Analyzing).await;

        let citations = self.state.read().await.citations.clone();

        for (idx, _citation) in citations.iter().enumerate() {
            tracing::debug!("Analyzing citation {} of {}", idx + 1, citations.len());
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

    async fn generate_outline(
        &self,
        state: &ResearchState,
    ) -> Result<crate::research_state::ReportOutline, ResearchError> {
        use crate::research_state::{OutlineSection, ReportOutline};

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
        Ok(format!(
            "本研究通过搜索和分析 {} 个来源，对「{}」进行了系统性研究。\
            主要发现了 {} 条相关信息，并生成了包含 {} 个引用的研究报告。",
            state.search_results.len(),
            state.topic,
            state.search_results.len(),
            state.citations.len()
        ))
    }

    async fn update_phase(&self, new_phase: ResearchPhase) {
        let old_phase = {
            let mut state = self.state.write().await;
            let old = state.current_phase;
            state.set_phase(new_phase);
            old
        };

        self.emit(ResearchEvent::PhaseChanged {
            from: old_phase,
            to: new_phase,
        });
    }

    pub async fn pause(&self) -> Result<(), ResearchError> {
        let mut state = self.state.write().await;

        if state.status != ResearchStatus::InProgress {
            return Err(ResearchError::InvalidStateTransition {
                from: state.status,
                to: ResearchStatus::Paused,
            });
        }

        state.pause();
        self.emit(ResearchEvent::Paused);

        tracing::info!("Research paused");

        Ok(())
    }

    pub async fn resume(&self) -> Result<(), ResearchError> {
        {
            let mut state = self.state.write().await;

            if state.status != ResearchStatus::Paused {
                return Err(ResearchError::InvalidStateTransition {
                    from: state.status,
                    to: ResearchStatus::InProgress,
                });
            }

            state.resume();
        }

        self.emit(ResearchEvent::Resumed);
        tracing::info!("Research resumed");

        Ok(())
    }

    pub async fn stop(&self) -> Result<(), ResearchError> {
        let mut state = self.state.write().await;

        if state.status.is_terminal() {
            return Err(ResearchError::AlreadyCompleted);
        }

        state.status = ResearchStatus::Failed;
        state.completed_at = Some(chrono::Utc::now());

        self.emit(ResearchEvent::Failed {
            error: "Research stopped by user".to_string(),
        });

        tracing::info!("Research stopped");

        Ok(())
    }

    pub async fn add_citation(&self, citation: Citation) -> Result<(), ResearchError> {
        let mut state = self.state.write().await;

        if state.citations.len() >= state.config.max_citations {
            return Err(ResearchError::Failed(
                "Maximum citations reached".to_string(),
            ));
        }

        state.add_citation(citation.clone());
        self.emit(ResearchEvent::CitationAdded {
            citation_id: citation.id,
        });

        Ok(())
    }

    pub async fn remove_citation(&self, citation_id: &str) -> Result<(), ResearchError> {
        let mut state = self.state.write().await;
        state.citations.retain(|c| c.id != citation_id);
        Ok(())
    }

    pub async fn set_citation_in_report(
        &self,
        citation_id: &str,
        in_report: bool,
    ) -> Result<(), ResearchError> {
        let mut state = self.state.write().await;

        if let Some(citation) = state.citations.iter_mut().find(|c| c.id == citation_id) {
            citation.in_report = in_report;
        }

        Ok(())
    }
}

impl Default for ResearchAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_research() {
        let agent = ResearchAgent::new();
        let result = agent.start("Rust programming".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_pause_resume() {
        let agent = ResearchAgent::new();
        agent.start("Test topic".to_string()).await.unwrap();
        agent.pause().await.unwrap();
        let state = agent.get_state().await;
        assert_eq!(state.status, ResearchStatus::Paused);
        agent.resume().await.unwrap();
        let state = agent.get_state().await;
        assert_eq!(state.status, ResearchStatus::InProgress);
    }
}
